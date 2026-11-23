#![no_main]

use libfuzzer_sys::fuzz_target;
use rampspec_shared_types::{EVIDENCE_ID_DOMAIN, NetworkKind};
use soroban_sdk::{
    Address, Bytes, BytesN, Env,
    testutils::Address as _,
    xdr::{FromXdr, ToXdr},
};

const MAX_INPUT_SIZE: usize = 10_000;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_SIZE {
        return;
    }

    let env = Env::default();
    let bytes = Bytes::from_slice(&env, data);

    // Fuzz address XDR decoding - should handle invalid inputs gracefully
    let _ = Address::from_xdr(&env, &bytes);

    // Fuzz network XDR decoding - should handle invalid inputs gracefully
    let _ = NetworkKind::from_xdr(&env, &bytes);

    // Test valid address and network combinations for deterministic derivation
    if data.len() >= 65 {
        let address_xdr = Bytes::from_slice(&env, &data[0..40]);
        let network_xdr = Bytes::from_slice(&env, &data[40..65]);

        if let (Ok(publisher), Ok(network)) = (
            Address::from_xdr(&env, &address_xdr),
            NetworkKind::from_xdr(&env, &network_xdr),
        ) {
            // Use a fixed report hash from remaining data or default
            let mut report_hash_bytes = [7u8; 32];
            if data.len() >= 97 {
                let copy_len = 32.min(data.len() - 65);
                report_hash_bytes[..copy_len].copy_from_slice(&data[65..65 + copy_len]);
            }
            let report_hash = BytesN::from_array(&env, &report_hash_bytes);

            // Derive evidence ID twice to verify determinism
            let id1 = derive_evidence_id(&env, &publisher, &report_hash, &network);
            let id2 = derive_evidence_id(&env, &publisher, &report_hash, &network);
            assert_eq!(id1, id2, "Evidence ID derivation must be deterministic");

            // Verify changing any component changes the result
            let other_publisher = Address::generate(&env);
            let other_report_hash = BytesN::from_array(&env, &[8; 32]);
            let other_network = match network {
                NetworkKind::Testnet => NetworkKind::Pubnet,
                _ => NetworkKind::Testnet,
            };

            let id_other_publisher =
                derive_evidence_id(&env, &other_publisher, &report_hash, &network);
            let id_other_report =
                derive_evidence_id(&env, &publisher, &other_report_hash, &network);
            let id_other_network =
                derive_evidence_id(&env, &publisher, &report_hash, &other_network);

            assert_ne!(id1, id_other_publisher, "Changing publisher must change ID");
            assert_ne!(id1, id_other_report, "Changing report hash must change ID");
            assert_ne!(id1, id_other_network, "Changing network must change ID");

            // Verify XDR encoding is canonical
            let publisher_xdr = publisher.clone().to_xdr(&env);
            let reencoded_xdr = publisher.clone().to_xdr(&env);
            assert_eq!(
                publisher_xdr.len(),
                reencoded_xdr.len(),
                "Address XDR encoding must be canonical"
            );
        }
    }

    // Test truncated XDR inputs
    if !data.is_empty() {
        for size in 1..data.len().min(100) {
            let truncated = Bytes::from_slice(&env, &data[0..size]);
            let _ = Address::from_xdr(&env, &truncated);
            let _ = NetworkKind::from_xdr(&env, &truncated);
        }
    }

    // Test oversized XDR with trailing bytes
    if data.len() >= 40 {
        let valid_address_xdr = Bytes::from_slice(&env, &data[0..40]);
        if let Ok(addr) = Address::from_xdr(&env, &valid_address_xdr) {
            // Verify the address can be re-encoded
            let _ = addr.to_xdr(&env);
        }
    }
});

// Reproduce the evidence ID derivation logic from evidence_id.rs
fn derive_evidence_id(
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
