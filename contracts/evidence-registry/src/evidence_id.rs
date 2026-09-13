#![allow(dead_code)]

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
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn publisher(env: &Env) -> Address {
        Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        )
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
}
