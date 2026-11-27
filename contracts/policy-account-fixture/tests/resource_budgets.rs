use policy_account_fixture::{
    AccountSignature, Ed25519Signature, Ed25519Signer, MAX_SIGNERS, PasskeySigner,
    PolicyAccountFixture, PolicyAccountFixtureClient, PolicyControls, PolicyError,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, FromVal, IntoVal, Symbol, Vec,
    auth::{Context, ContractContext},
    testutils::{Address as _, Ledger as _},
    vec,
};

const CPU_BUDGET_TOLERANCE: u64 = 5_000_000;
const MEMORY_BUDGET_TOLERANCE: u64 = 4_000_000;

#[test]
fn initialize_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

    env.cost_estimate().budget().reset_tracker();
    PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(cpu <= 15_000_000, "initialize CPU: {} exceeds 15M", cpu);
    assert!(mem <= 12_000_000, "initialize memory: {} exceeds 12M", mem);
}

#[test]
fn set_signers_normal_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key1 = ed25519_key(1);
    let key2 = ed25519_key(2);
    let key3 = ed25519_key(3);
    let initial_signers = vec![&env, ed25519_signer(&env, &key1, 1)];
    let new_signers = vec![
        &env,
        ed25519_signer(&env, &key1, 2),
        ed25519_signer(&env, &key2, 1),
        ed25519_signer(&env, &key3, 1),
    ];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &initial_signers, &1);

    env.cost_estimate().budget().reset_tracker();
    client.set_signers(&new_signers, &3);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(cpu <= 20_000_000, "set_signers CPU: {} exceeds 20M", cpu);
    assert!(mem <= 16_000_000, "set_signers memory: {} exceeds 16M", mem);
}

#[test]
fn set_signers_maximum_count_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let initial_key = ed25519_key(1);
    let initial_signers = vec![&env, ed25519_signer(&env, &initial_key, 1)];

    let mut max_signers = Vec::new(&env);
    for seed in 1..=MAX_SIGNERS {
        let key = ed25519_key(u8::try_from(seed).unwrap());
        max_signers.push_back(ed25519_signer(&env, &key, 1));
    }

    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &initial_signers, &1);

    env.cost_estimate().budget().reset_tracker();
    client.set_signers(&max_signers, &MAX_SIGNERS);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 20_000_000 + CPU_BUDGET_TOLERANCE,
        "set_signers max CPU: {} exceeds limit",
        cpu
    );
    assert!(
        mem <= 16_000_000 + MEMORY_BUDGET_TOLERANCE,
        "set_signers max memory: {} exceeds limit",
        mem
    );
}

#[test]
fn set_passkeys_normal_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let bootstrap_key = ed25519_key(1);
    let bootstrap_signers = vec![&env, ed25519_signer(&env, &bootstrap_key, 1)];

    let passkey1 = passkey_key(2);
    let passkey2 = passkey_key(3);
    let passkey_signers = vec![
        &env,
        passkey_signer(&env, &passkey1, 2),
        passkey_signer(&env, &passkey2, 1),
    ];

    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &bootstrap_signers, &1);

    env.cost_estimate().budget().reset_tracker();
    client.set_passkeys(&passkey_signers, &2);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(cpu <= 20_000_000, "set_passkeys CPU: {} exceeds 20M", cpu);
    assert!(
        mem <= 16_000_000,
        "set_passkeys memory: {} exceeds 16M",
        mem
    );
}

#[test]
fn set_passkeys_maximum_count_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let bootstrap_key = ed25519_key(1);
    let bootstrap_signers = vec![&env, ed25519_signer(&env, &bootstrap_key, 1)];

    let mut max_passkeys = Vec::new(&env);
    for seed in 1..=MAX_SIGNERS {
        let key = passkey_key(u8::try_from(seed).unwrap());
        max_passkeys.push_back(passkey_signer(&env, &key, 1));
    }

    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &bootstrap_signers, &1);

    env.cost_estimate().budget().reset_tracker();
    client.set_passkeys(&max_passkeys, &MAX_SIGNERS);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 20_000_000 + CPU_BUDGET_TOLERANCE,
        "set_passkeys max CPU: {} exceeds limit",
        cpu
    );
    assert!(
        mem <= 16_000_000 + MEMORY_BUDGET_TOLERANCE,
        "set_passkeys max memory: {} exceeds limit",
        mem
    );
}

#[test]
fn set_controls_within_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);

    let mut controls = default_controls();
    controls.valid_from_ledger = 100;
    controls.expires_at_ledger = 10_000;
    controls.reject_all = false;
    controls.additional_signer = Some(BytesN::from_array(&env, &[1; 32]));
    controls.required_contract = Some(Address::generate(&env));
    controls.required_function = Some(Symbol::new(&env, "test_function"));

    env.cost_estimate().budget().reset_tracker();
    client.set_controls(&controls);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(cpu <= 15_000_000, "set_controls CPU: {} exceeds 15M", cpu);
    assert!(
        mem <= 12_000_000,
        "set_controls memory: {} exceeds 12M",
        mem
    );
}

#[test]
fn invalid_signer_configuration_fails_efficiently() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let valid_signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &valid_signers, &1);

    // Zero weight signer
    let invalid_signers = vec![&env, ed25519_signer(&env, &key, 0)];
    env.cost_estimate().budget().reset_tracker();
    let result = client.try_set_signers(&invalid_signers, &1);
    assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 10_000_000,
        "Invalid signer rejection CPU: {} should be efficient",
        cpu
    );
    assert!(
        mem <= 8_000_000,
        "Invalid signer rejection memory: {} should be efficient",
        mem
    );
}

#[test]
fn duplicate_signer_rejection_is_efficient() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let valid_signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &valid_signers, &1);

    // Duplicate signers
    let duplicate_signers = vec![
        &env,
        ed25519_signer(&env, &key, 1),
        ed25519_signer(&env, &key, 1),
    ];

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_set_signers(&duplicate_signers, &2);
    assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 15_000_000,
        "Duplicate rejection CPU: {} should be bounded",
        cpu
    );
    assert!(
        mem <= 12_000_000,
        "Duplicate rejection memory: {} should be bounded",
        mem
    );
}

#[test]
fn invalid_passkey_configuration_fails_efficiently() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let bootstrap_key = ed25519_key(1);
    let bootstrap_signers = vec![&env, ed25519_signer(&env, &bootstrap_key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &bootstrap_signers, &1);

    // Invalid off-curve public key
    let invalid_passkey = vec![
        &env,
        PasskeySigner {
            public_key: BytesN::from_array(&env, &[0; 65]),
            weight: 1,
        },
    ];

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_set_passkeys(&invalid_passkey, &1);
    assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 15_000_000,
        "Invalid passkey rejection CPU: {} should be efficient",
        cpu
    );
    assert!(
        mem <= 12_000_000,
        "Invalid passkey rejection memory: {} should be efficient",
        mem
    );
}

#[test]
fn oversized_signer_list_rejection_is_efficient() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let valid_signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &valid_signers, &1);

    // Try to exceed MAX_SIGNERS
    let mut oversized_signers = Vec::new(&env);
    for seed in 1..=(MAX_SIGNERS + 1) {
        let k = ed25519_key(u8::try_from(seed % 256).unwrap());
        oversized_signers.push_back(ed25519_signer(&env, &k, 1));
    }

    env.cost_estimate().budget().reset_tracker();
    let result = client.try_set_signers(&oversized_signers, &1);
    assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    assert!(
        cpu <= 20_000_000,
        "Oversized list rejection CPU: {} should be bounded",
        cpu
    );
    assert!(
        mem <= 16_000_000,
        "Oversized list rejection memory: {} should be bounded",
        mem
    );
}

#[test]
fn idempotent_update_consumes_minimal_resources() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);

    env.cost_estimate().budget().reset_tracker();
    client.set_signers(&signers, &1);

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    // Idempotent updates should not write, publish events, or hash unnecessarily
    assert!(
        cpu <= 10_000_000,
        "Idempotent update CPU: {} should be minimal",
        cpu
    );
    assert!(
        mem <= 8_000_000,
        "Idempotent update memory: {} should be minimal",
        mem
    );
}

#[test]
fn unauthorized_update_consumes_minimal_resources() {
    let env = Env::default();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];

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

    env.cost_estimate().budget().reset_tracker();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.set_signers(&vec![&env, ed25519_signer(&env, &key, 2)], &1);
    }));

    assert!(result.is_err());

    let cpu = env.cost_estimate().budget().cpu_instruction_cost();
    let mem = env.cost_estimate().budget().memory_bytes_cost();

    // Unauthorized calls should fail before expensive operations
    assert!(
        cpu <= 10_000_000,
        "Unauthorized update CPU: {} should be minimal",
        cpu
    );
    assert!(
        mem <= 8_000_000,
        "Unauthorized update memory: {} should be minimal",
        mem
    );
}

#[test]
fn failed_operation_does_not_persist_state() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let key = ed25519_key(1);
    let signers = vec![&env, ed25519_signer(&env, &key, 1)];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);

    client.initialize(&admin, &signers, &1);
    let initial_config = client.config();
    let initial_version = client.config_version();

    // Attempt invalid update
    let _ = client.try_set_signers(&vec![&env, ed25519_signer(&env, &key, 0)], &1);

    // State should remain unchanged
    assert_eq!(client.config(), initial_config);
    assert_eq!(client.config_version(), initial_version);
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
