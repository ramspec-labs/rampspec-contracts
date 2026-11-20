use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{
    EVIDENCE_ID_DOMAIN, EvidenceInput, NetworkKind, PersistentKeyV1, SCHEMA_VERSION,
    SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env,
    testutils::{Address as _, Ledger as _, storage::Instance as _, storage::Persistent as _},
    xdr::ToXdr,
};

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

fn evidence_id(
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

#[test]
fn maximum_valid_publication_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    env.cost_estimate().budget().reset_tracker();

    client.publish_evidence(&input(&env, &publisher, 1));

    assert_budget(&env, 20_000_000, 16_000_000);
}

#[test]
fn maximum_valid_supersession_walk_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let current = client.publish_evidence(&input(&env, &publisher, 1));
    let replacement = input(&env, &publisher, 2);
    let replacement_id = evidence_id(
        &env,
        &replacement.publisher,
        &replacement.report_hash,
        &replacement.network,
    );
    env.as_contract(&contract_id, || {
        let mut cursor = replacement_id;
        for marker in 1..=24_u8 {
            let next = BytesN::from_array(&env, &[100 + marker; 32]);
            env.storage()
                .persistent()
                .set(&PersistentKeyV1::SupersededBy(cursor), &next);
            cursor = next;
        }
    });
    env.cost_estimate().budget().reset_tracker();

    client.supersede_evidence(&current, &replacement, &publisher);

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 1_250_000);
    assert!(resources.mem_bytes <= 140_000);
    assert!(resources.memory_read_entries <= 40);
    assert!(resources.write_entries <= 7);
}

#[test]
fn long_replacement_chain_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let current = client.publish_evidence(&input(&env, &publisher, 1));
    let replacement = input(&env, &publisher, 2);
    let replacement_id = evidence_id(
        &env,
        &replacement.publisher,
        &replacement.report_hash,
        &replacement.network,
    );
    env.as_contract(&contract_id, || {
        let mut cursor = replacement_id;
        for marker in 1..=24_u8 {
            let next = BytesN::from_array(&env, &[100 + marker; 32]);
            env.storage()
                .persistent()
                .set(&PersistentKeyV1::SupersededBy(cursor), &next);
            cursor = next;
        }
    });
    env.cost_estimate().budget().reset_tracker();

    client.supersede_evidence(&current, &replacement, &publisher);

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 1_250_000);
    assert!(resources.mem_bytes <= 140_000);
    assert!(resources.memory_read_entries <= 40);
    assert!(resources.write_entries <= 7);
}

#[test]
fn successful_evidence_read_extends_only_instance_ttl_within_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    env.ledger().set_sequence_number(2_000_000);
    let before = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id.clone())),
        )
    });

    assert!(client.get_evidence(&id).is_some());

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 5_000_000);
    assert!(resources.mem_bytes <= 5_000_000);
    assert!(resources.memory_read_entries <= 4);
    assert!(resources.write_entries <= 2);
    let after = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id)),
        )
    });
    assert!(after.0 > before.0);
    assert_eq!(after.1, before.1);
}

#[test]
fn revoked_evidence_maintenance_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    client.revoke_evidence(&id, &BytesN::from_array(&env, &[8; 32]), &publisher);

    env.ledger().set_sequence_number(1_500_000);
    client.set_paused(&false);
    env.ledger().set_sequence_number(2_600_000);
    env.cost_estimate().budget().reset_tracker();

    client.maintain_evidence(&id);

    assert_budget(&env, 10_000_000, 8_000_000);
}

#[test]
fn maximum_supersession_chain_maintenance_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let mut current = client.publish_evidence(&input(&env, &publisher, 1));
    let first = current.clone();
    for marker in 2..=24 {
        current = client.supersede_evidence(&current, &input(&env, &publisher, marker), &publisher);
    }
    env.cost_estimate().budget().reset_tracker();

    client.maintain_supersession_chain(&first);

    assert_budget(&env, 80_000_000, 64_000_000);
}

#[test]
fn successful_evidence_read_extends_only_instance_ttl_within_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    env.ledger().set_sequence_number(2_000_000);
    let before = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id.clone())),
        )
    });

    assert!(client.get_evidence(&id).is_some());

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 5_000_000);
    assert!(resources.mem_bytes <= 5_000_000);
    assert!(resources.memory_read_entries <= 4);
    assert!(resources.write_entries <= 2);
    let after = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id)),
        )
    });
    assert!(after.0 > before.0);
    assert_eq!(after.1, before.1);
}
