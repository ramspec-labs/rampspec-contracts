use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use proptest::prelude::*;
use rampspec_shared_types::{
    ContractError, EvidenceInput, EvidenceStatus, NetworkKind, SCHEMA_VERSION,
};
use soroban_sdk::{Address, BytesN, Env, testutils::Address as _};

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

fn input(
    env: &Env,
    publisher: &Address,
    report_byte: u8,
    target_byte: u8,
    score_bps: u32,
    passed: u32,
    bitmap: u64,
) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[report_byte; 32]),
        target_hash: BytesN::from_array(env, &[target_byte; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
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
        let first = input(&env, &publisher, start, 2, 5_000, 1, 1);
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
    fn revocation_is_terminal_for_arbitrary_valid_records(
        report_byte in 1u8..=250,
        reason_byte in 1u8..=250,
        by_admin in any::<bool>(),
    ) {
        let (env, contract_id, admin, publisher) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let evidence = input(&env, &publisher, report_byte, 2, 5_000, 1, 1);
        let id = client.publish_evidence(&evidence);
        let revoker = if by_admin { admin } else { publisher };
        let reason = BytesN::from_array(&env, &[reason_byte; 32]);
        client.revoke_evidence(&id, &reason, &revoker);

        prop_assert!(!client.is_active(&id));
        prop_assert_eq!(
            client.try_revoke_evidence(&id, &reason, &revoker),
            Err(Ok(ContractError::EvidenceNotActive))
        );
    }
}
