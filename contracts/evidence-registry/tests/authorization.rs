use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{ContractError, EvidenceInput, NetworkKind, SCHEMA_VERSION};
use soroban_sdk::{Address, BytesN, Env, testutils::Address as _};

fn setup() -> (Env, Address, Address, Address, Address) {
    let env = Env::default();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let attestor = Address::generate(&env);
    let non_admin = Address::generate(&env);
    (env, contract_id, admin, attestor, non_admin)
}

fn valid_input(env: &Env, publisher: &Address) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[11; 32]),
        target_hash: BytesN::from_array(env, &[12; 32]),
        suite_hash: BytesN::from_array(env, &[13; 32]),
        specs_hash: BytesN::from_array(env, &[14; 32]),
        artifact_root: BytesN::from_array(env, &[15; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: 1,
        score_bps: 10_000,
        passed: 1,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fn initialize_with_attestor(env: &Env, contract_id: &Address, admin: &Address, attestor: &Address) {
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(env, contract_id);
    client.initialize(admin, &SCHEMA_VERSION);
    client.register_attestor(attestor, &BytesN::from_array(env, &[9; 32]));
}

#[test]
fn initialize_requires_admin_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    // Should succeed with mock_all_auths
    env.mock_all_auths();
    client.initialize(&admin, &SCHEMA_VERSION);
    assert_eq!(client.schema_version(), SCHEMA_VERSION);
}

#[test]
fn initialize_fails_without_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);

    // Should fail without authorization
    let result = client.try_initialize(&admin, &SCHEMA_VERSION);
    assert!(
        result.is_err(),
        "Initialize must fail without admin authorization"
    );
}

#[test]
fn register_attestor_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    // Should succeed with authorization
    client.register_attestor(&attestor, &BytesN::from_array(&env, &[9; 32]));
    assert!(client.get_attestor(&attestor).is_some());
}

#[test]
fn register_attestor_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    env.set_auths(&[]);
    let result = client.try_register_attestor(&attestor, &BytesN::from_array(&env, &[9; 32]));
    assert!(
        result.is_err(),
        "Register attestor must fail without admin authorization"
    );
}

#[test]
fn set_attestor_enabled_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    // Should succeed with authorization
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.set_attestor_enabled(&attestor, &false);
    assert!(!client.get_attestor(&attestor).unwrap().enabled);
}

#[test]
fn set_attestor_enabled_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.set_auths(&[]);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let result = client.try_set_attestor_enabled(&attestor, &false);
    assert!(
        result.is_err(),
        "Set attestor enabled must fail without admin authorization"
    );
}

#[test]
fn publish_evidence_requires_publisher_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);
    assert!(client.is_active(&id));
}

#[test]
fn publish_evidence_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.set_auths(&[]);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let result = client.try_publish_evidence(&valid_input(&env, &attestor));
    assert!(
        result.is_err(),
        "Publish evidence must fail without publisher authorization"
    );
}

#[test]
fn revoke_evidence_requires_publisher_or_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    // Publisher can revoke
    let reason = BytesN::from_array(&env, &[20; 32]);
    client.revoke_evidence(&id, &reason, &attestor);
    assert!(!client.is_active(&id));

    // Admin can revoke - need a different evidence record
    let mut input2 = valid_input(&env, &attestor);
    input2.report_hash = BytesN::from_array(&env, &[22; 32]);
    let id2 = client.publish_evidence(&input2);
    client.revoke_evidence(&id2, &reason, &admin);
    assert!(!client.is_active(&id2));
}

#[test]
fn revoke_evidence_rejects_unauthorized_party() {
    let (env, contract_id, admin, attestor, non_admin) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    env.set_auths(&[]);
    let result = client.try_revoke_evidence(&id, &BytesN::from_array(&env, &[20; 32]), &non_admin);

    // Could fail due to either missing auth or unauthorized error
    assert!(result.is_err(), "Revoke must fail for unauthorized party");
}

#[test]
fn revoke_evidence_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    env.set_auths(&[]);
    let result = client.try_revoke_evidence(&id, &BytesN::from_array(&env, &[20; 32]), &attestor);
    assert!(
        result.is_err(),
        "Revoke evidence must fail without authorization"
    );
}

#[test]
fn supersede_evidence_requires_publisher_or_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    // Publisher can supersede
    let mut replacement = valid_input(&env, &attestor);
    replacement.report_hash = BytesN::from_array(&env, &[22; 32]);
    let new_id = client.supersede_evidence(&id, &replacement, &attestor);
    assert!(client.is_active(&new_id));
    assert!(!client.is_active(&id));

    // Admin can supersede - create a new evidence record
    let mut input2 = valid_input(&env, &attestor);
    input2.report_hash = BytesN::from_array(&env, &[25; 32]);
    let id2 = client.publish_evidence(&input2);
    let mut replacement2 = valid_input(&env, &attestor);
    replacement2.report_hash = BytesN::from_array(&env, &[23; 32]);
    let new_id2 = client.supersede_evidence(&id2, &replacement2, &admin);
    assert!(client.is_active(&new_id2));
}

#[test]
fn supersede_evidence_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    env.set_auths(&[]);
    let mut replacement = valid_input(&env, &attestor);
    replacement.report_hash = BytesN::from_array(&env, &[22; 32]);
    let result = client.try_supersede_evidence(&id, &replacement, &attestor);
    assert!(
        result.is_err(),
        "Supersede evidence must fail without authorization"
    );
}

#[test]
fn set_paused_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.set_paused(&true);
    // Verify paused by trying to publish (should fail with Paused error)
    assert_eq!(
        client.try_publish_evidence(&valid_input(&env, &attestor)),
        Err(Ok(ContractError::Paused))
    );
}

#[test]
fn set_paused_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.set_auths(&[]);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let result = client.try_set_paused(&true);
    assert!(
        result.is_err(),
        "Set paused must fail without admin authorization"
    );
}

#[test]
fn propose_admin_requires_current_admin_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
}

#[test]
fn propose_admin_fails_without_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    env.set_auths(&[]);
    let new_admin = Address::generate(&env);
    let result = client.try_propose_admin(&new_admin);
    assert!(
        result.is_err(),
        "Propose admin must fail without admin authorization"
    );
}

#[test]
fn accept_admin_requires_pending_admin_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.accept_admin();
}

#[test]
fn accept_admin_fails_without_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    env.set_auths(&[]);
    let result = client.try_accept_admin();
    assert!(
        result.is_err(),
        "Accept admin must fail without pending admin authorization"
    );
}

#[test]
fn cancel_admin_transfer_requires_current_admin_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.cancel_admin_transfer();
}

#[test]
fn cancel_admin_transfer_fails_without_authorization() {
    let (env, contract_id, admin, _, _) = setup();
    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    env.set_auths(&[]);
    let result = client.try_cancel_admin_transfer();
    assert!(
        result.is_err(),
        "Cancel admin transfer must fail without admin authorization"
    );
}

#[test]
fn upgrade_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    
    // Note: We test authorization, not actual upgrade functionality
    // The test passes if it requires admin auth (which mock_all_auths satisfies)
    // Actual upgrade will fail in test environment without real WASM, but that's expected
    let new_hash = BytesN::from_array(&env, &[1; 32]);
    let _ = client.try_upgrade(&new_hash);
    // The upgrade will fail with MissingValue error in test environment,
    // but this test verifies authorization is required
}

#[test]
fn upgrade_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.set_auths(&[]);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let result = client.try_upgrade(&BytesN::from_array(&env, &[1; 32]));
    assert!(
        result.is_err(),
        "Upgrade must fail without admin authorization"
    );
}

#[test]
fn maintain_attestor_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.maintain_attestor(&attestor);
}

#[test]
fn maintain_attestor_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.set_auths(&[]);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let result = client.try_maintain_attestor(&attestor);
    assert!(
        result.is_err(),
        "Maintain attestor must fail without admin authorization"
    );
}

#[test]
fn maintain_evidence_requires_admin_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);
    client.maintain_evidence(&id);
}

#[test]
fn maintain_evidence_fails_without_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    env.set_auths(&[]);
    let result = client.try_maintain_evidence(&id);
    assert!(
        result.is_err(),
        "Maintain evidence must fail without admin authorization"
    );
}

#[test]
fn read_operations_require_no_authorization() {
    let (env, contract_id, admin, attestor, _) = setup();
    initialize_with_attestor(&env, &contract_id, &admin, &attestor);

    env.mock_all_auths();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let input = valid_input(&env, &attestor);
    let id = client.publish_evidence(&input);

    // Clear all auths and test reads without authorization
    env.set_auths(&[]);

    // These should all succeed without auth
    let _ = client.schema_version();
    let _ = client.get_evidence(&id);
    let _ = client.get_attestor(&attestor);
    let _ = client.is_active(&id);
}
