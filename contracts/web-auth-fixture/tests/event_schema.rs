use soroban_sdk::{
    address_payload::AddressPayload,
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String, Symbol,
};
use web_auth_fixture::{FixtureMode, WebAuthError, WebAuthFixture, WebAuthFixtureClient};

const CURRENT_LEDGER: u32 = 100;
const RAW_NONCE: &str = "raw-secret-nonce-value";

fn setup(mode: FixtureMode) -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(CURRENT_LEDGER);
    let contract_id = env.register(WebAuthFixture, ());
    let admin = Address::generate(&env);
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &mode, &(CURRENT_LEDGER - 1), &(CURRENT_LEDGER + 1));
    let account = Address::generate(&env);
    let server = AddressPayload::AccountIdPublicKeyEd25519(BytesN::from_array(&env, &[1; 32]))
        .to_address(&env);
    (env, contract_id, account, server)
}

fn args(
    env: &Env,
    account: &Address,
    server: &Address,
    nonce: &str,
) -> soroban_sdk::Map<Symbol, String> {
    let mut values = soroban_sdk::Map::new(env);
    values.set(Symbol::new(env, "account"), account.to_string());
    values.set(
        Symbol::new(env, "home_domain"),
        String::from_str(env, "example.com"),
    );
    values.set(
        Symbol::new(env, "web_auth_domain"),
        String::from_str(env, "auth.example.com"),
    );
    values.set(
        Symbol::new(env, "web_auth_domain_account"),
        server.to_string(),
    );
    values.set(Symbol::new(env, "nonce"), String::from_str(env, nonce));
    values
}

#[test]
fn successful_events_omit_the_raw_nonce() {
    let (env, contract_id, account, server) = setup(FixtureMode::Accept);
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    client.web_auth_verify(&args(&env, &account, &server, RAW_NONCE));
    let rendered = format!("{:?}", env.events().all());
    assert!(!rendered.contains(RAW_NONCE));
}

#[test]
fn rejected_verify_emits_nothing_and_hides_input() {
    let (env, contract_id, account, server) = setup(FixtureMode::Reject);
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    let before = env.events().all().events().len();
    assert_eq!(
        client.try_web_auth_verify(&args(&env, &account, &server, RAW_NONCE)),
        Err(Ok(WebAuthError::Rejected))
    );
    assert_eq!(env.events().all().events().len(), before);
    assert!(!format!("{:?}", env.events().all()).contains(RAW_NONCE));
}

#[test]
fn nonce_reset_event_uses_a_hash_topic() {
    let (env, contract_id, account, server) = setup(FixtureMode::Accept);
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    client.web_auth_verify(&args(&env, &account, &server, RAW_NONCE));
    client.reset_nonce(&String::from_str(&env, RAW_NONCE));
    let rendered = format!("{:?}", env.events().all());
    assert!(!rendered.contains(RAW_NONCE));
}
