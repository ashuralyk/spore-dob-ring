use alloc::{string::String, vec::Vec};
use ckb_gen_types::{packed::OutPoint, prelude::Entity};
use core::cmp::{self, Ordering};

pub mod types;
use serde_json::Value;
use types::{Error, Parameters, ParsedDNA, ParsedTrait, Pattern, TraitSchema};

use self::types::decode_trait_schema;

// example:
// argv[0] = efc2866a311da5b6dfcdfc4e3c22d00d024a53217ebc33855eeab1068990ed9d (hexed DNA string in Spore)
// argv[1] = d48869363ff41a103b131a29f43...d7be6eeaf513c2c3ae056b9b8c2e1 (hexed pattern string in Cluster)
pub fn dobs_parse_parameters(args: Vec<&[u8]>) -> Result<Parameters, Error> {
    if args.len() < 2 {
        return Err(Error::ParseInvalidArgCount);
    }

    let outpoint_extended_dna = {
        let value = args[0];
        if value.is_empty() || value.len() % 2 != 0 {
            return Err(Error::ParseInvalidSporeDNA);
        }
        hex::decode(value).map_err(|_| Error::ParseInvalidSporeDNA)?
    };
    let traits_base = {
        let value = args[1];
        let traits_pool: Value =
            serde_json::from_slice(value).map_err(|_| Error::ParseInvalidTraitsBase)?;
        decode_trait_schema(traits_pool)?
    };

    // check outpoint validity
    let spore_dna = match outpoint_extended_dna.len().cmp(&OutPoint::TOTAL_SIZE) {
        Ordering::Less => return Err(Error::ParseInvalidSporeDNA),
        Ordering::Equal => None,
        Ordering::Greater => Some(outpoint_extended_dna[OutPoint::TOTAL_SIZE..].to_vec()),
    };
    let ring_tail_outpoint =
        OutPoint::from_compatible_slice(&outpoint_extended_dna[..OutPoint::TOTAL_SIZE])
            .map_err(|_| Error::ParseInvalidSporeDNA)?;

    Ok(Parameters {
        ring_tail_outpoint,
        spore_dna,
        traits_base,
    })
}

pub fn dobs_decode<F>(parameters: Parameters, get_cluster_dnas: F) -> Result<Vec<u8>, Error>
where
    F: Fn(&OutPoint, &[u8; 32]) -> Result<Vec<String>, Error>,
{
    let Parameters {
        ring_tail_outpoint,
        spore_dna,
        traits_base,
    } = parameters;

    let mut result = Vec::new();
    for schema_base in traits_base.into_iter() {
        let mut parsed_dna = ParsedDNA {
            name: schema_base.name.clone(),
            ..Default::default()
        };
        let dnas = get_cluster_dnas(&ring_tail_outpoint, &schema_base.cluster_id)?;
        dnas.into_iter().try_for_each(|dna| {
            let dna = hex::decode(dna).map_err(|_| Error::ParseInvalidRingDNA)?;
            parsed_dna.traits.push(decode_dna(&dna, &schema_base)?);
            Ok(())
        })?;
        if let Some(spore_dna) = &spore_dna {
            parsed_dna.traits.push(decode_dna(spore_dna, &schema_base)?);
        }
        result.push(parsed_dna);
    }

    Ok(serde_json::to_string(&result).unwrap().into_bytes())
}

fn decode_dna(dna: &[u8], schema_base: &TraitSchema) -> Result<ParsedTrait, Error> {
    let byte_offset = cmp::min(schema_base.offset as usize, dna.len());
    let byte_end = cmp::min(byte_offset + schema_base.len as usize, dna.len());
    let mut dna_segment = dna[byte_offset..byte_end].to_vec();
    let value: Value = match schema_base.pattern {
        Pattern::RawNumber => Value::Number(parse_u64(dna_segment)?.into()),
        Pattern::RawString => Value::String(hex::encode(&dna_segment)),
        Pattern::Utf8 => {
            while dna_segment.last() == Some(&0) {
                dna_segment.pop();
            }
            Value::String(String::from_utf8(dna_segment).map_err(|_| Error::DecodeBadUTF8Format)?)
        }
        Pattern::Range => {
            let args = schema_base
                .args
                .clone()
                .ok_or(Error::DecodeMissingRangeArgs)?;
            if args.len() != 2 {
                return Err(Error::DecodeInvalidRangeArgs);
            }
            let lower = args[0].as_u64().ok_or(Error::DecodeInvalidRangeArgs)?;
            let upper = args[1].as_u64().ok_or(Error::DecodeInvalidRangeArgs)?;
            if upper <= lower {
                return Err(Error::DecodeInvalidRangeArgs);
            }
            let offset = parse_u64(dna_segment)?;
            let offset = offset % (upper - lower);
            Value::Number((lower + offset).into())
        }
        Pattern::Options => {
            let args = schema_base
                .args
                .clone()
                .ok_or(Error::DecodeMissingOptionArgs)?;
            if args.is_empty() {
                return Err(Error::DecodeInvalidOptionArgs);
            }
            let offset = parse_u64(dna_segment)?;
            let offset = offset as usize % args.len();
            args[offset].clone()
        }
    };
    Ok(ParsedTrait {
        type_: schema_base.type_.clone(),
        value,
    })
}

fn parse_u64(dna_segment: Vec<u8>) -> Result<u64, Error> {
    let offset = match dna_segment.len() {
        1 => dna_segment[0] as u64,
        2 => u16::from_le_bytes(dna_segment.clone().try_into().unwrap()) as u64,
        3 | 4 => {
            let mut buf = [0u8; 4];
            buf[..dna_segment.len()].copy_from_slice(&dna_segment);
            u32::from_le_bytes(buf) as u64
        }
        5..=8 => {
            let mut buf = [0u8; 8];
            buf[..dna_segment.len()].copy_from_slice(&dna_segment);
            u64::from_le_bytes(buf)
        }
        _ => return Err(Error::DecodeUnexpectedDNASegment),
    };
    Ok(offset)
}
