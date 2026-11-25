use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use proptest::prelude::*;
use rampspec_shared_types::{
    ContractError, EvidenceInput, EvidenceStatus, NetworkKind, PersistentKeyV1, SCHEMA_VERSION,
};
use soroban_sdk::{
    Address, BytesN, Env, testutils::Address as _, testutils::Events as _, testutils::Ledger as _,
    testutils::storage::Persistent as _,
};
use std::collections::BTreeMap;
use rampspec_shared_types::{
    EvidenceInput, NetworkKind, PersistentKeyV1, SCHEMA_VERSION, SUPPORTED_PROTOCOL_BITMAP,
};
use rampspec_test_support::{
    AdminOp, AttestorOp, EvidenceIndexOp, admin_op_strategy, attestor_op_strategy,
    evidence_index_op_strategy,
};
use soroban_sdk::{
    Address, BytesN, Env,
    testutils::{
        Address as _, Events as _, MockAuth, MockAuthInvoke,
        storage::{Instance as _, Persistent as _},
    },
    IntoVal,
};
const PERSISTENT_TTL_TARGET: u32 = 3_110_400;

fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    (env, contract_id, admin, publisher)
}

#[allow(clippy::too_many_arguments)]
fn input(
    env: &Env,
    publisher: &Address,
    report_byte: u8,
    target_byte: u8,
    score_bps: u32,
    passed: u32,
    bitmap: u64,
    network: NetworkKind,
) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[report_byte; 32]),
        target_hash: BytesN::from_array(env, &[target_byte; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network,
        protocol_bitmap: bitmap,
        score_bps,
        passed,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn arbitrary_evidence_ids_are_deterministic_across_equivalent_inputs(
        report_byte in 1u8..=255,
        other_report_byte in 1u8..=255,
        network_variant in 0u8..=3,
        other_network_variant in 0u8..=3,
        use_other_publisher in any::<bool>(),
    ) {
        let network = match network_variant {
            0 => NetworkKind::Testnet,
            1 => NetworkKind::Futurenet,
            2 => NetworkKind::Pubnet,
            _ => NetworkKind::Standalone,
        };
        let other_network = match other_network_variant {
            0 => NetworkKind::Testnet,
            1 => NetworkKind::Futurenet,
            2 => NetworkKind::Pubnet,
            _ => NetworkKind::Standalone,
        };
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EvidenceRegistry, ());
        let admin = Address::generate(&env);
        let publisher = Address::from_str(
            &env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        );
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));

        let baseline = input(
            &env,
            &publisher,
            report_byte,
            2,
            5_000,
            1,
            1,
            network.clone(),
        );
        let baseline_id = client.publish_evidence(&baseline);
        prop_assert_eq!(
            client.try_publish_evidence(&baseline),
            Err(Ok(ContractError::EvidenceAlreadyExists))
        );

        let equivalent_env = Env::default();
        equivalent_env.mock_all_auths();
        let equivalent_contract = equivalent_env.register(EvidenceRegistry, ());
        let equivalent_admin = Address::generate(&equivalent_env);
        let equivalent_publisher = Address::from_str(
            &equivalent_env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        );
        let equivalent_client = EvidenceRegistryClient::new(
            &equivalent_env,
            &equivalent_contract,
        );
        equivalent_client.initialize(&equivalent_admin, &SCHEMA_VERSION);
        equivalent_client.register_attestor(
            &equivalent_publisher,
            &BytesN::from_array(&equivalent_env, &[9; 32]),
        );
        let equivalent = input(
            &equivalent_env,
            &equivalent_publisher,
            report_byte,
            2,
            5_000,
            1,
            1,
            match network_variant {
                0 => NetworkKind::Testnet,
                1 => NetworkKind::Futurenet,
                2 => NetworkKind::Pubnet,
                _ => NetworkKind::Standalone,
            },
        );
        prop_assert_eq!(
            equivalent_client.publish_evidence(&equivalent),
            baseline_id.clone()
        );

        if other_report_byte != report_byte {
            let changed_report = input(
                &env,
                &publisher,
                other_report_byte,
                2,
                5_000,
                1,
                1,
                network.clone(),
            );
            let changed_report_id = client.publish_evidence(&changed_report);
            prop_assert_ne!(changed_report_id, baseline_id.clone());
        }

        if other_network_variant != network_variant {
            let changed_network = input(
                &env,
                &publisher,
                report_byte,
                2,
                5_000,
                1,
                1,
                other_network,
            );
            let changed_network_id = client.publish_evidence(&changed_network);
            prop_assert_ne!(changed_network_id, baseline_id.clone());
        }

        if use_other_publisher {
            let other_publisher = Address::generate(&env);
            client.register_attestor(
                &other_publisher,
                &BytesN::from_array(&env, &[10; 32]),
            );
            let changed_publisher = input(
                &env,
                &other_publisher,
                report_byte,
                2,
                5_000,
                1,
                1,
                network.clone(),
            );
            let changed_publisher_id = client.publish_evidence(&changed_publisher);
            prop_assert_ne!(changed_publisher_id, baseline_id);
        }
    }

    #[test]
    fn arbitrary_valid_records_remain_unique(
        report_byte in 1u8..=250,
        target_byte in 1u8..=250,
        replacement_target in 1u8..=250,
        score_bps in 0u32..=10_000,
        passed in 1u32..=10_000,
        bitmap in 1u64..=255,
    ) {
        let (env, contract_id, _admin, publisher) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let original = input(
            &env,
            &publisher,
            report_byte,
            target_byte,
            score_bps,
            passed,
            bitmap,
            NetworkKind::Testnet,
        );
        let id = client.publish_evidence(&original);
        let mut duplicate = original.clone();
        duplicate.target_hash = BytesN::from_array(&env, &[replacement_target; 32]);

        prop_assert_eq!(
            client.try_publish_evidence(&duplicate),
            Err(Ok(ContractError::EvidenceAlreadyExists))
        );
        let stored = client.get_evidence(&id).unwrap();
        prop_assert_eq!(stored.target_hash, original.target_hash);
        prop_assert_eq!(stored.status, EvidenceStatus::Active);
    }

    #[test]
    fn arbitrary_replacement_chains_are_acyclic(
        start in 1u8..=200,
        chain_length in 1u8..=12,
    ) {
        let (env, contract_id, _admin, publisher) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let first = input(
            &env,
            &publisher,
            start,
            2,
            5_000,
            1,
            1,
            NetworkKind::Testnet,
        );
        let mut current_id = client.publish_evidence(&first);

        for offset in 1..=chain_length {
            let replacement = input(
                &env,
                &publisher,
                start + offset,
                offset,
                5_000,
                u32::from(offset),
                1,
                NetworkKind::Testnet,
            );
            let next_id = client.supersede_evidence(&current_id, &replacement, &publisher);
            prop_assert_ne!(next_id.clone(), current_id.clone());
            prop_assert_eq!(
                client.get_evidence(&current_id).unwrap().status,
                EvidenceStatus::Superseded
            );
            prop_assert_eq!(
                client.get_evidence(&next_id).unwrap().supersedes,
                Some(current_id)
            );
            current_id = next_id;
        }
        prop_assert!(client.is_active(&current_id));
    }

    #[test]
    fn arbitrary_supersession_chains_are_maintained_to_target(
        start in 1u8..=200,
        chain_length in 1u8..=12,
    ) {
        let (env, contract_id, _admin, publisher) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let first = input(
            &env,
            &publisher,
            start,
            2,
            5_000,
            1,
            1,
            NetworkKind::Testnet,
        );
        let first_id = client.publish_evidence(&first);
        let mut current_id = first_id.clone();
        let mut ids = vec![first_id];

        for offset in 1..=chain_length {
            env.ledger()
                .set_sequence_number(env.ledger().sequence() + 100_000);
            let replacement = input(
                &env,
                &publisher,
                start + offset,
                offset,
                5_000,
                u32::from(offset),
                1,
                NetworkKind::Testnet,
            );
            current_id = client.supersede_evidence(&current_id, &replacement, &publisher);
            ids.push(current_id.clone());
        }

        env.ledger()
            .set_sequence_number(env.ledger().sequence() + 1_800_000);
        let before = env.as_contract(&contract_id, || {
            ids.iter()
                .enumerate()
                .map(|(index, id)| {
                    let evidence_ttl = env
                        .storage()
                        .persistent()
                        .get_ttl(&PersistentKeyV1::Evidence(id.clone()));
                    let link_ttl = (index + 1 < ids.len()).then(|| {
                        env.storage()
                            .persistent()
                            .get_ttl(&PersistentKeyV1::SupersededBy(id.clone()))
                    });
                    (evidence_ttl, link_ttl)
                })
                .collect::<Vec<_>>()
        });
        let all_ttls_below_target = before.iter().all(|(evidence_ttl, link_ttl)| {
            *evidence_ttl < PERSISTENT_TTL_TARGET
                && link_ttl.is_none_or(|ttl| ttl < PERSISTENT_TTL_TARGET)
        });
        prop_assert!(all_ttls_below_target);
        prop_assert!(before.windows(2).any(|window| window[0] != window[1]));
        let event_count = env.events().all().events().len();

        client.maintain_supersession_chain(&ids[0]);
        prop_assert_eq!(env.events().all().events().len(), event_count);

        let after = env.as_contract(&contract_id, || {
            ids.iter()
                .enumerate()
                .map(|(index, id)| {
                    let evidence_ttl = env
                        .storage()
                        .persistent()
                        .get_ttl(&PersistentKeyV1::Evidence(id.clone()));
                    let link_ttl = (index + 1 < ids.len()).then(|| {
                        env.storage()
                            .persistent()
                            .get_ttl(&PersistentKeyV1::SupersededBy(id.clone()))
                    });
                    (evidence_ttl, link_ttl)
                })
                .collect::<Vec<_>>()
        });
        let all_ttls_at_target = after.iter().all(|(evidence_ttl, link_ttl)| {
            *evidence_ttl >= PERSISTENT_TTL_TARGET - 1
                && link_ttl.is_none_or(|ttl| ttl >= PERSISTENT_TTL_TARGET - 1)
        });
        prop_assert!(all_ttls_at_target);
    }

    #[test]
    fn revocation_is_terminal_for_arbitrary_valid_records(
        report_byte in 1u8..=250,
        reason_byte in 1u8..=250,
        by_admin in any::<bool>(),
    ) {
        let (env, contract_id, admin, publisher) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let evidence = input(
            &env,
            &publisher,
            report_byte,
            2,
            5_000,
            1,
            1,
            NetworkKind::Testnet,
        );
        let id = client.publish_evidence(&evidence);
        let revoker = if by_admin { admin } else { publisher };
        let reason = BytesN::from_array(&env, &[reason_byte; 32]);
        client.revoke_evidence(&id, &reason, &revoker);

        prop_assert!(!client.is_active(&id));
        let event_count = env.events().all().events().len();
        client.maintain_evidence(&id);
        prop_assert_eq!(
            client.get_evidence(&id).unwrap().status,
            EvidenceStatus::Revoked
        );
        prop_assert_eq!(env.events().all().events().len(), event_count);
        prop_assert_eq!(
            client.try_revoke_evidence(&id, &reason, &revoker),
            Err(Ok(ContractError::EvidenceNotActive))
        );
    }
}

fn make_evidence_input(env: &Env, publisher: &Address, report_marker: u8) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[report_marker; 32]),
        target_hash: BytesN::from_array(env, &[2; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: SUPPORTED_PROTOCOL_BITMAP,
        score_bps: 10_000,
        passed: 10,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

// ============================================================================
// #62 — Attestor Lifecycle State-Machine Property Tests
// ============================================================================
proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn attestor_lifecycle_preserves_state_machine_and_no_ghost_effects(
        ops in prop::collection::vec(attestor_op_strategy(4), 1..30)
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EvidenceRegistry, ());
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &SCHEMA_VERSION);

        let pool: Vec<Address> = (0..4).map(|_| Address::generate(&env)).collect();
        // Model: actor_idx -> (enabled, metadata_marker)
        let mut model: BTreeMap<usize, (bool, u8)> = BTreeMap::new();

        for op in ops {
            let events_before = env.events().all().len();
            let instance_ttl_before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

            match op {
                AttestorOp::Register { actor_idx, meta_marker } => {
                    let addr = &pool[actor_idx];
                    let meta = BytesN::from_array(&env, &[meta_marker; 32]);
                    let res = client.try_register_attestor(addr, &meta);

                    if model.contains_key(&actor_idx) {
                        prop_assert!(res.is_err(), "duplicate registration must fail");
                        prop_assert_eq!(env.events().all().len(), events_before);
                        let ttl_after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
                        prop_assert_eq!(ttl_after, instance_ttl_before);
                    } else {
                        prop_assert!(res.is_ok(), "fresh registration must succeed");
                        model.insert(actor_idx, (true, meta_marker));
                    }
                }
                AttestorOp::SetEnabled { actor_idx, enabled } => {
                    let addr = &pool[actor_idx];
                    let res = client.try_set_attestor_enabled(addr, &enabled);

                    match model.get_mut(&actor_idx) {
                        None => {
                            prop_assert!(res.is_err(), "unregistered attestor toggle must fail");
                            prop_assert_eq!(env.events().all().len(), events_before);
                        }
                        Some((current_enabled, _)) => {
                            prop_assert!(res.is_ok());
                            let was_noop = *current_enabled == enabled;
                            *current_enabled = enabled;
                            if was_noop {
                                // Idempotent toggle must not emit ghost events
                                prop_assert_eq!(env.events().all().len(), events_before);
                            }
                        }
                    }
                }
                AttestorOp::UnauthorizedSetEnabled { actor_idx, enabled } => {
                    let addr = &pool[actor_idx];
                    let outsider = Address::generate(&env);
                    let res = client
                        .mock_auths(&[MockAuth {
                            address: &outsider,
                            invoke: &MockAuthInvoke {
                                contract: &contract_id,
                                fn_name: "set_attestor_enabled",
                                args: (addr.clone(), enabled).into_val(&env),
                                sub_invokes: &[],
                            },
                        }])
                        .try_set_attestor_enabled(addr, &enabled);
                    prop_assert!(res.is_err(), "unauthorized toggle must fail");
                    prop_assert_eq!(env.events().all().len(), events_before);
                }
                AttestorOp::Maintain { actor_idx } => {
                    let addr = &pool[actor_idx];
                    let res = client.try_maintain_attestor(addr);
                    if model.contains_key(&actor_idx) {
                        prop_assert!(res.is_ok());
                    } else {
                        prop_assert!(res.is_err());
                        prop_assert_eq!(env.events().all().len(), events_before);
                    }
                }
            }

            // Reconcile contract state against reference model after every step
            for (idx, addr) in pool.iter().enumerate() {
                let on_chain = client.get_attestor(addr);
                match model.get(&idx) {
                    None => prop_assert!(on_chain.is_none()),
                    Some((expected_enabled, _)) => {
                        let rec = on_chain.expect("registered attestor must exist");
                        prop_assert_eq!(rec.enabled, *expected_enabled);
                    }
                }
            }
        }
    }
}

// ============================================================================
// #64 — Active Index Bijection Invariants
// ============================================================================
proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn active_index_maintains_strict_bijection_across_transitions(
        ops in prop::collection::vec(evidence_index_op_strategy(3), 1..25)
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EvidenceRegistry, ());
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.initialize(&admin, &SCHEMA_VERSION);

        let publishers: Vec<Address> = (0..3)
            .map(|i| {
                let p = Address::generate(&env);
                client.register_attestor(&p, &BytesN::from_array(&env, &[10 + i as u8; 32]));
                p
            })
            .collect();

        // Track all created evidence IDs and expected active mapping: (publisher_idx, report_marker) -> id
        let mut all_ids: Vec<(BytesN<32>, usize, u8)> = Vec::new();
        let mut active_by_key: BTreeMap<(usize, u8), BytesN<32>> = BTreeMap::new();

        for op in ops {
            let events_before = env.events().all().len();

            match op {
                EvidenceIndexOp::Publish { publisher_idx, report_marker } => {
                    let inp = make_evidence_input(&env, &publishers[publisher_idx], report_marker);
                    let res = client.try_publish_evidence(&inp);
                    if active_by_key.contains_key(&(publisher_idx, report_marker)) {
                        prop_assert!(res.is_err(), "duplicate active publication must fail");
                        prop_assert_eq!(env.events().all().len(), events_before);
                    } else if let Ok(Ok(id)) = res {
                        active_by_key.insert((publisher_idx, report_marker), id.clone());
                        all_ids.push((id, publisher_idx, report_marker));
                    }
                }
                EvidenceIndexOp::Revoke { record_idx, authorized } => {
                    if all_ids.is_empty() { continue; }
                    let (id, pub_idx, marker) = all_ids[record_idx % all_ids.len()].clone();
                    let caller = if authorized { publishers[pub_idx].clone() } else { Address::generate(&env) };
                    let reason = BytesN::from_array(&env, &[9; 32]);
                    let res = client.try_revoke_evidence(&id, &reason, &caller);

                    let was_active = active_by_key.get(&(pub_idx, marker)) == Some(&id);
                    if authorized && was_active {
                        prop_assert!(res.is_ok());
                        active_by_key.remove(&(pub_idx, marker));
                    } else {
                        prop_assert!(res.is_err());
                        prop_assert_eq!(env.events().all().len(), events_before);
                    }
                }
                EvidenceIndexOp::Supersede { record_idx, replacement_marker, authorized } => {
                    if all_ids.is_empty() { continue; }
                    let (id, pub_idx, marker) = all_ids[record_idx % all_ids.len()].clone();
                    let caller = if authorized { publishers[pub_idx].clone() } else { Address::generate(&env) };
                    let replacement = make_evidence_input(&env, &publishers[pub_idx], replacement_marker);
                    let res = client.try_supersede_evidence(&id, &replacement, &caller);

                    let was_active = active_by_key.get(&(pub_idx, marker)) == Some(&id);
                    let target_taken = active_by_key.contains_key(&(pub_idx, replacement_marker))
                        && replacement_marker != marker;

                    if authorized && was_active && !target_taken {
                        if let Ok(Ok(new_id)) = res {
                            active_by_key.remove(&(pub_idx, marker));
                            active_by_key.insert((pub_idx, replacement_marker), new_id.clone());
                            all_ids.push((new_id, pub_idx, replacement_marker));
                        }
                    } else {
                        prop_assert!(res.is_err());
                        prop_assert_eq!(env.events().all().len(), events_before);
                    }
                }
            }

            // Bijection check: every known ID's `is_active` must agree 1:1 with `active_by_key`
            for (id, pub_idx, marker) in &all_ids {
                let expected_active = active_by_key.get(&(*pub_idx, *marker)) == Some(id);
                prop_assert_eq!(client.is_active(id), expected_active);
            }
        }
    }
}

// ============================================================================
// #65 — Randomized Administrator Lifecycle Transitions
// ============================================================================
proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn administrator_lifecycle_preserves_single_admin_and_pending_proposal_invariant(
        ops in prop::collection::vec(admin_op_strategy(4), 1..30)
    ) {
        let env = Env::default();
        let contract_id = env.register(EvidenceRegistry, ());
        let client = EvidenceRegistryClient::new(&env, &contract_id);

        let pool: Vec<Address> = (0..4).map(|_| Address::generate(&env)).collect();
        let mut current_admin_idx = 0usize;
        let mut pending_admin_idx: Option<usize> = None;

        env.mock_all_auths();
        client.initialize(&pool[current_admin_idx], &SCHEMA_VERSION);

        for op in ops {
            let events_before = env.events().all().len();
            let ttl_before = env.as_contract(&contract_id, || env.storage().instance().get_ttl());

            match op {
                AdminOp::Propose { candidate_idx, by_admin } => {
                    let caller_idx = if by_admin { current_admin_idx } else { (current_admin_idx + 1) % pool.len() };
                    let candidate = &pool[candidate_idx];
                    let res = client
                        .mock_auths(&[MockAuth {
                            address: &pool[caller_idx],
                            invoke: &MockAuthInvoke {
                                contract: &contract_id,
                                fn_name: "propose_admin",
                                args: (candidate.clone(),).into_val(&env),
                                sub_invokes: &[],
                            },
                        }])
                        .try_propose_admin(candidate);

                    if by_admin && candidate_idx != current_admin_idx {
                        prop_assert!(res.is_ok(), "admin proposing distinct candidate must succeed");
                        pending_admin_idx = Some(candidate_idx);
                    } else {
                        prop_assert!(res.is_err(), "unauthorized or self-proposal must fail");
                        prop_assert_eq!(env.events().all().len(), events_before);
                        let ttl_after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
                        prop_assert_eq!(ttl_after, ttl_before);
                    }
                }
                AdminOp::Cancel { by_admin } => {
                    let caller_idx = if by_admin { current_admin_idx } else { (current_admin_idx + 1) % pool.len() };
                    let res = client
                        .mock_auths(&[MockAuth {
                            address: &pool[caller_idx],
                            invoke: &MockAuthInvoke {
                                contract: &contract_id,
                                fn_name: "cancel_admin_transfer",
                                args: ().into_val(&env),
                                sub_invokes: &[],
                            },
                        }])
                        .try_cancel_admin_transfer();

                    if by_admin && pending_admin_idx.is_some() {
                        prop_assert!(res.is_ok());
                        pending_admin_idx = None;
                    } else {
                        prop_assert!(res.is_err());
                        prop_assert_eq!(env.events().all().len(), events_before);
                        let ttl_after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
                        prop_assert_eq!(ttl_after, ttl_before);
                    }
                }
                AdminOp::Accept { caller_idx } => {
                    let res = client
                        .mock_auths(&[MockAuth {
                            address: &pool[caller_idx],
                            invoke: &MockAuthInvoke {
                                contract: &contract_id,
                                fn_name: "accept_admin",
                                args: ().into_val(&env),
                                sub_invokes: &[],
                            },
                        }])
                        .try_accept_admin();

                    if pending_admin_idx == Some(caller_idx) {
                        prop_assert!(res.is_ok(), "pending admin accepting must succeed");
                        current_admin_idx = caller_idx;
                        pending_admin_idx = None;
                    } else {
                        prop_assert!(res.is_err(), "non-pending caller accepting must fail");
                        prop_assert_eq!(env.events().all().len(), events_before);
                        let ttl_after = env.as_contract(&contract_id, || env.storage().instance().get_ttl());
                        prop_assert_eq!(ttl_after, ttl_before);
                    }
                }
            }
        }
    }
}