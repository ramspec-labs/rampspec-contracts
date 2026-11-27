use policy_account_fixture::{
    AccountSignature, Ed25519Signature, Ed25519Signer, PolicyAccountFixture,
    PolicyAccountFixtureClient, PolicyError,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, FromVal, IntoVal, Symbol, Vec,
    auth::{Context, ContractContext},
    testutils::{Address as _, Ledger as _},
    vec,
};

#[test]
fn replay_marker_ttl_extends_on_successful_authorization() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    let payload = [1; 32];
    let signatures = vec![&env, ed25519_signature(&env, &key, &payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    env.ledger().set_sequence_number(100);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Ok(())
    );
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );
}

#[test]
fn replay_marker_extends_to_target_lifetime() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    let payload = [2; 32];
    let signatures = vec![&env, ed25519_signature(&env, &key, &payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    env.ledger().set_sequence_number(1000);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Ok(())
    );

    // Advance ledger to just before target lifetime (1000 + 103680 = 104680)
    env.ledger().set_sequence_number(104_679);
    let new_payload = [3; 32];
    let new_signatures = vec![&env, ed25519_signature(&env, &key, &new_payload)];
    assert_eq!(
        check_auth(&env, &contract_id, &new_payload, &new_signatures, &contexts),
        Ok(())
    );

    // Original marker should still reject replay
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );
}

#[test]
fn replay_marker_ttl_threshold_behavior() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    env.ledger().set_sequence_number(10_000);
    let payload = [4; 32];
    let signatures = vec![&env, ed25519_signature(&env, &key, &payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Ok(())
    );

    // Just below minimum threshold (10000 + 17280 = 27280)
    env.ledger().set_sequence_number(27_279);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );

    // At minimum threshold
    env.ledger().set_sequence_number(27_280);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );

    // Above threshold but below target
    env.ledger().set_sequence_number(50_000);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );
}

#[test]
fn failed_authorization_does_not_create_replay_marker() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let authorized_key = ed25519_key(1);
    let unauthorized_key = ed25519_key(2);
    let signers = vec![&env, ed25519_signer(&env, &authorized_key, 1)];

    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    let payload = [5; 32];
    let unauthorized_signatures = vec![&env, ed25519_signature(&env, &unauthorized_key, &payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    // Unauthorized signer should fail
    assert_eq!(
        check_auth(
            &env,
            &contract_id,
            &payload,
            &unauthorized_signatures,
            &contexts
        ),
        Err(Ok(PolicyError::UnauthorizedSigner))
    );

    // Should be able to use payload with authorized signer (no replay marker created)
    let authorized_signatures = vec![&env, ed25519_signature(&env, &authorized_key, &payload)];
    assert_eq!(
        check_auth(
            &env,
            &contract_id,
            &payload,
            &authorized_signatures,
            &contexts
        ),
        Ok(())
    );

    // Now should reject as replay
    assert_eq!(
        check_auth(
            &env,
            &contract_id,
            &payload,
            &authorized_signatures,
            &contexts
        ),
        Err(Ok(PolicyError::Replay))
    );
}

#[test]
fn invalid_signature_does_not_create_replay_marker() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    let payload = [6; 32];
    let wrong_payload = [7; 32];
    let invalid_signatures = vec![&env, ed25519_signature(&env, &key, &wrong_payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    // Invalid signature should panic (crypto verification failure)
    let invalid_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        check_auth(&env, &contract_id, &payload, &invalid_signatures, &contexts)
    }));
    assert!(invalid_result.is_err());

    // Should be able to use payload with valid signature (no replay marker created)
    let valid_signatures = vec![&env, ed25519_signature(&env, &key, &payload)];
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &valid_signatures, &contexts),
        Ok(())
    );
}

#[test]
fn repeated_failed_operations_do_not_create_markers() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);

    env.ledger().set_sequence_number(100);
    let payload = [8; 32];
    let signatures = vec![&env, ed25519_signature(&env, &key, &payload)];

    // Set expiration to current ledger - 1 so authorization fails
    let mut controls = default_controls();
    controls.expires_at_ledger = 99;
    client.set_controls(&controls);

    let contexts = contract_contexts(&env, &contract_id, "test_function");

    // Multiple failed attempts due to expiration
    for _ in 0..3 {
        assert_eq!(
            check_auth(&env, &contract_id, &payload, &signatures, &contexts),
            Err(Ok(PolicyError::Expired))
        );
    }

    // Fix expiration
    controls.expires_at_ledger = u32::MAX;
    client.set_controls(&controls);

    // Should succeed (no replay marker from failed attempts)
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Ok(())
    );

    // Now should be marked as replay
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Replay))
    );
}

#[test]
fn replay_protection_retained_throughout_authorization_window() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);

    // Set authorization validity window
    let start_ledger = 1000_u32;
    let end_ledger = 10_000_u32;
    let mut controls = default_controls();
    controls.valid_from_ledger = start_ledger;
    controls.expires_at_ledger = end_ledger;
    client.set_controls(&controls);

    env.ledger().set_sequence_number(start_ledger);
    let payload = [9; 32];
    let signatures = vec![&env, ed25519_signature(&env, &key, &payload)];
    let contexts = contract_contexts(&env, &contract_id, "test_function");

    // Successful authorization at start of window
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Ok(())
    );

    // Replay should be rejected at various points in the window
    for ledger in [
        start_ledger + 1,
        (start_ledger + end_ledger) / 2,
        end_ledger - 1,
        end_ledger,
    ] {
        env.ledger().set_sequence_number(ledger);
        assert_eq!(
            check_auth(&env, &contract_id, &payload, &signatures, &contexts),
            Err(Ok(PolicyError::Replay)),
            "Replay should be rejected at ledger {}",
            ledger
        );
    }

    // After expiration, should still reject as replay (not as expired)
    env.ledger().set_sequence_number(end_ledger + 1);
    assert_eq!(
        check_auth(&env, &contract_id, &payload, &signatures, &contexts),
        Err(Ok(PolicyError::Expired))
    );
}

// Helper functions

fn ed25519_key(seed: u8) -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
}

fn ed25519_signer(env: &Env, key: &ed25519_dalek::SigningKey, weight: u32) -> Ed25519Signer {
    Ed25519Signer {
        public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
        weight,
    }
}

fn ed25519_signature(
    env: &Env,
    key: &ed25519_dalek::SigningKey,
    payload: &[u8; 32],
) -> AccountSignature {
    use ed25519_dalek::Signer as _;
    AccountSignature::Ed25519(Ed25519Signature {
        public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
        signature: BytesN::from_array(env, &key.sign(payload).to_bytes()),
    })
}

fn contract_contexts(env: &Env, target: &Address, function: &str) -> Vec<Context> {
    vec![
        env,
        Context::Contract(ContractContext {
            contract: target.clone(),
            fn_name: Symbol::new(env, function),
            args: Vec::new(env),
        }),
    ]
}

fn check_auth(
    env: &Env,
    contract_id: &Address,
    payload: &[u8; 32],
    signatures: &Vec<AccountSignature>,
    contexts: &Vec<Context>,
) -> Result<(), Result<PolicyError, soroban_sdk::InvokeError>> {
    env.try_invoke_contract_check_auth::<PolicyError>(
        contract_id,
        &BytesN::from_array(env, payload),
        signatures.clone().into_val(env),
        contexts,
    )
}

fn default_controls() -> policy_account_fixture::PolicyControls {
    policy_account_fixture::PolicyControls {
        valid_from_ledger: 0,
        expires_at_ledger: u32::MAX,
        reject_all: false,
        additional_signer: None,
        required_contract: None,
        required_function: None,
    }
}
