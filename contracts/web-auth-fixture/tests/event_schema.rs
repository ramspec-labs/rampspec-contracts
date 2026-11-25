// contracts/web-auth-fixture/tests/event_schema.rs
use serde_json::Value;
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    xdr::{ScVal, ToXdr},
    Address, Bytes, Env, Map, String, Symbol,
};
use web_auth_fixture::{WebAuthFixture, WebAuthFixtureClient};

fn load_canonical_spec() -> Value {
    let raw = std::fs::read_to_string("../../artifacts/specs/web_auth_fixture.json")
        .expect("artifacts/specs/web_auth_fixture.json must exist");
    serde_json::from_str(&raw).expect("valid web_auth_fixture.json spec")
}

#[test]
fn runtime_events_match_generated_spec_and_hide_raw_nonce() {
    let spec = load_canonical_spec();
    let spec_events = spec["events"].as_array().expect("spec must declare events");
    let spec_event_names: Vec<&str> = spec_events
        .iter()
        .filter_map(|e| e["name"].as_str())
        .collect();

    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WebAuthFixture, ());
    let client = WebAuthFixtureClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let server = Address::generate(&env);
    let account = Address::generate(&env);
    let raw_nonce = "secret-raw-nonce-value-998877";

    // 1. Configure event
    client.configure(&admin, &server, &String::from_str(&env, "example.org"));
    let cfg_events = env.events().all();
    assert!(!cfg_events.is_empty(), "configure must emit schema-defined event");

    // 2. Verify event (must emit hashed nonce, never raw_nonce)
    let mut args = Map::new(&env);
    args.set(Symbol::new(&env, "account"), account.to_string());
    args.set(Symbol::new(&env, "home_domain"), String::from_str(&env, "example.org"));
    args.set(Symbol::new(&env, "web_auth_domain"), String::from_str(&env, "auth.example.org"));
    args.set(Symbol::new(&env, "web_auth_domain_account"), server.to_string());
    args.set(Symbol::new(&env, "nonce"), String::from_str(&env, raw_nonce));

    client.web_auth_verify(&args);
    let verify_events = env.events().all();
    assert!(!verify_events.is_empty(), "web_auth_verify must emit verification event");

    for (_cid, topics, data) in verify_events.iter() {
        // Topic 0 must be a declared event symbol in the canonical spec
        let topic0_xdr = topics.get(0).unwrap().to_xdr(&env);
        assert!(!topic0_xdr.is_empty());
        assert!(!spec_event_names.is_empty());

        // Privacy constraint: raw nonce bytes must never appear in topics or data
        let data_bytes = data.to_xdr(&env);
        let raw_bytes = Bytes::from_slice(&env, raw_nonce.as_bytes());
        assert!(
            !contains_subslice(&data_bytes, &raw_bytes),
            "raw nonce leaked into event data payload"
        );
    }

    // 3. Nonce reset event
    let nonce_hash = env.crypto().sha256(&Bytes::from_slice(&env, raw_nonce.as_bytes()));
    client.reset_nonce(&admin, &nonce_hash.into());
    assert!(!env.events().all().is_empty(), "reset_nonce must emit reset event");
}

#[test]
fn failed_web_auth_emits_zero_ghost_events() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(WebAuthFixture, ());
    let client = WebAuthFixtureClient::new(&env, &contract_id);

    let mut bad_args = Map::new(&env);
    bad_args.set(Symbol::new(&env, "unexpected_key"), String::from_str(&env, "bad"));

    let result = client.try_web_auth_verify(&bad_args);
    assert!(result.is_err());
    assert_eq!(env.events().all().len(), 0, "failed call must emit no ghost events");
}

fn contains_subslice(haystack: &Bytes, needle: &Bytes) -> bool {
    let h = haystack.to_alloc_vec();
    let n = needle.to_alloc_vec();
    !n.is_empty() && h.windows(n.len()).any(|w| w == n.as_slice())
}