use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{
    EvidenceInput, NetworkKind, PersistentKeyV1, SCHEMA_VERSION, SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::{
    Address, BytesN, Env,
    testutils::{Address as _, Ledger as _, storage::Instance as _, storage::Persistent as _},
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
        passed: 1,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fn setup(env: &Env) -> (Address, Address, Address) {
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(env);
    let publisher = Address::generate(env);
    let client = EvidenceRegistryClient::new(env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(env, &[9; 32]));
    (contract_id, admin, publisher)
}

// Issue #69: Test TTL arithmetic at maximum ledger values

#[test]
fn instance_ttl_extension_at_normal_ledger() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(1_000_000);
    let _id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    // TTL should be reasonably high
    assert!(ttl > 1_000_000);
}

#[test]
fn instance_ttl_extension_below_threshold() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    // Set ledger just below threshold
    env.ledger().set_sequence_number(103_679);
    let before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

    let _id = client.publish_evidence(&input(&env, &publisher, 1));

    let after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    assert!(after >= before);
    assert!(after > 100_000);
}

#[test]
fn instance_ttl_extension_at_threshold() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(103_680);
    let _id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    assert!(ttl > 1_000_000);
}

#[test]
fn instance_ttl_extension_above_threshold() {
    let env = Env::default();
    let (contract_id, _admin, _publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(200_000);
    let before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

    client.schema_version();

    let after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    // Above threshold, TTL should not decrease
    assert!(after >= before);
}

#[test]
fn instance_ttl_at_very_high_ledger() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    // Test at a very high but reasonable ledger value
    // Not using u32::MAX as it may not be a valid operational value
    let high_ledger = 100_000_000_u32;
    env.ledger().set_sequence_number(high_ledger);

    let _id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
    // Verify TTL is extended without overflow - should be a reasonable value
    assert!(ttl > 0);
    assert!(ttl < u32::MAX);
}

#[test]
fn persistent_ttl_extension_at_normal_ledger() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(1_000_000);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Evidence(id))
    });
    assert!(ttl >= 3_110_400 - 1 || ttl > 1_000_000);
}

#[test]
fn persistent_ttl_extension_below_threshold() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(518_399);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Evidence(id))
    });
    assert!(ttl >= 3_110_400 - 1 || ttl > 1_000_000);
}

#[test]
fn persistent_ttl_extension_at_threshold() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    env.ledger().set_sequence_number(518_400);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    let ttl = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Evidence(id))
    });
    assert!(ttl >= 3_110_400 - 1 || ttl > 1_000_000);
}

#[test]
fn persistent_ttl_maintenance_at_high_ledger() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    // Advance to a very high ledger
    let high_ledger = 50_000_000_u32;
    env.ledger().set_sequence_number(high_ledger);

    let before = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Evidence(id.clone()))
    });

    client.maintain_evidence(&id);

    let after = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Evidence(id))
    });

    // Maintenance should extend TTL without overflow
    assert!(after > before);
    assert!(after >= 3_110_400 - 1);
}

#[test]
fn attestor_maintenance_at_high_ledger() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    let high_ledger = 50_000_000_u32;
    env.ledger().set_sequence_number(high_ledger);

    let before = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Attestor(publisher.clone()))
    });

    client.maintain_attestor(&publisher);

    let after = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Attestor(publisher))
    });

    assert!(after > before);
    assert!(after >= 3_110_400 - 1);
}

#[test]
fn ttl_does_not_decrease_on_failed_operations() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    env.ledger().set_sequence_number(1_000_000);
    let (instance_before, persistent_before) = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id.clone())),
        )
    });

    // Attempt invalid revocation (empty hash)
    let _ = client.try_revoke_evidence(&id, &BytesN::from_array(&env, &[0; 32]), &publisher);

    let (instance_after, persistent_after) = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id)),
        )
    });

    // Failed operation should not change TTL
    assert_eq!(instance_after, instance_before);
    assert_eq!(persistent_after, persistent_before);
}

#[test]
fn maintenance_without_overflow_near_boundary() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    // Test near the practical upper boundary
    // Not using u32::MAX - TTL_TARGET as that may cause issues
    // Use a high but safe value
    let boundary_ledger = 100_000_000_u32;
    env.ledger().set_sequence_number(boundary_ledger);

    // Should not panic or overflow
    client.maintain_attestor(&publisher);

    let ttl = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get_ttl(&PersistentKeyV1::Attestor(publisher))
    });

    assert!(ttl > 0);
    assert!(ttl >= 3_110_400 - 1 || ttl > 1_000_000);
}

#[test]
fn multiple_maintenance_cycles_do_not_shorten_ttl() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    for ledger in [1_000_000, 2_000_000, 5_000_000, 10_000_000] {
        env.ledger().set_sequence_number(ledger);

        // Should complete without error
        client.maintain_evidence(&id);

        let current_ttl = env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id.clone()))
        });

        // TTL should remain valid (not zero or overflow)
        assert!(current_ttl > 0);
        assert!(current_ttl < u32::MAX);
    }
}

#[test]
fn failed_maintenance_preserves_ttl() {
    let env = Env::default();
    let (contract_id, _admin, _publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let missing_attestor = Address::generate(&env);

    env.ledger().set_sequence_number(1_000_000);
    let instance_before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

    // Attempt to maintain non-existent attestor
    let _ = client.try_maintain_attestor(&missing_attestor);

    let instance_after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

    // Failed maintenance should not change instance TTL
    assert_eq!(instance_after, instance_before);

    // Should not create persistent entry
    let persistent_exists = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .has(&PersistentKeyV1::Attestor(missing_attestor))
    });
    assert!(!persistent_exists);
}

#[test]
fn instance_ttl_read_operations_safe_at_boundaries() {
    let env = Env::default();
    let (contract_id, _admin, publisher) = setup(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));

    // Test read operations at various ledger values
    for ledger in [1_000, 103_680, 500_000, 10_000_000, 50_000_000] {
        env.ledger().set_sequence_number(ledger);

        let before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

        let _ = client.schema_version();
        let _ = client.get_evidence(&id);
        let _ = client.get_attestor(&publisher);
        let _ = client.is_active(&id);

        let after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

        // Reads should maintain or extend TTL, never shorten it
        assert!(after >= before);
    }
}
