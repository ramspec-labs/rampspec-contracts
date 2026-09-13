#![no_main]

use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use libfuzzer_sys::fuzz_target;
use rampspec_shared_types::{EvidenceInput, NetworkKind, SCHEMA_VERSION};
use soroban_sdk::{Address, BytesN, Env, testutils::Address as _};

fn input(env: &Env, publisher: &Address, report_byte: u8) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[report_byte.max(1); 32]),
        target_hash: BytesN::from_array(env, &[2; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: 1,
        score_bps: 5_000,
        passed: 1,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    let mut active = None;

    for (index, action) in data.iter().take(32).enumerate() {
        match action % 4 {
            0 => {
                let evidence = input(&env, &publisher, u8::try_from(index + 1).unwrap());
                if let Ok(Ok(id)) = client.try_publish_evidence(&evidence) {
                    active = Some(id);
                }
            }
            1 => {
                let _ = client.try_set_paused(&(action & 0x80 != 0));
            }
            2 => {
                if let Some(id) = active.clone() {
                    let reason = BytesN::from_array(&env, &[7; 32]);
                    if client.try_revoke_evidence(&id, &reason, &publisher).is_ok() {
                        active = None;
                    }
                }
            }
            _ => {
                if let Some(id) = active.clone() {
                    let replacement = input(&env, &publisher, u8::try_from(index + 100).unwrap());
                    if let Ok(Ok(next_id)) =
                        client.try_supersede_evidence(&id, &replacement, &publisher)
                    {
                        active = Some(next_id);
                    }
                }
            }
        }
    }
});
