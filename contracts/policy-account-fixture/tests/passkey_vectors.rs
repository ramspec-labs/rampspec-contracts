//! Deterministic passkey assertion vectors.
//!
//! `fixtures/passkey-assertion-vectors.json` publishes positive and negative passkey-compatible
//! assertions for a fixed policy, so verification can be reproduced without runtime-generated
//! keys. The P-256 keys come from fixed test seeds and signatures are RFC 6979 deterministic, so
//! the file is byte-stable. This test checks it against the contract's `__check_auth`; the same
//! file is verified with Node's own P-256 implementation by `scripts/passkey-vectors.test.mjs`.
//! To regenerate after an intended change:
//! `RAMPSPEC_WRITE_VECTORS=1 cargo test -p policy-account-fixture --test passkey_vectors`.

use ed25519_dalek::SigningKey;
use p256::ecdsa::{Signature, SigningKey as P256SigningKey, signature::hazmat::PrehashSigner};
use policy_account_fixture::{
    AccountSignature, Ed25519Signer, PasskeySignature, PasskeySigner, PolicyAccountFixture,
    PolicyAccountFixtureClient, PolicyError,
};
use serde_json::{Value, json};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, IntoVal, Symbol, TryFromVal, Vec,
    auth::{Context, ContractContext},
    contracttype,
    xdr::{AccountId, PublicKey, ScAddress, Uint256},
};
use std::path::PathBuf;

/// The fixture's private storage keys, declared the same way so the test can see replay markers.
#[contracttype]
enum DataKey {
    Config,
    ConfigVersion,
    UsedPayload(BytesN<32>),
}

/// SHA-256 through the Soroban host, as the contract computes its digest, so the test needs no
/// extra hashing dependency.
fn sha256(data: &[u8]) -> [u8; 32] {
    let env = Env::default();
    env.crypto()
        .sha256(&Bytes::from_slice(&env, data))
        .to_array()
}

/// Test-only P-256 seeds. Never production keys.
const KEY_A: u8 = 0x21;
const KEY_B: u8 = 0x22;
const KEY_UNCONFIGURED: u8 = 0x23;
const PAYLOAD_BASE: u8 = 0x40;

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

/// Authenticator data of the given length: a fixed 32-byte RP ID hash stand-in, flags (user
/// present and verified), a sign counter, then filler for longer vectors.
fn authenticator_data(length: usize) -> std::vec::Vec<u8> {
    let mut data = std::vec![0xA5; 32];
    data.push(0x05);
    data.extend_from_slice(&[0, 0, 0, 1]);
    data.resize(length, 0x5A);
    data
}

const CLIENT_DATA_HASH: [u8; 32] = [0xB6; 32];

fn sign(seed: u8, authenticator_data: &[u8], client_data_hash: &[u8; 32]) -> [u8; 64] {
    let mut message = authenticator_data.to_vec();
    message.extend_from_slice(client_data_hash);
    let signature: Signature = p256_key(seed).sign_prehash(&sha256(&message)).unwrap();
    let signature = signature.normalize_s().unwrap_or(signature);
    signature.to_bytes().into()
}

/// The same signature with `s` replaced by `n - s`: valid ECDSA, but not canonical low-S.
fn high_s(signature: &[u8; 64]) -> [u8; 64] {
    const ORDER: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2, 0xfc, 0x63,
        0x25, 0x51,
    ];
    let mut out = *signature;
    let mut borrow = 0_i16;
    for index in (0..32).rev() {
        let mut difference = i16::from(ORDER[index]) - i16::from(signature[32 + index]) - borrow;
        borrow = i16::from(difference < 0);
        if difference < 0 {
            difference += 256;
        }
        out[32 + index] = u8::try_from(difference).unwrap();
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Expected {
    Accepted,
    Refused(PolicyError),
    /// The host's P-256 verification fails, which traps instead of returning a contract error.
    HostTrap,
}

struct Vector {
    name: &'static str,
    payload: [u8; 32],
    public_key: [u8; 65],
    authenticator_data: std::vec::Vec<u8>,
    client_data_hash: [u8; 32],
    challenge: [u8; 32],
    signature: [u8; 64],
    expected: Expected,
}

#[allow(clippy::too_many_lines)]
fn vectors() -> std::vec::Vec<Vector> {
    let mut out = std::vec::Vec::new();
    let mut next = PAYLOAD_BASE;
    let mut payload = || {
        next += 1;
        [next; 32]
    };
    let valid = |name, seed, length, payload: [u8; 32]| {
        let data = authenticator_data(length);
        Vector {
            name,
            payload,
            public_key: public_key(seed),
            signature: sign(seed, &data, &CLIENT_DATA_HASH),
            authenticator_data: data,
            client_data_hash: CLIENT_DATA_HASH,
            challenge: payload,
            expected: Expected::Accepted,
        }
    };

    out.push(valid(
        "key-a-minimum-authenticator-data",
        KEY_A,
        37,
        payload(),
    ));
    out.push(valid(
        "key-a-maximum-authenticator-data",
        KEY_A,
        512,
        payload(),
    ));
    out.push(valid(
        "key-b-typical-authenticator-data",
        KEY_B,
        64,
        payload(),
    ));

    let mut v = valid("challenge-differs-from-payload", KEY_A, 37, payload());
    v.challenge = [0xEE; 32];
    v.expected = Expected::Refused(PolicyError::InvalidPasskey);
    out.push(v);

    let mut v = valid("authenticator-data-too-short", KEY_A, 36, payload());
    v.expected = Expected::Refused(PolicyError::InvalidPasskey);
    out.push(v);

    let mut v = valid("authenticator-data-too-long", KEY_A, 513, payload());
    v.expected = Expected::Refused(PolicyError::InvalidPasskey);
    out.push(v);

    let mut v = valid("high-s-signature", KEY_A, 37, payload());
    v.signature = high_s(&v.signature);
    v.expected = Expected::Refused(PolicyError::InvalidPasskey);
    out.push(v);

    let mut v = valid("key-not-in-policy", KEY_UNCONFIGURED, 37, payload());
    v.expected = Expected::Refused(PolicyError::UnauthorizedSigner);
    out.push(v);

    let mut v = valid("signature-r-corrupted", KEY_A, 37, payload());
    v.signature[0] ^= 0x01;
    v.expected = Expected::HostTrap;
    out.push(v);

    let mut v = valid("client-data-hash-tampered", KEY_A, 37, payload());
    v.client_data_hash[0] ^= 0x01;
    v.expected = Expected::HostTrap;
    out.push(v);

    let mut v = valid("authenticator-data-tampered", KEY_A, 37, payload());
    v.authenticator_data[32] ^= 0x01;
    v.expected = Expected::HostTrap;
    out.push(v);

    let mut v = valid("signature-by-another-configured-key", KEY_A, 37, payload());
    v.signature = sign(KEY_B, &v.authenticator_data, &v.client_data_hash);
    v.expected = Expected::HostTrap;
    out.push(v);

    out
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

fn admin(env: &Env) -> Address {
    let account = ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
        [0x11; 32],
    ))));
    Address::try_from_val(env, &account).unwrap()
}

/// The policy every vector is checked against: keys A and B, weight 1 each, threshold 1.
fn install(env: &Env) -> Address {
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
    client.initialize(&admin(env), &Vec::from_array(env, [bootstrap]), &1);
    let passkeys = Vec::from_array(
        env,
        [KEY_A, KEY_B].map(|seed| PasskeySigner {
            public_key: BytesN::from_array(env, &public_key(seed)),
            weight: 1,
        }),
    );
    client.set_passkeys(&passkeys, &1);
    contract_id
}

fn envelope(env: &Env, v: &Vector) -> Vec<AccountSignature> {
    Vec::from_array(
        env,
        [AccountSignature::Passkey(PasskeySignature {
            public_key: BytesN::from_array(env, &v.public_key),
            authenticator_data: Bytes::from_slice(env, &v.authenticator_data),
            client_data_hash: BytesN::from_array(env, &v.client_data_hash),
            challenge: BytesN::from_array(env, &v.challenge),
            signature: BytesN::from_array(env, &v.signature),
        })],
    )
}

fn check(env: &Env, contract_id: &Address, payload: &[u8; 32], v: &Vector) -> Expected {
    let result = env.try_invoke_contract_check_auth::<PolicyError>(
        contract_id,
        &BytesN::from_array(env, payload),
        envelope(env, v).into_val(env),
        &Vec::from_array(
            env,
            [Context::Contract(ContractContext {
                contract: contract_id.clone(),
                fn_name: Symbol::new(env, "is_test_only"),
                args: Vec::new(env),
            })],
        ),
    );
    match result {
        Ok(()) => Expected::Accepted,
        Err(Ok(error)) => Expected::Refused(error),
        Err(Err(_)) => Expected::HostTrap,
    }
}

fn replay_marker(env: &Env, contract_id: &Address, payload: &[u8; 32]) -> bool {
    env.as_contract(contract_id, || {
        env.storage()
            .temporary()
            .has(&DataKey::UsedPayload(BytesN::from_array(env, payload)))
    })
}

fn expected_json(expected: Expected) -> Value {
    match expected {
        Expected::Accepted => json!({ "result": "accepted" }),
        Expected::Refused(error) => {
            json!({ "result": "contractError", "error": format!("{error:?}"), "code": error as u32 })
        }
        Expected::HostTrap => json!({ "result": "hostTrap" }),
    }
}

fn computed_vectors() -> Value {
    let env = Env::default();
    let passkey_signers: std::vec::Vec<Value> = [KEY_A, KEY_B]
        .iter()
        .map(|&seed| json!({ "publicKey": hex(&public_key(seed)), "weight": 1 }))
        .collect();
    let vectors: std::vec::Vec<Value> = vectors()
        .iter()
        .map(|v| {
            json!({
                "name": v.name,
                "payload": hex(&v.payload),
                "assertion": {
                    "publicKey": hex(&v.public_key),
                    "authenticatorData": hex(&v.authenticator_data),
                    "clientDataHash": hex(&v.client_data_hash),
                    "challenge": hex(&v.challenge),
                    "signature": hex(&v.signature),
                },
                "expected": expected_json(v.expected),
            })
        })
        .collect();
    json!({
        "description": "Passkey-compatible assertion vectors for the policy-account fixture. Test-only: keys come from fixed seeds (0x21, 0x22, 0x23 repeated) and authenticator data is fixed filler; there are no production keys, credentials or relying-party identifiers.",
        "policy": {
            "admin": admin(&env).to_string().to_string(),
            "threshold": 1,
            "passkeySigners": passkey_signers,
        },
        "verification": "__check_auth with one Passkey envelope, the vector's payload as signature_payload, and one contract-call auth context (default controls require no particular contract or function). In order: challenge must equal the payload; authenticatorData must be 37 to 512 bytes; signature must be a canonical low-S P-256 signature (r || s, 64 bytes); publicKey must be a configured signer; then secp256r1 verification of sha256(authenticatorData || clientDataHash). The first four refuse with a contract error; a failed signature check traps in the host.",
        "replay": "No refused or trapped vector leaves a replay marker: the payload stays usable by a later valid assertion.",
        "regenerate": "RAMPSPEC_WRITE_VECTORS=1 cargo test -p policy-account-fixture --test passkey_vectors",
        "vectors": vectors,
    })
}

fn vectors_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/passkey-assertion-vectors.json")
}

#[test]
fn published_vectors_match_the_definitions() {
    let computed = computed_vectors();
    let path = vectors_path();
    if std::env::var_os("RAMPSPEC_WRITE_VECTORS").is_some() {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&computed).unwrap() + "\n",
        )
        .unwrap();
    }
    let published: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        published, computed,
        "regenerate the vectors with RAMPSPEC_WRITE_VECTORS=1 if the change is intended"
    );
}

#[test]
fn every_vector_has_its_published_outcome() {
    for v in vectors() {
        let env = Env::default();
        let contract_id = install(&env);
        assert_eq!(
            check(&env, &contract_id, &v.payload, &v),
            v.expected,
            "{}",
            v.name
        );
        assert_eq!(
            replay_marker(&env, &contract_id, &v.payload),
            v.expected == Expected::Accepted,
            "{}: replay marker",
            v.name
        );
    }
}

#[test]
fn a_rejected_vector_leaves_its_payload_usable_once() {
    for v in vectors()
        .iter()
        .filter(|v| v.expected != Expected::Accepted)
    {
        let env = Env::default();
        let contract_id = install(&env);
        assert_ne!(check(&env, &contract_id, &v.payload, v), Expected::Accepted);

        // A fresh valid assertion for the same payload is still accepted, and only once.
        let data = authenticator_data(37);
        let retry = Vector {
            name: "retry",
            payload: v.payload,
            public_key: public_key(KEY_A),
            signature: sign(KEY_A, &data, &CLIENT_DATA_HASH),
            authenticator_data: data,
            client_data_hash: CLIENT_DATA_HASH,
            challenge: v.payload,
            expected: Expected::Accepted,
        };
        assert_eq!(
            check(&env, &contract_id, &v.payload, &retry),
            Expected::Accepted,
            "{}",
            v.name
        );
        assert!(replay_marker(&env, &contract_id, &v.payload));
        assert_eq!(
            check(&env, &contract_id, &v.payload, &retry),
            Expected::Refused(PolicyError::Replay),
            "{}",
            v.name
        );
    }
}

#[test]
fn accepted_vectors_cannot_be_replayed() {
    for v in vectors()
        .iter()
        .filter(|v| v.expected == Expected::Accepted)
    {
        let env = Env::default();
        let contract_id = install(&env);
        assert_eq!(check(&env, &contract_id, &v.payload, v), Expected::Accepted);
        assert_eq!(
            check(&env, &contract_id, &v.payload, v),
            Expected::Refused(PolicyError::Replay),
            "{}",
            v.name
        );
    }
}

#[test]
fn signatures_are_deterministic_and_canonical() {
    // Signing again gives the same bytes (RFC 6979), so the file never changes by itself.
    for v in vectors()
        .iter()
        .filter(|v| v.expected == Expected::Accepted)
    {
        let seed = [KEY_A, KEY_B]
            .into_iter()
            .find(|&seed| public_key(seed) == v.public_key)
            .unwrap();
        assert_eq!(
            sign(seed, &v.authenticator_data, &v.client_data_hash),
            v.signature
        );
        let parsed = Signature::from_slice(&v.signature).unwrap();
        assert!(parsed.normalize_s().is_none(), "{} is not low-S", v.name);
    }
}

#[test]
fn mirrored_storage_keys_reach_the_contract() {
    let env = Env::default();
    let contract_id = install(&env);
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    let (config, version) = env.as_contract(&contract_id, || {
        let instance = env.storage().instance();
        (
            instance.get::<_, policy_account_fixture::PolicyConfig>(&DataKey::Config),
            instance.get::<_, u32>(&DataKey::ConfigVersion),
        )
    });
    assert_eq!(config, Some(client.config()));
    assert_eq!(version, Some(client.config_version()));
}
