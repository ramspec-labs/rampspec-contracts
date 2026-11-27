use ed25519_dalek::{Signer as _, SigningKey};
use policy_account_fixture::{
    AccountSignature, Ed25519Signature, Ed25519Signer, PolicyAccountFixture,
    PolicyAccountFixtureClient, PolicyError,
};
use proptest::prelude::*;
use soroban_sdk::{
    Address, BytesN, Env, IntoVal, Symbol, Vec,
    auth::{Context, ContractContext},
    testutils::Address as _,
    vec,
};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn signer(env: &Env, key: &SigningKey, weight: u32) -> Ed25519Signer {
    Ed25519Signer {
        public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
        weight,
    }
}

fn signature(env: &Env, key: &SigningKey, payload: &[u8; 32]) -> AccountSignature {
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

fn check(
    env: &Env,
    contract_id: &Address,
    payload: &[u8; 32],
    signatures: &Vec<AccountSignature>,
) -> Result<(), Result<PolicyError, soroban_sdk::InvokeError>> {
    let contexts = contract_contexts(env, contract_id, "invoke");
    env.try_invoke_contract_check_auth::<PolicyError>(
        contract_id,
        &BytesN::from_array(env, payload),
        signatures.clone().into_val(env),
        &contexts,
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn successful_payload_creates_replay_marker_blocking_reuse(
        payload_a in any::<[u8; 32]>(),
        payload_b in any::<[u8; 32]>(),
    ) {
        prop_assume!(payload_a != payload_b);

        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

        let sig_a = vec![&env, signature(&env, &first, &payload_a)];
        let sig_b = vec![&env, signature(&env, &first, &payload_b)];

        prop_assert_eq!(check(&env, &contract_id, &payload_a, &sig_a), Ok(()));
        prop_assert_eq!(
            check(&env, &contract_id, &payload_a, &sig_a),
            Err(Ok(PolicyError::Replay))
        );
        prop_assert_eq!(check(&env, &contract_id, &payload_b, &sig_b), Ok(()));
        prop_assert_eq!(
            check(&env, &contract_id, &payload_b, &sig_b),
            Err(Ok(PolicyError::Replay))
        );
    }

    #[test]
    fn independent_payloads_remain_usable_after_one_is_consumed(
        payload_a in any::<[u8; 32]>(),
        payload_b in any::<[u8; 32]>(),
    ) {
        prop_assume!(payload_a != payload_b);

        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

        let sig_a = vec![&env, signature(&env, &first, &payload_a)];
        let sig_b = vec![&env, signature(&env, &first, &payload_b)];

        prop_assert_eq!(check(&env, &contract_id, &payload_a, &sig_a), Ok(()));
        prop_assert_eq!(check(&env, &contract_id, &payload_b, &sig_b), Ok(()));
        prop_assert_eq!(
            check(&env, &contract_id, &payload_a, &sig_a),
            Err(Ok(PolicyError::Replay))
        );
    }

    #[test]
    fn failed_crypto_verification_does_not_create_replay_marker(
        payload in any::<[u8; 32]>(),
        wrong_payload in any::<[u8; 32]>(),
    ) {
        prop_assume!(payload != wrong_payload);

        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

        let wrong_sig = vec![&env, signature(&env, &first, &wrong_payload)];
        let result = check(&env, &contract_id, &payload, &wrong_sig);
        prop_assert!(result.is_err());
        if let Err(Ok(error)) = result {
            prop_assert_ne!(error, PolicyError::Replay);
        }

        let correct_sig = vec![&env, signature(&env, &first, &payload)];
        prop_assert_eq!(check(&env, &contract_id, &payload, &correct_sig), Ok(()));
    }

    #[test]
    fn unauthorized_signer_does_not_create_replay_marker(
        payload in any::<[u8; 32]>(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let unknown = key(99);
        let signers = vec![&env, signer(&env, &first, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

        let unknown_sig = vec![&env, signature(&env, &unknown, &payload)];
        prop_assert_eq!(
            check(&env, &contract_id, &payload, &unknown_sig),
            Err(Ok(PolicyError::UnauthorizedSigner))
        );

        let authorized_sig = vec![&env, signature(&env, &first, &payload)];
        prop_assert_eq!(check(&env, &contract_id, &payload, &authorized_sig), Ok(()));
    }

    #[test]
    fn insufficient_threshold_does_not_create_replay_marker(
        payload in any::<[u8; 32]>(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let second = key(2);
        let signers = vec![&env, signer(&env, &first, 1), signer(&env, &second, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &2);

        let insufficient_sig = vec![&env, signature(&env, &first, &payload)];
        prop_assert_eq!(
            check(&env, &contract_id, &payload, &insufficient_sig),
            Err(Ok(PolicyError::InsufficientWeight))
        );

        let sufficient_sig = vec![
            &env,
            signature(&env, &first, &payload),
            signature(&env, &second, &payload),
        ];
        prop_assert_eq!(check(&env, &contract_id, &payload, &sufficient_sig), Ok(()));
    }

    #[test]
    fn duplicate_signer_does_not_create_replay_marker(
        payload in any::<[u8; 32]>(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

        let sig = signature(&env, &first, &payload);
        let duplicate = vec![&env, sig.clone(), sig];
        prop_assert_eq!(
            check(&env, &contract_id, &payload, &duplicate),
            Err(Ok(PolicyError::DuplicateSigner))
        );

        let valid_sig = vec![&env, signature(&env, &first, &payload)];
        prop_assert_eq!(check(&env, &contract_id, &payload, &valid_sig), Ok(()));
    }
}
