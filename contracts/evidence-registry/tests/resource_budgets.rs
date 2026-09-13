use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{
    EvidenceInput, NetworkKind, SCHEMA_VERSION, SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::{Address, BytesN, Env, testutils::Address as _};

fn input(env: &Env, publisher: &Address, marker: u8) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[marker; 32]),
        target_hash: BytesN::from_array(env, &[2; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: SUPPORTED_PROTOCOL_BITMAP,
        score_bps: 10_000,
        passed: u32::MAX,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    (env, contract_id, publisher)
}

fn assert_budget(env: &Env, cpu_max: u64, memory_max: u64) {
    let budget = env.cost_estimate().budget();
    let cpu = budget.cpu_instruction_cost();
    let memory = budget.memory_bytes_cost();
    assert!(cpu <= cpu_max, "CPU cost {cpu} exceeds {cpu_max}");
    assert!(
        memory <= memory_max,
        "memory cost {memory} exceeds {memory_max}"
    );
}

#[test]
fn maximum_valid_publication_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    env.cost_estimate().budget().reset_tracker();

    client.publish_evidence(&input(&env, &publisher, 1));

    assert_budget(&env, 20_000_000, 16_000_000);
}

#[test]
fn long_replacement_chain_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let mut current = client.publish_evidence(&input(&env, &publisher, 1));
    for marker in 2..=12 {
        current = client.supersede_evidence(&current, &input(&env, &publisher, marker), &publisher);
    }
    env.cost_estimate().budget().reset_tracker();

    client.supersede_evidence(&current, &input(&env, &publisher, 13), &publisher);

    assert_budget(&env, 30_000_000, 24_000_000);
}
