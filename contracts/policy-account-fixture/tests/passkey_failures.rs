//! Passkey verification failures must leave no state behind.
//!
//! A failed P-256 check in `secp256r1_verify` traps in the host instead of returning a contract
//! error, and a check that passes can still be followed by a contract error (insufficient weight).
//! Either way the host rolls the invocation back. These tests pin the observable result: after
//! any failed passkey authorization there is no replay marker, no configuration or version
//! change, no event and no TTL change, and the payload is still accepted once by a later valid
//! authorization.

use ed25519_dalek::SigningKey;
use p256::ecdsa::{Signature, SigningKey as P256SigningKey, signature::hazmat::PrehashSigner};
use policy_account_fixture::{
    AccountSignature, Ed25519Signer, PasskeySignature, PasskeySigner, PolicyAccountFixture,
    PolicyAccountFixtureClient, PolicyConfig, PolicyError,
};
use proptest::prelude::*;
use soroban_sdk::{
    Address, Bytes, BytesN, Env, IntoVal, Symbol, Vec,
    auth::{Context, ContractContext},
    contracttype,
    testutils::{
        Address as _, Events as _,
        storage::{Instance as _, Temporary as _},
    },
    xdr::ToXdr,
};

/// The fixture's private storage keys, declared the same way so the test can see replay markers.
/// `mirrored_keys_reach_the_contract_storage` proves the encodings agree.
#[contracttype]
enum DataKey {
    Config,
    ConfigVersion,
    UsedPayload(BytesN<32>),
}

const KEY_A: u8 = 0x31;
const KEY_B: u8 = 0x32;
const CLIENT_DATA_HASH: [u8; 32] = [0xB6; 32];

fn p256_key(seed: u8) -> P256SigningKey {
    P256SigningKey::from_bytes((&[seed; 32]).into()).unwrap()
}

fn public_key(seed: u8) -> [u8; 65] {
    p256_key(seed)
        .verifying_key()
        .to_encoded_point(false)
        .as_bytes()
        .try_into()
        .unwrap()
}

/// A passkey assertion's parts, kept as plain bytes so tests can corrupt any of them.
#[derive(Clone, Debug)]
struct Assertion {
    public_key: [u8; 65],
    authenticator_data: std::vec::Vec<u8>,
    client_data_hash: [u8; 32],
    challenge: [u8; 32],
    signature: [u8; 64],
}

fn valid(env: &Env, seed: u8, payload: &[u8; 32]) -> Assertion {
    let mut authenticator_data = std::vec![0xA5; 32];
    authenticator_data.extend_from_slice(&[0x05, 0, 0, 0, 1]);
    let mut message = authenticator_data.clone();
    message.extend_from_slice(&CLIENT_DATA_HASH);
    let digest = env
        .crypto()
        .sha256(&Bytes::from_slice(env, &message))
        .to_array();
    let signature: Signature = p256_key(seed).sign_prehash(&digest).unwrap();
    let signature = signature.normalize_s().unwrap_or(signature);
    Assertion {
        public_key: public_key(seed),
        authenticator_data,
        client_data_hash: CLIENT_DATA_HASH,
        challenge: *payload,
        signature: signature.to_bytes().into(),
    }
}

fn envelope(env: &Env, a: &Assertion) -> AccountSignature {
    AccountSignature::Passkey(PasskeySignature {
        public_key: BytesN::from_array(env, &a.public_key),
        authenticator_data: Bytes::from_slice(env, &a.authenticator_data),
        client_data_hash: BytesN::from_array(env, &a.client_data_hash),
        challenge: BytesN::from_array(env, &a.challenge),
        signature: BytesN::from_array(env, &a.signature),
    })
}

/// A fixture whose policy is passkeys A and B, weight 1 each, with the given threshold.
fn setup(env: &Env, threshold: u32) -> (Address, PolicyAccountFixtureClient<'_>) {
    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let client = PolicyAccountFixtureClient::new(env, &contract_id);
    let bootstrap = Ed25519Signer {
        public_key: BytesN::from_array(
            env,
            SigningKey::from_bytes(&[9; 32]).verifying_key().as_bytes(),
        ),
        weight: 1,
    };
    client.initialize(
        &Address::generate(env),
        &Vec::from_array(env, [bootstrap]),
        &1,
    );
    let passkeys = Vec::from_array(
        env,
        [KEY_A, KEY_B].map(|seed| PasskeySigner {
            public_key: BytesN::from_array(env, &public_key(seed)),
            weight: 1,
        }),
    );
    client.set_passkeys(&passkeys, &threshold);
    (contract_id, client)
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Accepted,
    Refused(PolicyError),
    HostTrap,
}

fn authorize(
    env: &Env,
    contract_id: &Address,
    payload: &[u8; 32],
    assertions: &[Assertion],
) -> Outcome {
    let mut signatures = Vec::new(env);
    for a in assertions {
        signatures.push_back(envelope(env, a));
    }
    let contexts = Vec::from_array(
        env,
        [Context::Contract(ContractContext {
            contract: contract_id.clone(),
            fn_name: Symbol::new(env, "is_test_only"),
            args: Vec::new(env),
        })],
    );
    match env.try_invoke_contract_check_auth::<PolicyError>(
        contract_id,
        &BytesN::from_array(env, payload),
        signatures.into_val(env),
        &contexts,
    ) {
        Ok(()) => Outcome::Accepted,
        Err(Ok(error)) => Outcome::Refused(error),
        Err(Err(_)) => Outcome::HostTrap,
    }
}

/// Everything a failed authorization could touch, in a comparable form.
#[derive(Debug, PartialEq)]
struct State {
    instance: Bytes,
    temporary: Bytes,
    instance_ttl: u32,
    config: PolicyConfig,
    version: u32,
}

fn state(env: &Env, contract_id: &Address, client: &PolicyAccountFixtureClient<'_>) -> State {
    let (instance, temporary, instance_ttl) = env.as_contract(contract_id, || {
        let storage = env.storage();
        (
            storage.instance().all().to_xdr(env),
            storage.temporary().all().to_xdr(env),
            storage.instance().get_ttl(),
        )
    });
    State {
        instance,
        temporary,
        instance_ttl,
        config: client.config(),
        version: client.config_version(),
    }
}

fn marker(env: &Env, contract_id: &Address, payload: &[u8; 32]) -> bool {
    env.as_contract(contract_id, || {
        env.storage()
            .temporary()
            .has(&DataKey::UsedPayload(BytesN::from_array(env, payload)))
    })
}

/// Run a failing authorization and check it left nothing behind, then that the payload is still
/// accepted once by a valid assertion and refused as a replay after that.
fn assert_failure_leaves_payload_usable(
    env: &Env,
    contract_id: &Address,
    client: &PolicyAccountFixtureClient<'_>,
    payload: &[u8; 32],
    failing: &[Assertion],
    retry: &[Assertion],
) -> Outcome {
    let before = state(env, contract_id, client);
    let outcome = authorize(env, contract_id, payload, failing);
    let events = env.events().all().events().len();
    assert_ne!(outcome, Outcome::Accepted);
    assert_eq!(events, 0, "a failed authorization published an event");
    assert!(
        !marker(env, contract_id, payload),
        "a failed authorization left a replay marker"
    );
    assert_eq!(state(env, contract_id, client), before);

    assert_eq!(
        authorize(env, contract_id, payload, retry),
        Outcome::Accepted
    );
    assert!(marker(env, contract_id, payload));
    assert_eq!(
        authorize(env, contract_id, payload, retry),
        Outcome::Refused(PolicyError::Replay)
    );
    outcome
}

#[test]
fn mirrored_keys_reach_the_contract_storage() {
    let env = Env::default();
    let (contract_id, client) = setup(&env, 1);
    let (config, version) = env.as_contract(&contract_id, || {
        let instance = env.storage().instance();
        (
            instance.get::<_, PolicyConfig>(&DataKey::Config),
            instance.get::<_, u32>(&DataKey::ConfigVersion),
        )
    });
    assert_eq!(config, Some(client.config()));
    assert_eq!(version, Some(client.config_version()));
    let payload = [0x50; 32];
    assert_eq!(
        authorize(
            &env,
            &contract_id,
            &payload,
            &[valid(&env, KEY_A, &payload)]
        ),
        Outcome::Accepted
    );
    assert!(marker(&env, &contract_id, &payload));
}

/// A named way to break an otherwise valid assertion.
type Corruption = (&'static str, fn(&mut Assertion));

#[test]
fn a_failed_signature_check_traps_and_leaves_the_payload_usable() {
    let env = Env::default();
    let (contract_id, client) = setup(&env, 1);
    let corruptions: [Corruption; 5] = [
        ("r corrupted", |a| a.signature[0] ^= 0x01),
        ("s corrupted", |a| a.signature[63] ^= 0x01),
        ("authenticator data tampered", |a| {
            a.authenticator_data[33] ^= 0x01;
        }),
        ("client data hash tampered", |a| {
            a.client_data_hash[31] ^= 0x80;
        }),
        ("signature by the other configured key", |a| {
            a.signature = valid(&Env::default(), KEY_B, &a.challenge).signature;
        }),
    ];
    for (index, (name, corrupt)) in corruptions.iter().enumerate() {
        let payload = [0x60 + u8::try_from(index).unwrap(); 32];
        let mut failing = valid(&env, KEY_A, &payload);
        corrupt(&mut failing);
        let outcome = assert_failure_leaves_payload_usable(
            &env,
            &contract_id,
            &client,
            &payload,
            &[failing],
            &[valid(&env, KEY_A, &payload)],
        );
        assert_eq!(outcome, Outcome::HostTrap, "{name}");
    }
}

#[test]
fn one_failing_envelope_fails_the_whole_attempt_whatever_its_position() {
    let env = Env::default();
    // Threshold 1: the valid envelope alone would be enough, yet the attempt must still fail.
    let (contract_id, client) = setup(&env, 1);
    for (index, valid_first) in [true, false].into_iter().enumerate() {
        let payload = [0x70 + u8::try_from(index).unwrap(); 32];
        let good = valid(&env, KEY_A, &payload);
        let mut bad = valid(&env, KEY_B, &payload);
        bad.signature[10] ^= 0x01;
        let failing = if valid_first {
            [good, bad]
        } else {
            [bad, good]
        };
        let outcome = assert_failure_leaves_payload_usable(
            &env,
            &contract_id,
            &client,
            &payload,
            &failing,
            &[valid(&env, KEY_A, &payload)],
        );
        assert_eq!(outcome, Outcome::HostTrap, "valid first: {valid_first}");
    }
}

#[test]
fn a_contract_error_after_a_passing_signature_check_leaves_no_state() {
    let env = Env::default();
    // Threshold 2: key A verifies, then the weight check refuses.
    let (contract_id, client) = setup(&env, 2);
    let payload = [0x80; 32];
    let outcome = assert_failure_leaves_payload_usable(
        &env,
        &contract_id,
        &client,
        &payload,
        &[valid(&env, KEY_A, &payload)],
        &[valid(&env, KEY_A, &payload), valid(&env, KEY_B, &payload)],
    );
    assert_eq!(outcome, Outcome::Refused(PolicyError::InsufficientWeight));
}

#[test]
fn repeated_failures_do_not_accumulate_and_do_not_consume_the_payload() {
    let env = Env::default();
    let (contract_id, client) = setup(&env, 1);
    let payload = [0x90; 32];
    let before = state(&env, &contract_id, &client);
    for byte in 0..64 {
        let mut failing = valid(&env, KEY_A, &payload);
        failing.signature[byte] ^= 0x04;
        assert_ne!(
            authorize(&env, &contract_id, &payload, &[failing]),
            Outcome::Accepted
        );
        assert!(!marker(&env, &contract_id, &payload));
    }
    assert_eq!(state(&env, &contract_id, &client), before);
    assert_eq!(
        authorize(
            &env,
            &contract_id,
            &payload,
            &[valid(&env, KEY_A, &payload)]
        ),
        Outcome::Accepted
    );
}

#[test]
fn a_failure_on_one_payload_does_not_touch_another_payloads_marker() {
    let env = Env::default();
    let (contract_id, client) = setup(&env, 1);
    let used = [0xA0; 32];
    assert_eq!(
        authorize(&env, &contract_id, &used, &[valid(&env, KEY_A, &used)]),
        Outcome::Accepted
    );
    let ttl = env.as_contract(&contract_id, || {
        env.storage()
            .temporary()
            .get_ttl(&DataKey::UsedPayload(BytesN::from_array(&env, &used)))
    });

    let other = [0xA1; 32];
    let mut failing = valid(&env, KEY_A, &other);
    failing.signature[5] ^= 0x01;
    assert_failure_leaves_payload_usable(
        &env,
        &contract_id,
        &client,
        &other,
        &[failing],
        &[valid(&env, KEY_A, &other)],
    );
    assert!(marker(&env, &contract_id, &used));
    let after = env.as_contract(&contract_id, || {
        env.storage()
            .temporary()
            .get_ttl(&DataKey::UsedPayload(BytesN::from_array(&env, &used)))
    });
    assert_eq!(after, ttl);
}

#[derive(Clone, Copy, Debug)]
enum Part {
    Signature,
    AuthenticatorData,
    ClientDataHash,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Any single-bit change to the signed material or the signature makes authorization fail,
    /// by host trap or by contract error, and leaves the payload usable.
    #[test]
    fn any_corruption_fails_without_state(
        part in prop_oneof![Just(Part::Signature), Just(Part::AuthenticatorData), Just(Part::ClientDataHash)],
        index in 0_usize..64,
        bit in 0_u8..8,
        payload in any::<[u8; 32]>(),
    ) {
        let env = Env::default();
        let (contract_id, client) = setup(&env, 1);
        let mut failing = valid(&env, KEY_A, &payload);
        let mask = 1_u8 << bit;
        match part {
            Part::Signature => failing.signature[index] ^= mask,
            Part::AuthenticatorData => {
                let at = index % failing.authenticator_data.len();
                failing.authenticator_data[at] ^= mask;
            }
            Part::ClientDataHash => failing.client_data_hash[index % 32] ^= mask,
        }
        let outcome = assert_failure_leaves_payload_usable(
            &env,
            &contract_id,
            &client,
            &payload,
            &[failing],
            &[valid(&env, KEY_A, &payload)],
        );
        prop_assert!(
            matches!(outcome, Outcome::HostTrap | Outcome::Refused(PolicyError::InvalidPasskey)),
            "{:?}",
            outcome
        );
    }
}
