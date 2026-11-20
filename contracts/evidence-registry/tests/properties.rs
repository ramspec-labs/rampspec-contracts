use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use proptest::prelude::*;
use rampspec_shared_types::{
    ContractError, EvidenceInput, EvidenceStatus, NetworkKind, PersistentKeyV1, SCHEMA_VERSION,
};
use soroban_sdk::{
    Address, BytesN, Env, testutils::Address as _, testutils::Events as _, testutils::Ledger as _,
    testutils::storage::Persistent as _,
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
