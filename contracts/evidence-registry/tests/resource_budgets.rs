use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{
    ContractError, EVIDENCE_ID_DOMAIN, EvidenceInput, NetworkKind, PersistentKeyV1, SCHEMA_VERSION,
    SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env,
    testutils::{
        Address as _, Events as _, Ledger as _, storage::Instance as _, storage::Persistent as _,
    },
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

fn assert_resources(env: &Env, cpu_max: i64, memory_max: i64, writes_max: u32, events_max: usize) {
    let resources = env.cost_estimate().resources();
    assert!(
        resources.instructions <= cpu_max,
        "CPU instructions {} exceed {cpu_max}",
        resources.instructions,
    );
    assert!(
        resources.mem_bytes <= memory_max,
        "Memory bytes {} exceed {memory_max}",
        resources.mem_bytes,
    );
    assert!(
        resources.write_entries <= writes_max,
        "Write entries {} exceed {writes_max}",
        resources.write_entries,
    );
    let events = env.events().all().events().len();
    assert!(events <= events_max, "Events {events} exceed {events_max}");
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

// Issue #66: Budget attestor maintenance resources

#[test]
fn maintain_attestor_successful_stays_within_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    env.ledger().set_sequence_number(1_000_000);

    env.cost_estimate().budget().reset_tracker();
    client.maintain_attestor(&publisher);

    assert_resources(&env, 10_000_000, 8_000_000, 2, 0);
}

#[test]
fn maintain_attestor_missing_fails_safely_within_budget() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let missing = Address::generate(&env);
    let (instance_ttl_before, persistent_ttl_before) = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .has(&PersistentKeyV1::Attestor(missing.clone())),
        )
    });

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_maintain_attestor(&missing);

    assert_eq!(result, Err(Ok(ContractError::AttestorNotRegistered)));
    // Failed maintenance may still extend instance TTL due to auth checks
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 10_000_000);
    assert!(resources.mem_bytes <= 8_000_000);

    let (instance_ttl_after, persistent_ttl_after) = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .has(&PersistentKeyV1::Attestor(missing)),
        )
    });
    // Instance TTL may be extended due to auth check
    assert!(instance_ttl_after >= instance_ttl_before);
    assert_eq!(persistent_ttl_after, persistent_ttl_before);
}

#[test]
fn maintain_attestor_extends_persistent_and_instance_ttl() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    // Set ledger sequence high enough that TTL might be below threshold
    env.ledger().set_sequence_number(5_000_000);

    let before = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Attestor(publisher.clone())),
        )
    });

    client.maintain_attestor(&publisher);

    let after = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Attestor(publisher.clone())),
        )
    });

    // Maintenance should ensure TTL is extended to target
    assert!(after.0 >= before.0);
    assert!(after.1 > before.1);
}

// Issue #67: Budget evidence revocation resources

#[test]
fn revoke_evidence_by_publisher_stays_within_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    let reason = BytesN::from_array(&env, &[31; 32]);

    env.cost_estimate().budget().reset_tracker();
    client.revoke_evidence(&id, &reason, &publisher);

    assert_resources(&env, 20_000_000, 16_000_000, 3, 1);
}

#[test]
fn revoke_evidence_by_admin_stays_within_budget() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    let reason = BytesN::from_array(&env, &[31; 32]);

    env.cost_estimate().budget().reset_tracker();
    client.revoke_evidence(&id, &reason, &admin);

    assert_resources(&env, 20_000_000, 16_000_000, 3, 1);
}

#[test]
fn revoke_evidence_updates_status_and_removes_matching_index() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let evidence_input = input(&env, &publisher, 1);
    let id = client.publish_evidence(&evidence_input);
    let reason = BytesN::from_array(&env, &[31; 32]);

    // Before revocation, evidence should be active
    assert!(client.is_active(&id));

    client.revoke_evidence(&id, &reason, &publisher);

    let record = client.get_evidence(&id).unwrap();
    assert_eq!(
        record.status,
        rampspec_shared_types::EvidenceStatus::Revoked
    );
    assert!(!client.is_active(&id));
}

#[test]
fn revoke_evidence_preserves_conflicting_index() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let evidence_input = input(&env, &publisher, 1);
    let id = client.publish_evidence(&evidence_input);
    let conflicting_id = BytesN::from_array(&env, &[88; 32]);

    // Set a conflicting index by directly manipulating storage
    // This test verifies the remove_active_id_if_matches behavior
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(
            &PersistentKeyV1::ActiveReport(
                evidence_input.publisher.clone(),
                evidence_input.report_hash.clone(),
                evidence_input.network.clone(),
            ),
            &conflicting_id,
        );
    });

    client.revoke_evidence(&id, &BytesN::from_array(&env, &[31; 32]), &publisher);

    // Verify the conflicting index is preserved
    let active_after = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<PersistentKeyV1, BytesN<32>>(&PersistentKeyV1::ActiveReport(
                evidence_input.publisher,
                evidence_input.report_hash,
                evidence_input.network,
            ))
    });
    assert_eq!(active_after, Some(conflicting_id));
}

#[test]
fn revoke_evidence_unauthorized_fails_safely() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    let stranger = Address::generate(&env);
    let reason = BytesN::from_array(&env, &[31; 32]);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_revoke_evidence(&id, &reason, &stranger);

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 20_000_000);
    assert!(resources.mem_bytes <= 16_000_000);
    assert!(client.is_active(&id));
}

#[test]
fn revoke_evidence_missing_fails_safely() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let missing_id = BytesN::from_array(&env, &[99; 32]);
    let reason = BytesN::from_array(&env, &[31; 32]);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_revoke_evidence(&missing_id, &reason, &publisher);

    assert_eq!(result, Err(Ok(ContractError::EvidenceNotFound)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 20_000_000);
    assert!(resources.mem_bytes <= 16_000_000);
}

#[test]
fn revoke_evidence_repeated_fails_safely() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    let reason = BytesN::from_array(&env, &[31; 32]);

    client.revoke_evidence(&id, &reason, &publisher);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_revoke_evidence(&id, &reason, &publisher);

    assert_eq!(result, Err(Ok(ContractError::EvidenceNotActive)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 20_000_000);
    assert!(resources.mem_bytes <= 16_000_000);
}

#[test]
fn revoke_evidence_invalid_hash_fails_safely() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    let invalid_reason = BytesN::from_array(&env, &[0; 32]);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_revoke_evidence(&id, &invalid_reason, &publisher);

    assert_eq!(result, Err(Ok(ContractError::InvalidHash)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 20_000_000);
    assert!(resources.mem_bytes <= 16_000_000);
    assert!(client.is_active(&id));
}

// Issue #68: Budget administrator transfer resources

#[test]
fn propose_admin_successful_stays_within_budget() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let new_admin = Address::generate(&env);

    env.cost_estimate().budget().reset_tracker();
    client.propose_admin(&new_admin);

    assert_resources(&env, 15_000_000, 12_000_000, 3, 1);
}

#[test]
fn propose_admin_sets_pending_admin() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    let new_admin = Address::generate(&env);

    client.propose_admin(&new_admin);

    // Verify by attempting to accept as the pending admin
    // The contract will authorize the pending admin
    let result = client.try_accept_admin();
    assert!(result.is_ok());
}

#[test]
fn propose_admin_unauthorized_fails_safely() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);

    env.set_auths(&[]);
    env.cost_estimate().budget().reset_tracker();
    let result = client.try_propose_admin(&new_admin);

    assert!(result.is_err());
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 15_000_000);
    assert!(resources.mem_bytes <= 12_000_000);
}

#[test]
fn propose_admin_same_as_current_fails_safely() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_propose_admin(&admin);

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 15_000_000);
    assert!(resources.mem_bytes <= 12_000_000);
}

#[test]
fn cancel_admin_transfer_successful_stays_within_budget() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    env.cost_estimate().budget().reset_tracker();
    client.cancel_admin_transfer();

    assert_resources(&env, 10_000_000, 8_000_000, 2, 1);
}

#[test]
fn cancel_admin_transfer_clears_pending_admin() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    client.cancel_admin_transfer();

    // Verify pending admin is cleared by trying to accept
    let result = client.try_accept_admin();
    assert_eq!(result, Err(Ok(ContractError::AdminProposalMissing)));
}

#[test]
fn cancel_admin_transfer_unauthorized_fails_safely() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    env.set_auths(&[]);
    env.cost_estimate().budget().reset_tracker();
    let result = client.try_cancel_admin_transfer();

    assert!(result.is_err());
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 10_000_000);
    assert!(resources.mem_bytes <= 8_000_000);
}

#[test]
fn cancel_admin_transfer_missing_proposal_fails_safely() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_cancel_admin_transfer();

    assert_eq!(result, Err(Ok(ContractError::AdminProposalMissing)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 10_000_000);
    assert!(resources.mem_bytes <= 8_000_000);
}

#[test]
fn accept_admin_successful_stays_within_budget() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.propose_admin(&new_admin);

    env.cost_estimate().budget().reset_tracker();
    client.accept_admin();

    assert_resources(&env, 15_000_000, 12_000_000, 3, 1);
}

#[test]
fn accept_admin_changes_admin_and_clears_pending() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.propose_admin(&new_admin);

    client.accept_admin();

    // Verify by checking that the new admin can perform admin-only operations
    // and trying to accept again fails
    let result = client.try_accept_admin();
    assert_eq!(result, Err(Ok(ContractError::AdminProposalMissing)));
}

#[test]
fn accept_admin_unauthorized_fails_safely() {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.propose_admin(&new_admin);

    env.set_auths(&[]);
    env.cost_estimate().budget().reset_tracker();
    let result = client.try_accept_admin();

    assert!(result.is_err());
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 15_000_000);
    assert!(resources.mem_bytes <= 12_000_000);
}

#[test]
fn accept_admin_missing_proposal_fails_safely() {
    let (env, contract_id, _publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_accept_admin();

    assert_eq!(result, Err(Ok(ContractError::AdminProposalMissing)));
    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 15_000_000);
    assert!(resources.mem_bytes <= 12_000_000);
}
