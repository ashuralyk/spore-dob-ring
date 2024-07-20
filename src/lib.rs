#![cfg_attr(not(test), no_std)]

extern crate alloc;
pub mod decoder;

#[cfg(test)]
mod test {
    use ckb_gen_types::{packed::OutPoint, prelude::Entity};
    use ckb_hash::blake2b_256;
    use lazy_static::lazy_static;
    use serde_json::json;

    use crate::decoder::{dobs_decode, dobs_parse_parameters, types::Error};

    lazy_static! {
        static ref CLUSTER_ID_1: String = hex::encode([0u8; 32]);
        static ref CLUSTER_ID_2: String = hex::encode([1u8; 32]);
        static ref CLUSTER_ID_3: String = hex::encode([2u8; 32]);
        static ref CLUSTER_ID_4: String = hex::encode([3u8; 32]);
        static ref CLUSTER_ID_5: String = hex::encode([4u8; 32]);
        static ref CLUSTER_ID_6: String = hex::encode([5u8; 32]);
    }

    fn get_cluster_dnas(_outpoint: &OutPoint, cluster_id: &[u8; 32]) -> Result<Vec<String>, Error> {
        let fixed_fragment = "687474703a2f2f3132372e302e302e313a383039300000";
        Ok(vec![
            hex::encode(&blake2b_256(cluster_id)[..6]) + fixed_fragment,
        ])
    }

    #[test]
    fn test_generate_basic_example() {
        let traits_base = json!([
            [
                "Name",
                "String",
                *CLUSTER_ID_1,
                0,
                1,
                "options",
                ["Alice", "Bob", "Charlie", "David", "Ethan", "Florence", "Grace", "Helen",],
            ],
            ["Age", "Number", *CLUSTER_ID_2, 1, 1, "range", [0, 100]],
            ["Score", "Number", *CLUSTER_ID_3, 2, 1, "rawNumber"],
            ["DNA", "String", *CLUSTER_ID_4, 3, 3, "rawString"],
            ["URL", "String", *CLUSTER_ID_5, 6, 30, "utf8"],
            ["Value", "Timestamp", *CLUSTER_ID_6, 3, 3, "rawNumber"],
        ])
        .to_string();
        println!("traits_base = {traits_base}");

        let dna = "ac7b88aabbcc687474703a2f2f3132372e302e302e313a383039300000";
        let ring_dna = hex::encode(OutPoint::default().as_slice()) + dna;
        let parameters = dobs_parse_parameters(vec![ring_dna.as_bytes(), traits_base.as_bytes()])
            .expect("parse parameters");

        let dna_traits = dobs_decode(parameters, get_cluster_dnas)
            .map_err(|error| format!("error code = {}", error as u64))
            .unwrap();

        println!("dna_traits = {}\n", String::from_utf8_lossy(&dna_traits));
    }
}
