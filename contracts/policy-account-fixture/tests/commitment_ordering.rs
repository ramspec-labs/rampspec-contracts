//! Signer ordering rule for policy commitments.
//!
//! The fixture stores signers in canonical order, ascending by public key bytes, whatever order a
//! caller lists them in. Equivalent policies therefore store the same configuration and publish the
//! same commitment (`sha256` of the stored `PolicyConfig` XDR, as `store_config` computes it), and
//! a reordered update is a no-op.
//!
//! `fixtures/policy-commitment-vectors.json` publishes commitments for fixed policies so other
//! toolchains can reproduce them; this test checks the file against the contract. To regenerate it
//! after an intended change:
//! `RAMPSPEC_WRITE_VECTORS=1 cargo test -p policy-account-fixture --test commitment_ordering`.

use ed25519_dalek::SigningKey;
use p256::ecdsa::SigningKey as P256SigningKey;
use policy_account_fixture::{
    Ed25519Signer, PasskeySigner, PolicyAccountFixture, PolicyAccountFixtureClient, PolicyError,
};
use proptest::prelude::*;
use serde_json::{Value, json};
use soroban_sdk::{
    Address, BytesN, Env, TryFromVal, Vec,
    testutils::Events as _,
    xdr::{AccountId, PublicKey, ScAddress, ToXdr, Uint256},
};
use std::path::PathBuf;

type Signers = std::vec::Vec<(u8, u32)>;

/// A fixed account, so commitments (which cover the administrator) are reproducible.
fn admin(env: &Env) -> Address {
    let account = ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
        [0x11; 32],
    ))));
    Address::try_from_val(env, &account).unwrap()
}

fn ed25519_key(seed: u8) -> [u8; 32] {
    *SigningKey::from_bytes(&[seed; 32])
        .verifying_key()
        .as_bytes()
}

fn passkey_key(seed: u8) -> [u8; 65] {
    let key = P256SigningKey::from_bytes((&[seed; 32]).into()).unwrap();
    key.verifying_key()
        .to_encoded_point(false)
        .as_bytes()
        .try_into()
        .unwrap()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Ed25519,
    Passkey,
}

impl Mode {
    fn public_key(self, seed: u8) -> std::vec::Vec<u8> {
        match self {
            Mode::Ed25519 => ed25519_key(seed).to_vec(),
            Mode::Passkey => passkey_key(seed).to_vec(),
        }
    }
}

/// One published vector: signers as `(seed, weight)`, deliberately not in canonical order.
struct Definition {
    name: &'static str,
    mode: Mode,
    signers: &'static [(u8, u32)],
    threshold: u32,
}

const DEFINITIONS: &[Definition] = &[
    Definition {
        name: "ed25519-single",
        mode: Mode::Ed25519,
        signers: &[(1, 1)],
        threshold: 1,
    },
    Definition {
        name: "ed25519-two-of-three",
        mode: Mode::Ed25519,
        signers: &[(3, 1), (1, 1), (2, 1)],
        threshold: 2,
    },
    Definition {
        name: "ed25519-three-of-three",
        mode: Mode::Ed25519,
        signers: &[(3, 1), (1, 1), (2, 1)],
        threshold: 3,
    },
    Definition {
        name: "ed25519-weighted",
        mode: Mode::Ed25519,
        signers: &[(2, 3), (4, 1), (1, 2)],
        threshold: 3,
    },
    Definition {
        name: "passkey-two-of-two",
        mode: Mode::Passkey,
        signers: &[(6, 1), (5, 1)],
        threshold: 2,
    },
    Definition {
        name: "passkey-weighted",
        mode: Mode::Passkey,
        signers: &[(7, 2), (5, 1), (6, 1)],
        threshold: 2,
    },
];

fn ed25519_signers(env: &Env, signers: &[(u8, u32)]) -> Vec<Ed25519Signer> {
    let mut out = Vec::new(env);
    for &(seed, weight) in signers {
        out.push_back(Ed25519Signer {
            public_key: BytesN::from_array(env, &ed25519_key(seed)),
            weight,
        });
    }
    out
}

fn passkey_signers(env: &Env, signers: &[(u8, u32)]) -> Vec<PasskeySigner> {
    let mut out = Vec::new(env);
    for &(seed, weight) in signers {
        out.push_back(PasskeySigner {
            public_key: BytesN::from_array(env, &passkey_key(seed)),
            weight,
        });
    }
    out
}

fn events(env: &Env) -> usize {
    env.events().all().events().len()
}

/// The stored policy's commitment, computed exactly as `store_config` does.
fn commitment(env: &Env, client: &PolicyAccountFixtureClient<'_>) -> BytesN<32> {
    env.crypto().sha256(&client.config().to_xdr(env)).to_bytes()
}

/// A fresh fixture with a definition's policy installed, signers in the given order.
fn install<'a>(
    env: &'a Env,
    mode: Mode,
    signers: &[(u8, u32)],
    threshold: u32,
) -> PolicyAccountFixtureClient<'a> {
    env.mock_all_auths();
    let client = PolicyAccountFixtureClient::new(env, &env.register(PolicyAccountFixture, ()));
    match mode {
        Mode::Ed25519 => client.initialize(&admin(env), &ed25519_signers(env, signers), &threshold),
        Mode::Passkey => {
            client.initialize(&admin(env), &ed25519_signers(env, &[(9, 1)]), &1);
            client.set_passkeys(&passkey_signers(env, signers), &threshold);
        }
    }
    assert_eq!(events(env), 1, "installing a policy publishes one change");
    client
}

fn permutations(items: &[(u8, u32)]) -> std::vec::Vec<Signers> {
    if items.len() <= 1 {
        return std::vec![items.to_vec()];
    }
    let mut out = std::vec::Vec::new();
    for i in 0..items.len() {
        let mut rest = items.to_vec();
        let first = rest.remove(i);
        for mut tail in permutations(&rest) {
            tail.insert(0, first);
            out.push(tail);
        }
    }
    out
}

fn canonical(mode: Mode, signers: &[(u8, u32)]) -> Signers {
    let mut sorted = signers.to_vec();
    sorted.sort_by_key(|&(seed, _)| mode.public_key(seed));
    sorted
}

/// The vectors file as the contract computes it now.
fn computed_vectors() -> Value {
    let vectors: std::vec::Vec<Value> = DEFINITIONS
        .iter()
        .map(|d| {
            let env = Env::default();
            let client = install(&env, d.mode, d.signers, d.threshold);
            let signers: std::vec::Vec<Value> = canonical(d.mode, d.signers)
                .iter()
                .map(|&(seed, weight)| {
                    json!({ "publicKey": hex(&d.mode.public_key(seed)), "weight": weight })
                })
                .collect();
            json!({
                "name": d.name,
                "signatureMode": match d.mode {
                    Mode::Ed25519 => "Ed25519",
                    Mode::Passkey => "PasskeyCompatible",
                },
                "threshold": d.threshold,
                "signers": signers,
                "commitment": hex(&commitment(&env, &client).to_array()),
            })
        })
        .collect();
    let env = Env::default();
    json!({
        "description": "SHA-256 policy commitments of the policy-account fixture for fixed, test-only policies. The keys are derived from fixed test seeds; there are no production keys or credentials.",
        "orderingRule": "Signers are stored ascending by public key bytes (32-byte Ed25519 keys, 65-byte uncompressed P-256 keys), whatever order they are given in. Every permutation of a vector's signers yields its commitment.",
        "commitment": "sha256 of the XDR of the stored PolicyConfig: admin, signers, passkey_signers, threshold, signature_mode, controls.",
        "admin": admin(&env).to_string().to_string(),
        "controls": "defaults: valid_from_ledger 0, expires_at_ledger 4294967295, reject_all false, no additional signer, contract or function",
        "passkeyVectors": "installed with set_passkeys after initialize, so their Ed25519 signer list is empty",
        "regenerate": "RAMPSPEC_WRITE_VECTORS=1 cargo test -p policy-account-fixture --test commitment_ordering",
        "vectors": vectors,
    })
}

fn vectors_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/policy-commitment-vectors.json")
}

#[test]
fn published_vectors_match_the_contract() {
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
fn every_signer_order_stores_the_canonical_list_and_one_commitment() {
    for d in DEFINITIONS {
        let env = Env::default();
        let expected = commitment(&env, &install(&env, d.mode, d.signers, d.threshold));
        let sorted = canonical(d.mode, d.signers);
        for order in permutations(d.signers) {
            let env = Env::default();
            let client = install(&env, d.mode, &order, d.threshold);
            assert_eq!(
                commitment(&env, &client),
                expected,
                "{} in order {order:?}",
                d.name
            );
            let config = client.config();
            match d.mode {
                Mode::Ed25519 => assert_eq!(config.signers, ed25519_signers(&env, &sorted)),
                Mode::Passkey => {
                    assert_eq!(config.passkey_signers, passkey_signers(&env, &sorted));
                }
            }
        }
    }
}

#[test]
fn distinct_policies_have_distinct_commitments() {
    let mut seen = std::collections::BTreeSet::new();
    for d in DEFINITIONS {
        let env = Env::default();
        let client = install(&env, d.mode, d.signers, d.threshold);
        assert!(
            seen.insert(commitment(&env, &client).to_array()),
            "{} shares a commitment",
            d.name
        );
    }
}

#[test]
fn a_reordered_update_is_a_no_op() {
    let signers = [(1, 1), (2, 1), (3, 1)];
    let env = Env::default();
    let client = install(&env, Mode::Ed25519, &signers, 2);
    let config = client.config();
    for order in permutations(&signers) {
        client.set_signers(&ed25519_signers(&env, &order), &2);
        assert_eq!(events(&env), 0, "{order:?} published a change");
        assert_eq!(client.config_version(), 1);
        assert_eq!(client.config(), config);
    }

    let env = Env::default();
    let client = install(&env, Mode::Passkey, &[(5, 1), (6, 1)], 2);
    let version = client.config_version();
    client.set_passkeys(&passkey_signers(&env, &[(6, 1), (5, 1)]), &2);
    assert_eq!(events(&env), 0);
    assert_eq!(client.config_version(), version);
}

#[test]
fn a_real_change_given_in_another_order_still_advances_once() {
    let env = Env::default();
    let client = install(&env, Mode::Ed25519, &[(1, 1), (2, 1)], 1);
    let before = commitment(&env, &client);
    client.set_signers(&ed25519_signers(&env, &[(2, 1), (1, 1)]), &2);
    assert_eq!(events(&env), 1);
    assert_eq!(client.config_version(), 2);
    assert_ne!(commitment(&env, &client), before);
}

#[test]
fn a_duplicate_key_is_refused_wherever_it_appears_and_changes_nothing() {
    for mode in [Mode::Ed25519, Mode::Passkey] {
        for order in permutations(&[(1, 1), (2, 1), (1, 2)]) {
            let env = Env::default();
            let client = install(&env, mode, &[(3, 1)], 1);
            let config = client.config();
            let version = client.config_version();
            let result = match mode {
                Mode::Ed25519 => client.try_set_signers(&ed25519_signers(&env, &order), &1),
                Mode::Passkey => client.try_set_passkeys(&passkey_signers(&env, &order), &1),
            };
            assert_eq!(
                result,
                Err(Ok(PolicyError::InvalidPolicy)),
                "{mode:?} {order:?}"
            );
            assert_eq!(events(&env), 0);
            assert_eq!(client.config(), config);
            assert_eq!(client.config_version(), version);
        }
    }

    let env = Env::default();
    env.mock_all_auths();
    let client = PolicyAccountFixtureClient::new(&env, &env.register(PolicyAccountFixture, ()));
    let signers = ed25519_signers(&env, &[(2, 1), (1, 1), (2, 1)]);
    assert_eq!(
        client.try_initialize(&admin(&env), &signers, &1),
        Err(Ok(PolicyError::InvalidPolicy))
    );
    assert_eq!(client.try_config(), Err(Ok(PolicyError::NotInitialized)));
}

#[test]
fn a_reordered_update_still_needs_the_administrator() {
    let env = Env::default();
    let client = install(&env, Mode::Ed25519, &[(1, 1), (2, 1)], 1);
    let config = client.config();
    env.set_auths(&[]);
    let result = client.try_set_signers(&ed25519_signers(&env, &[(2, 1), (1, 1)]), &1);
    assert!(matches!(result, Err(Err(_))), "{result:?}");
    assert_eq!(client.config(), config);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Any set of distinct keys, with any weights, commits the same in any order.
    #[test]
    fn commitments_do_not_depend_on_signer_order(
        seeds in proptest::sample::subsequence((1_u8..=12).collect::<std::vec::Vec<_>>(), 1..=5),
        weights in proptest::collection::vec(1_u32..=4, 5),
        pick in any::<proptest::sample::Index>(),
        passkey in any::<bool>(),
    ) {
        let signers: Signers = seeds.iter().zip(&weights).map(|(&s, &w)| (s, w)).collect();
        let threshold = signers.iter().map(|&(_, w)| w).sum::<u32>();
        let mode = if passkey { Mode::Passkey } else { Mode::Ed25519 };
        let orders = permutations(&signers);
        let reordered = &orders[pick.index(orders.len())];

        let env = Env::default();
        let given = commitment(&env, &install(&env, mode, &signers, threshold));
        let env = Env::default();
        let shuffled = commitment(&env, &install(&env, mode, reordered, threshold));
        prop_assert_eq!(given, shuffled);
    }
}

#[test]
fn sorting_the_largest_policy_stays_within_the_set_signers_budget() {
    // 16 signers (MAX_SIGNERS), given in descending key order: the most work the sort can do.
    let mut seeds: Signers = (1_u8..=16).map(|seed| (seed, 1)).collect();
    seeds.sort_by_key(|&(seed, _)| std::cmp::Reverse(Mode::Ed25519.public_key(seed)));
    let env = Env::default();
    let client = install(&env, Mode::Ed25519, &[(20, 1)], 1);
    env.cost_estimate().budget().reset_tracker();

    client.set_signers(&ed25519_signers(&env, &seeds), &16);

    // tests/resource-budgets/baselines.json: set_signers cpuMax 20_000_000, memoryMax 16_000_000.
    let resources = env.cost_estimate().resources();
    assert!(
        resources.instructions <= 20_000_000,
        "{}",
        resources.instructions
    );
    assert!(resources.mem_bytes <= 16_000_000, "{}", resources.mem_bytes);
    assert_eq!(
        client.config().signers,
        ed25519_signers(&env, &canonical(Mode::Ed25519, &seeds))
    );
}
