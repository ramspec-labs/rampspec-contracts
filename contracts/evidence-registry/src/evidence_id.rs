use rampspec_shared_types::{EVIDENCE_ID_DOMAIN, NetworkKind};
use soroban_sdk::{Address, Bytes, BytesN, Env, xdr::ToXdr};

pub(crate) fn derive(
    env: &Env,
    publisher: &Address,
    report_hash: &BytesN<32>,
    network: &NetworkKind,
) -> BytesN<32> {
    let mut preimage = Bytes::new(env);
    preimage.extend_from_slice(EVIDENCE_ID_DOMAIN);
    preimage.append(&publisher.clone().to_xdr(env));
    preimage.extend_from_slice(&report_hash.to_array());
    preimage.extend_from_slice(&network.discriminant().to_be_bytes());
    BytesN::from_array(env, &env.crypto().sha256(&preimage).to_array())
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;
    use alloc::{string::String, vec::Vec};
    use serde::Deserialize;
    use soroban_sdk::testutils::Address as _;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct VectorFile {
        schema_version: u32,
        hash_algorithm: String,
        domain_hex: String,
        network_encoding: String,
        publisher_address_encoding: String,
        vectors: Vec<Vector>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Vector {
        name: String,
        publisher_address: String,
        publisher_address_xdr_hex: String,
        report_hash_hex: String,
        network: String,
        network_discriminant: u32,
        preimage_hex: String,
        evidence_id_hex: String,
    }

    fn hex_bytes(hex: &str) -> Option<Vec<u8>> {
        if !hex.len().is_multiple_of(2) {
            return None;
        }
        let mut bytes = Vec::with_capacity(hex.len() / 2);
        let mut chars = hex.as_bytes().chunks_exact(2);
        for pair in chars.by_ref() {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            bytes.push(u8::try_from((high << 4) | low).ok()?);
        }
        Some(bytes)
    }

    fn publisher(env: &Env) -> Address {
        Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        )
    }

    #[test]
    fn published_evidence_id_vectors_are_stable() {
        let env = Env::default();
        let fixture: VectorFile =
            serde_json::from_str(include_str!("../../../fixtures/evidence-id-vectors.json"))
                .unwrap();
        assert_eq!(fixture.schema_version, 1);
        assert_eq!(fixture.hash_algorithm, "sha256");
        assert_eq!(
            fixture.domain_hex,
            "72616d70737065632d65766964656e63652d7631"
        );
        assert_eq!(fixture.network_encoding, "u32-big-endian");
        assert_eq!(
            fixture.publisher_address_encoding,
            "stellar-soroban-scval-xdr"
        );

        for vector in fixture.vectors {
            let report_hash = BytesN::from_array(
                &env,
                &hex_bytes(&vector.report_hash_hex)
                    .unwrap()
                    .try_into()
                    .unwrap(),
            );
            let network = match vector.network.as_str() {
                "Testnet" => NetworkKind::Testnet,
                "Futurenet" => NetworkKind::Futurenet,
                "Pubnet" => NetworkKind::Pubnet,
                "Standalone" => NetworkKind::Standalone,
                _ => panic!("unknown network in compatibility vector"),
            };
            assert_eq!(network.discriminant(), vector.network_discriminant);

            let publisher_address = Address::from_str(&env, &vector.publisher_address);
            let canonical = derive(&env, &publisher_address, &report_hash, &network);
            let expected_id: [u8; 32] = hex_bytes(&vector.evidence_id_hex)
                .unwrap()
                .try_into()
                .unwrap();
            assert_eq!(canonical.to_array(), expected_id, "{}", vector.name,);

            let address_xdr = hex_bytes(&vector.publisher_address_xdr_hex).unwrap();
            let canonical_address_xdr = publisher_address.to_xdr(&env);
            assert_eq!(
                canonical_address_xdr.len(),
                u32::try_from(address_xdr.len()).unwrap()
            );
            for (index, byte) in address_xdr.iter().enumerate() {
                assert_eq!(
                    canonical_address_xdr.get(u32::try_from(index).unwrap()),
                    Some(*byte),
                    "{} publisher address XDR mismatch",
                    vector.name,
                );
            }

            let mut preimage = Bytes::new(&env);
            preimage.extend_from_slice(EVIDENCE_ID_DOMAIN);
            preimage.extend_from_slice(&address_xdr);
            preimage.extend_from_slice(&hex_bytes(&vector.report_hash_hex).unwrap());
            preimage.extend_from_slice(&network.discriminant().to_be_bytes());
            let preimage_fixture = hex_bytes(&vector.preimage_hex).unwrap();
            assert_eq!(
                preimage.len(),
                u32::try_from(preimage_fixture.len()).unwrap()
            );
            for (index, byte) in preimage_fixture.iter().enumerate() {
                assert_eq!(
                    preimage.get(u32::try_from(index).unwrap()),
                    Some(*byte),
                    "{} preimage mismatch",
                    vector.name,
                );
            }
        }
    }

    #[test]
    fn derivation_is_deterministic() {
        let env = Env::default();
        let report_hash = BytesN::from_array(&env, &[7; 32]);
        let first = derive(&env, &publisher(&env), &report_hash, &NetworkKind::Testnet);
        let second = derive(&env, &publisher(&env), &report_hash, &NetworkKind::Testnet);
        assert_eq!(
            first.to_array(),
            [
                226, 162, 106, 207, 218, 115, 68, 176, 28, 109, 100, 113, 216, 105, 164, 135, 26,
                112, 228, 132, 222, 241, 63, 209, 160, 6, 80, 87, 103, 20, 180, 113,
            ]
        );
        assert_eq!(first, second);
    }

    #[test]
    fn every_identity_component_changes_the_result() {
        let env = Env::default();
        let report_hash = BytesN::from_array(&env, &[7; 32]);
        let baseline = derive(&env, &publisher(&env), &report_hash, &NetworkKind::Testnet);
        let other_publisher = Address::generate(&env);
        assert_ne!(
            baseline,
            derive(&env, &other_publisher, &report_hash, &NetworkKind::Testnet)
        );
        assert_ne!(
            baseline,
            derive(
                &env,
                &publisher(&env),
                &BytesN::from_array(&env, &[8; 32]),
                &NetworkKind::Testnet,
            )
        );
        assert_ne!(
            baseline,
            derive(&env, &publisher(&env), &report_hash, &NetworkKind::Pubnet)
        );
    }

    #[test]
    fn distinct_domains_do_not_collide() {
        let env = Env::default();
        let publisher = publisher(&env);
        let report_hash = BytesN::from_array(&env, &[7; 32]);
        let network = NetworkKind::Testnet;

        let mut canonical = Bytes::new(&env);
        canonical.extend_from_slice(EVIDENCE_ID_DOMAIN);
        canonical.append(&publisher.clone().to_xdr(&env));
        canonical.extend_from_slice(&report_hash.to_array());
        canonical.extend_from_slice(&network.discriminant().to_be_bytes());
        let canonical_id = BytesN::from_array(&env, &env.crypto().sha256(&canonical).to_array());

        let mut different_domain = Bytes::new(&env);
        different_domain.extend_from_slice(b"rampspec-evidence-v2");
        different_domain.append(&publisher.to_xdr(&env));
        different_domain.extend_from_slice(&report_hash.to_array());
        different_domain.extend_from_slice(&network.discriminant().to_be_bytes());
        let different_domain_id =
            BytesN::from_array(&env, &env.crypto().sha256(&different_domain).to_array());

        assert_ne!(canonical_id, different_domain_id);
    }
}
