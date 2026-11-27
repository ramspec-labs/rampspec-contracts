//! Configuration versions are a `u32` that every effective change advances by one. These tests pin
//! the bound: the last version is reachable, and a change past it fails with a deterministic error
//! that leaves configuration, commitment, version, events, replay state and TTLs exactly as they
//! were.

use ed25519_dalek::SigningKey;
use p256::ecdsa::SigningKey as P256SigningKey;
use policy_account_fixture::{
    Ed25519Signer, PasskeySigner, PolicyAccountFixture, PolicyAccountFixtureClient, PolicyConfig,
    PolicyControls, PolicyError,
};
use proptest::prelude::*;
use soroban_sdk::{
    Address, Bytes, BytesN, Env, Vec, contracttype,
    testutils::{
        Address as _, Events as _,
        storage::{Instance as _, Temporary as _},
    },
    vec,
    xdr::ToXdr,
};

/// The fixture's private storage keys, declared the same way so the tests reach the same
/// entries. `mirrored_keys_reach_the_contract_storage` proves the encodings agree.
#[contracttype]
enum DataKey {
    Config,
    ConfigVersion,
    UsedPayload(BytesN<32>),
}

const MARKER: [u8; 32] = [9; 32];

fn ed25519_signer(env: &Env, seed: u8, weight: u32) -> Ed25519Signer {
    let key = SigningKey::from_bytes(&[seed; 32]);
    Ed25519Signer {
        public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
        weight,
    }
}

fn passkey_signer(env: &Env, seed: u8) -> PasskeySigner {
    let key = P256SigningKey::from_bytes((&[seed; 32]).into()).unwrap();
    let encoded = key.verifying_key().to_encoded_point(false);
    let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();
    PasskeySigner {
        public_key: BytesN::from_array(env, &public_key),
        weight: 1,
    }
}

/// An initialized fixture with a replay marker left by an earlier authorization.
fn setup(env: &Env) -> (Address, PolicyAccountFixtureClient<'_>) {
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let client = PolicyAccountFixtureClient::new(env, &contract_id);
    client.initialize(
        &Address::generate(env),
        &vec![env, ed25519_signer(env, 1, 1)],
        &1,
    );
    env.as_contract(&contract_id, || {
        env.storage().temporary().set(
            &DataKey::UsedPayload(BytesN::from_array(env, &MARKER)),
            &true,
        );
    });
    (contract_id, client)
}

fn set_version(env: &Env, contract_id: &Address, version: u32) {
    env.as_contract(contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::ConfigVersion, &version);
    });
}

/// Everything a configuration change could touch, in a comparable form.
#[derive(Debug, PartialEq)]
struct State {
    instance: Bytes,
    temporary: Bytes,
    instance_ttl: u32,
    marker_ttl: u32,
    marker: Option<bool>,
    config: PolicyConfig,
    commitment: BytesN<32>,
    version: u32,
}

fn state(env: &Env, contract_id: &Address, client: &PolicyAccountFixtureClient<'_>) -> State {
    let config = client.config();
    let marker_key = DataKey::UsedPayload(BytesN::from_array(env, &MARKER));
    let (instance, temporary, instance_ttl, marker_ttl, marker) =
        env.as_contract(contract_id, || {
            let storage = env.storage();
            (
                storage.instance().all().to_xdr(env),
                storage.temporary().all().to_xdr(env),
                storage.instance().get_ttl(),
                storage.temporary().get_ttl(&marker_key),
                storage.temporary().get::<_, bool>(&marker_key),
            )
        });
    State {
        instance,
        temporary,
        instance_ttl,
        marker_ttl,
        marker,
        commitment: env.crypto().sha256(&config.clone().to_xdr(env)).to_bytes(),
        config,
        version: client.config_version(),
    }
}

/// Every entrypoint that writes configuration, each asked for an effective change. Each result
/// comes with the number of events that call published, read before anything else runs.
fn attempt_every_change(
    env: &Env,
    client: &PolicyAccountFixtureClient<'_>,
) -> [(Result<(), PolicyError>, usize); 3] {
    let flatten = |r: Result<Result<(), _>, Result<PolicyError, _>>| match r {
        Ok(Ok(())) => Ok(()),
        Err(Ok(error)) => Err(error),
        other => panic!("unexpected host failure: {other:?}"),
    };
    let events = || env.events().all().events().len();
    let mut controls: PolicyControls = client.config().controls;
    controls.reject_all = !controls.reject_all;
    let signers = flatten(client.try_set_signers(&vec![env, ed25519_signer(env, 2, 1)], &1));
    let after_signers = events();
    let passkeys = flatten(client.try_set_passkeys(&vec![env, passkey_signer(env, 3)], &1));
    let after_passkeys = events();
    let controls = flatten(client.try_set_controls(&controls));
    let after_controls = events();
    [
        (signers, after_signers),
        (passkeys, after_passkeys),
        (controls, after_controls),
    ]
}

#[test]
fn mirrored_keys_reach_the_contract_storage() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    let (config, version) = env.as_contract(&contract_id, || {
        (
            env.storage()
                .instance()
                .get::<_, PolicyConfig>(&DataKey::Config),
            env.storage()
                .instance()
                .get::<_, u32>(&DataKey::ConfigVersion),
        )
    });
    assert_eq!(config, Some(client.config()));
    assert_eq!(version, Some(client.config_version()));
}

#[test]
fn the_last_version_is_reachable_and_announced() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    set_version(&env, &contract_id, u32::MAX - 1);

    client.set_signers(&vec![&env, ed25519_signer(&env, 2, 1)], &1);

    assert_eq!(env.events().all().events().len(), 1);
    assert_eq!(client.config_version(), u32::MAX);
}

#[test]
fn overflow_is_a_deterministic_error_that_changes_nothing() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    set_version(&env, &contract_id, u32::MAX);
    let before = state(&env, &contract_id, &client);

    // Every entrypoint that would advance the version fails the same way, every time.
    for _ in 0..2 {
        for (result, events) in attempt_every_change(&env, &client) {
            assert_eq!(result, Err(PolicyError::InvalidPolicy));
            assert_eq!(events, 0);
        }
        assert_eq!(state(&env, &contract_id, &client), before);
    }
    assert_eq!(before.marker, Some(true));
    assert_eq!(before.version, u32::MAX);
}

#[test]
fn a_no_op_change_at_the_last_version_still_succeeds_without_a_new_version() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    set_version(&env, &contract_id, u32::MAX);
    let before = state(&env, &contract_id, &client);

    client.set_signers(&before.config.signers, &before.config.threshold);
    client.set_controls(&before.config.controls);

    assert_eq!(env.events().all().events().len(), 0);
    assert_eq!(state(&env, &contract_id, &client), before);
}

#[test]
fn an_unauthorized_change_at_the_last_version_fails_on_authorization_and_changes_nothing() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    set_version(&env, &contract_id, u32::MAX);
    let before = state(&env, &contract_id, &client);
    env.set_auths(&[]);

    let result = client.try_set_signers(&vec![&env, ed25519_signer(&env, 2, 1)], &1);

    // The admin's authorization is checked before the version: a stranger learns nothing from
    // an overflow error, and the attempt writes nothing either.
    assert!(matches!(result, Err(Err(_))), "{result:?}");
    env.mock_all_auths();
    assert_eq!(state(&env, &contract_id, &client), before);
}

#[test]
fn invalid_input_at_the_last_version_reports_the_input_and_changes_nothing() {
    let env = Env::default();
    let (contract_id, client) = setup(&env);
    set_version(&env, &contract_id, u32::MAX);
    let before = state(&env, &contract_id, &client);

    assert_eq!(
        client.try_set_signers(&Vec::new(&env), &1),
        Err(Ok(PolicyError::InvalidPolicy))
    );
    let mut controls = before.config.controls.clone();
    controls.valid_from_ledger = 10;
    controls.expires_at_ledger = 5;
    assert_eq!(
        client.try_set_controls(&controls),
        Err(Ok(PolicyError::InvalidTimeWindow))
    );
    assert_eq!(env.events().all().events().len(), 0);
    assert_eq!(state(&env, &contract_id, &client), before);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// From any version near the bound, each change advances by exactly one until the bound,
    /// and every change past it is refused without touching state.
    #[test]
    fn versions_advance_by_one_and_stop_at_the_bound(gap in 0_u32..4, changes in 1_usize..7) {
        let env = Env::default();
        let (contract_id, client) = setup(&env);
        let start = u32::MAX - gap;
        set_version(&env, &contract_id, start);

        for _ in 0..changes {
            let before = state(&env, &contract_id, &client);
            // Always ask for a signer set other than the current one, so every call is an
            // effective change and none is skipped as a no-op.
            let first = ed25519_signer(&env, 1, 1);
            let seed = if before.config.signers.get(0) == Some(first) { 2 } else { 1 };
            let result = client.try_set_signers(&vec![&env, ed25519_signer(&env, seed, 1)], &1);
            let events = env.events().all().events().len();
            if before.version < u32::MAX {
                prop_assert_eq!(result, Ok(Ok(())));
                prop_assert_eq!(events, 1);
                prop_assert_eq!(client.config_version(), before.version + 1);
            } else {
                prop_assert_eq!(result, Err(Ok(PolicyError::InvalidPolicy)));
                prop_assert_eq!(events, 0);
                prop_assert_eq!(state(&env, &contract_id, &client), before);
            }
        }
    }
}
