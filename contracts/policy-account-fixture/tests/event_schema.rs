use policy_account_fixture::{
    AccountSignature, Ed25519Signature, Ed25519Signer, PasskeySigner, PolicyAccountFixture,
    PolicyAccountFixtureClient, PolicyConfigurationChanged, PolicyControls, PolicyError,
    SignatureMode,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, FromVal, IntoVal, Map, Symbol, TryFromVal, Val, Vec,
    auth::{Context, ContractContext},
    events::Event,
    testutils::{Address as _, Events as _},
    vec,
};

#[test]
fn policy_configuration_event_schema_is_stable_and_privacy_safe() {
    let env = Env::default();
    let event = PolicyConfigurationChanged {
        change: Symbol::new(&env, "set_signers"),
        version: 2,
        policy_commitment: BytesN::from_array(&env, &[7; 32]),
        signature_mode: SignatureMode::Ed25519,
        signer_count: 3,
        threshold: 2,
    };
    let topics = event.topics(&env);
    assert_eq!(topics.len(), 2);
    assert_eq!(
        Symbol::from_val(&env, &topics.get(0).unwrap()),
        Symbol::new(&env, "policy_configuration_changed")
    );
    assert_eq!(
        Symbol::from_val(&env, &topics.get(1).unwrap()),
        Symbol::new(&env, "set_signers")
    );
    let data = Map::<Symbol, Val>::try_from_val(&env, &event.data(&env)).unwrap();
    assert_eq!(data.len(), 5);
    for field in [
        "version",
        "policy_commitment",
        "signature_mode",
        "signer_count",
        "threshold",
    ] {
        assert!(data.contains_key(Symbol::new(&env, field)));
    }
    assert!(!data.contains_key(Symbol::new(&env, "signers")));
    assert!(!data.contains_key(Symbol::new(&env, "signature")));
}

#[test]
fn events_never_expose_signatures_or_authenticator_data() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);

    let all_events = env.events().all();
    for event in all_events.iter() {
        let event_data_string = format!("{:?}", event);
        // Ensure no signature bytes or authenticator data appears in events
        assert!(
            !event_data_string.contains("signature"),
            "Event should not contain signature data"
        );
        assert!(
            !event_data_string.contains("authenticator_data"),
            "Event should not contain authenticator data"
        );
    }
}

#[test]
fn successful_mutation_publishes_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key1 = ed25519_key(1);
    let key2 = ed25519_key(2);
    let signers = vec![&env, ed25519_signer(&env, &key1, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);
    assert_eq!(env.events().all().events().len(), 1);

    let new_signers = vec![&env, ed25519_signer(&env, &key2, 1)];
    client.set_signers(&new_signers, &1);
    assert_eq!(env.events().all().events().len(), 1);
}

#[test]
fn failed_mutation_publishes_no_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);
    env.events().all().events().clear();

    // Invalid policy with zero weight should fail
    let invalid_signers = vec![&env, ed25519_signer(&env, &key, 0)];
    let result = client.try_set_signers(&invalid_signers, &1);
    assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
fn unauthorized_mutation_publishes_no_event() {
    let env = Env::default();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    // Only mock admin auth for initialization
    env.mock_auths(&[soroban_sdk::testutils::MockAuth {
        address: &admin,
        invoke: &soroban_sdk::testutils::MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: vec![
                &env,
                admin.clone().into_val(&env),
                signers.clone().into_val(&env),
                1_u32.into_val(&env),
            ],
            sub_invokes: &[],
        },
    }]);

    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &signers, &1);
    env.events().all().events().clear();

    // This should fail authorization
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_signers(&vec![&env, ed25519_signer(&env, &key, 2)], &1);
    }));
    assert!(result.is_err());
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
fn idempotent_mutation_publishes_no_event() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);
    env.events().all().events().clear();

    // Idempotent update (same signers and threshold)
    client.set_signers(&signers, &1);
    assert_eq!(env.events().all().events().len(), 0);
}

#[test]
fn event_version_progresses_with_non_idempotent_changes() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key1 = ed25519_key(1);
    let key2 = ed25519_key(2);
    let signers1 = vec![&env, ed25519_signer(&env, &key1, 1)];
    let signers2 = vec![&env, ed25519_signer(&env, &key2, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers1, &1);
    assert_eq!(client.config_version(), 1);

    client.set_signers(&signers2, &1);
    assert_eq!(client.config_version(), 2);

    let mut controls = default_controls();
    controls.reject_all = true;
    client.set_controls(&controls);
    assert_eq!(client.config_version(), 3);
}

#[test]
fn event_commitment_changes_with_policy_mutation() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key1 = ed25519_key(1);
    let key2 = ed25519_key(2);
    let signers1 = vec![&env, ed25519_signer(&env, &key1, 1)];
    let signers2 = vec![&env, ed25519_signer(&env, &key2, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers1, &1);
    let events1 = env.events().all();
    env.events().all().events().clear();

    client.set_signers(&signers2, &1);
    let events2 = env.events().all();

    // Extract policy_commitment from both events
    let commitment1 =
        extract_event_field(&env, &events1.iter().next().unwrap(), "policy_commitment");
    let commitment2 =
        extract_event_field(&env, &events2.iter().next().unwrap(), "policy_commitment");

    assert_ne!(
        commitment1, commitment2,
        "Policy commitment should change when configuration changes"
    );
}

#[test]
fn rejected_inputs_never_appear_in_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let key = ed25519_key(1);
    let valid_signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &valid_signers, &1);

    // Attempt invalid mutations with distinctive values
    let invalid_signers = vec![
        &env,
        Ed25519Signer {
            public_key: BytesN::from_array(&env, &[0xFF; 32]),
            weight: 0, // Invalid weight
        },
    ];

    let _ = client.try_set_signers(&invalid_signers, &1);

    // Check that the distinctive invalid public key doesn't appear in any event
    let all_events = env.events().all();
    for event in all_events.iter() {
        let event_string = format!("{:?}", event);
        assert!(
            !event_string.contains("ffff"),
            "Invalid signer data should not appear in events"
        );
    }
}

#[test]
fn passkey_mutation_events_exclude_public_keys() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);

    let bootstrap_key = ed25519_key(1);
    let bootstrap_signers = vec![&env, ed25519_signer(&env, &bootstrap_key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &bootstrap_signers, &1);
    env.events().all().events().clear();

    let passkey = passkey_key(2);
    let passkey_signers = vec![&env, passkey_signer(&env, &passkey, 1)];
    client.set_passkeys(&passkey_signers, &1);

    // Verify event was published but doesn't contain public key bytes
    assert_eq!(env.events().all().events().len(), 1);
    let event = env.events().all().iter().next().unwrap();
    let data = Map::<Symbol, Val>::try_from_val(&env, &event.data).unwrap();

    assert!(!data.contains_key(Symbol::new(&env, "public_key")));
    assert!(!data.contains_key(Symbol::new(&env, "passkey_signers")));
    assert!(data.contains_key(Symbol::new(&env, "signer_count")));
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

fn passkey_key(seed: u8) -> p256::ecdsa::SigningKey {
    p256::ecdsa::SigningKey::from_bytes((&[seed; 32]).into()).unwrap()
}

fn passkey_signer(env: &Env, key: &p256::ecdsa::SigningKey, weight: u32) -> PasskeySigner {
    let encoded = key.verifying_key().to_encoded_point(false);
    let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();
    PasskeySigner {
        public_key: BytesN::from_array(env, &public_key),
        weight,
    }
}

fn default_controls() -> PolicyControls {
    PolicyControls {
        valid_from_ledger: 0,
        expires_at_ledger: u32::MAX,
        reject_all: false,
        additional_signer: None,
        required_contract: None,
        required_function: None,
    }
}

fn extract_event_field(env: &Env, event: &Event, field_name: &str) -> Val {
    let data = Map::<Symbol, Val>::try_from_val(env, &event.data).unwrap();
    data.get(Symbol::new(env, field_name)).unwrap()
}

#[test]
fn policy_error_discriminants_are_stable_and_exhaustive() {
    assert_eq!(PolicyError::NotInitialized as u32, 1);
    assert_eq!(PolicyError::AlreadyInitialized as u32, 2);
    assert_eq!(PolicyError::InvalidPolicy as u32, 3);
    assert_eq!(PolicyError::MissingSignature as u32, 4);
    assert_eq!(PolicyError::DuplicateSigner as u32, 5);
    assert_eq!(PolicyError::UnauthorizedSigner as u32, 6);
    assert_eq!(PolicyError::InsufficientWeight as u32, 7);
    assert_eq!(PolicyError::InvalidTimeWindow as u32, 8);
    assert_eq!(PolicyError::NotYetValid as u32, 9);
    assert_eq!(PolicyError::Expired as u32, 10);
    assert_eq!(PolicyError::Rejected as u32, 11);
    assert_eq!(PolicyError::Replay as u32, 12);
    assert_eq!(PolicyError::RequiredSignerMissing as u32, 13);
    assert_eq!(PolicyError::InvalidContext as u32, 14);
    assert_eq!(PolicyError::WrongSignatureMode as u32, 15);
    assert_eq!(PolicyError::InvalidPasskey as u32, 16);
    assert_eq!(PolicyError::TooManySignatures as u32, 17);
}

#[test]
fn policy_commitment_vectors_are_deterministic_and_reproducible() {
    use policy_account_fixture::{Ed25519Signer, PasskeySigner, PolicyConfig, PolicyControls};
    use soroban_sdk::{Address, BytesN, Env, String, Vec, xdr::ToXdr};

    let env = Env::default();

    let admin_str = String::from_str(
        &env,
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
    );
    let config_a = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 1,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let encoded_a1 = config_a.clone().to_xdr(&env);
    let encoded_a2 = config_a.clone().to_xdr(&env);
    assert_eq!(
        encoded_a1, encoded_a2,
        "Identical configs must encode identically"
    );

    let commitment_a1 = env.crypto().sha256(&encoded_a1).to_bytes();
    let commitment_a2 = env.crypto().sha256(&encoded_a2).to_bytes();
    assert_eq!(
        commitment_a1, commitment_a2,
        "Identical configs must produce identical commitments"
    );

    let config_b = PolicyConfig {
        threshold: 2,
        ..config_a.clone()
    };
    let commitment_b = env.crypto().sha256(&config_b.to_xdr(&env)).to_bytes();
    assert_ne!(
        commitment_a1, commitment_b,
        "Threshold change must change commitment"
    );

    let config_ordered_1 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            },
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[2; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 2,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let config_ordered_2 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[2; 32]),
                weight: 1,
            },
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 2,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let commitment_ordered_1 = env
        .crypto()
        .sha256(&config_ordered_1.to_xdr(&env))
        .to_bytes();
    let commitment_ordered_2 = env
        .crypto()
        .sha256(&config_ordered_2.to_xdr(&env))
        .to_bytes();
    assert_ne!(
        commitment_ordered_1, commitment_ordered_2,
        "Signer order must affect commitment"
    );

    let config_ed25519 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[3; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 1,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let config_passkey = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: Vec::new(&env),
        passkey_signers: soroban_sdk::vec![
            &env,
            PasskeySigner {
                public_key: BytesN::from_array(&env, &{
                    let mut bytes = [0_u8; 65];
                    bytes[0] = 4;
                    for index in 1..65 {
                        bytes[index] = 3;
                    }
                    bytes
                }),
                weight: 1,
            }
        ],
        threshold: 1,
        signature_mode: SignatureMode::PasskeyCompatible,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let commitment_ed25519 = env.crypto().sha256(&config_ed25519.to_xdr(&env)).to_bytes();
    let commitment_passkey = env.crypto().sha256(&config_passkey.to_xdr(&env)).to_bytes();
    assert_ne!(
        commitment_ed25519, commitment_passkey,
        "Signature mode must affect commitment"
    );
}
