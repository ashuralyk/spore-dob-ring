use alloc::{borrow::ToOwned, string::String, vec::Vec};
use ckb_gen_types::packed::OutPoint;
use serde::{ser::SerializeMap, Serialize};
use serde_json::Value;

#[repr(u64)]
#[cfg_attr(test, derive(Debug))]
pub enum Error {
    ParseInvalidArgCount = 1,
    ParseInvalidSporeDNA,
    ParseInvalidRingDNA,
    ParseInvalidTraitsBase,

    SchemaInsufficientElements,
    SchemaInvalidName,
    SchemaInvalidType,
    SchemaInvalidClusterId,
    SchemaInvalidOffset,
    SchemaInvalidLen,
    SchemaInvalidPattern,
    SchemaPatternMismatch,
    SchemaInvalidArgs,
    SchemaInvalidArgsElement,

    DecodeUnexpectedDNASegment,
    DecodeArgsTypeMismatch,
    DecodeMissingRangeArgs,
    DecodeInvalidRangeArgs,
    DecodeMissingOptionArgs,
    DecodeInvalidOptionArgs,
    DecodeBadUTF8Format,

    SyscallEmptyDNASet,
    SyscallBadUTF8Stream,
}

pub struct ParsedTrait {
    pub type_: String,
    pub value: Value,
}

impl Serialize for ParsedTrait {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.type_, &self.value)?;
        map.end()
    }
}

#[derive(serde::Serialize, Default)]
pub struct ParsedDNA {
    pub name: String,
    pub traits: Vec<ParsedTrait>,
}

pub struct Parameters {
    pub ring_tail_outpoint: OutPoint,
    pub spore_dna: Option<Vec<u8>>,
    pub traits_base: Vec<TraitSchema>,
}

#[cfg_attr(test, derive(serde::Serialize, Clone))]
#[derive(serde::Deserialize)]
pub enum Pattern {
    Options,
    Range,
    RawNumber,
    RawString,
    Utf8,
}

#[cfg_attr(test, derive(serde::Serialize, Clone))]
#[derive(serde::Deserialize)]
pub struct TraitSchema {
    pub name: String,
    pub type_: String,
    pub cluster_id: [u8; 32],
    pub offset: u64,
    pub len: u64,
    pub pattern: Pattern,
    pub args: Option<Vec<Value>>,
}

#[cfg(test)]
impl TraitSchema {
    #[allow(dead_code)]
    pub fn new(
        name: String,
        type_: String,
        cluster_id: [u8; 32],
        offset: u64,
        len: u64,
        pattern: Pattern,
        args: Option<Vec<Value>>,
    ) -> Self {
        Self {
            name,
            type_,
            cluster_id,
            offset,
            len,
            pattern,
            args,
        }
    }

    #[allow(dead_code)]
    pub fn encode(self) -> Vec<Value> {
        let mut values = vec![
            Value::String(self.name),
            Value::String(self.type_),
            Value::String(hex::encode(self.cluster_id)),
            Value::Number(self.offset.into()),
            Value::Number(self.len.into()),
            Value::String(match self.pattern {
                Pattern::Options => "options".to_owned(),
                Pattern::Range => "range".to_owned(),
                Pattern::RawNumber => "rawNumber".to_owned(),
                Pattern::RawString => "rawString".to_owned(),
                Pattern::Utf8 => "utf8".to_owned(),
            }),
        ];
        if let Some(args) = &self.args {
            values.push(Value::Array(args.to_owned()));
        }
        values
    }
}

pub fn decode_trait_schema(traits_pool: Value) -> Result<Vec<TraitSchema>, Error> {
    let traits_base = traits_pool
        .as_array()
        .ok_or(Error::ParseInvalidTraitsBase)?
        .iter()
        .map(|schema| {
            let schema = schema.as_array().ok_or(Error::ParseInvalidTraitsBase)?;
            if schema.len() < 6 {
                return Err(Error::SchemaInsufficientElements);
            }
            let name = schema[0].as_str().ok_or(Error::SchemaInvalidName)?;
            let type_ = schema[1].as_str().ok_or(Error::SchemaInvalidType)?;
            let cluster_id_hexed_str = schema[2].as_str().ok_or(Error::SchemaInvalidClusterId)?;
            let cluster_id: [u8; 32] = hex::decode(cluster_id_hexed_str.trim_start_matches("0x"))
                .map_err(|_| Error::SchemaInvalidClusterId)?
                .try_into()
                .map_err(|_| Error::SchemaInvalidClusterId)?;
            let offset = schema[3].as_u64().ok_or(Error::SchemaInvalidOffset)?;
            let len = schema[4].as_u64().ok_or(Error::SchemaInvalidLen)?;
            let pattern_str = schema[5].as_str().ok_or(Error::SchemaInvalidPattern)?;
            let pattern = match pattern_str {
                "options" => Pattern::Options,
                "rawNumber" => Pattern::RawNumber,
                "rawString" => Pattern::RawString,
                "utf8" => Pattern::Utf8,
                "range" => Pattern::Range,
                _ => return Err(Error::SchemaPatternMismatch),
            };
            let args = if let Some(args) = schema.get(6) {
                let args = args.as_array().ok_or(Error::SchemaInvalidArgs)?.to_owned();
                Some(args)
            } else {
                None
            };
            Ok(TraitSchema {
                name: name.to_owned(),
                type_: type_.to_owned(),
                cluster_id,
                offset,
                len,
                pattern,
                args,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(traits_base)
}
