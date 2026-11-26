use soroban_sdk::{
    address_payload::AddressPayload,
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String, Symbol,
};
use web_auth_fixture::{FixtureMode, WebAuthFixture, WebAuthFixtureClient};

const CURRENT_LEDGER: u32 = 100;

fn setup() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(CURRENT_LEDGER);
    let contract_id = env.register(WebAuthFixture, ());
    let admin = Address::generate(&env);
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    client.initialize(
        &admin,
        &FixtureMode::Accept,
        &(CURRENT_LEDGER - 1),
        &(CURRENT_LEDGER + 1),
    );
    (env, contract_id)
}

fn assert_budget(env: &Env, cpu_max: u64, memory_max: u64) {
    let budget = env.cost_estimate().budget();
    let cpu = budget.cpu_instruction_cost();
    let memory = budget.memory_bytes_cost();
    assert!(cpu <= cpu_max, "CPU cost {cpu} exceeds {cpu_max}");
    assert!(
        memory <= memory_max,
        "memory cost {memory} exceeds {memory_max}"
    );
}

#[test]
fn nonce_reset_stays_within_reviewed_budget() {
    let (env, contract_id) = setup();
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    let account = Address::generate(&env);
    let server = AddressPayload::AccountIdPublicKeyEd25519(BytesN::from_array(&env, &[1; 32]))
        .to_address(&env);
    let mut values = soroban_sdk::Map::new(&env);
    values.set(Symbol::new(&env, "account"), account.to_string());
    values.set(
        Symbol::new(&env, "home_domain"),
        String::from_str(&env, "example.com"),
    );
    values.set(
        Symbol::new(&env, "web_auth_domain"),
        String::from_str(&env, "auth.example.com"),
    );
    values.set(
        Symbol::new(&env, "web_auth_domain_account"),
        server.to_string(),
    );
    values.set(
        Symbol::new(&env, "nonce"),
        String::from_str(&env, "budget-nonce"),
    );
    client.web_auth_verify(&values);

    env.cost_estimate().budget().reset_tracker();
    client.reset_nonce(&String::from_str(&env, "budget-nonce"));
    assert_budget(&env, 15_000_000, 12_000_000);
}

#[test]
fn rejected_nonce_reset_stays_within_budget_without_writes() {
    let (env, contract_id) = setup();
    let client = WebAuthFixtureClient::new(&env, &contract_id);
    let before = env.events().all().events().len();
    env.cost_estimate().budget().reset_tracker();
    let _ = client.try_reset_nonce(&String::from_str(&env, "absent"));
    assert_budget(&env, 15_000_000, 12_000_000);
    assert_eq!(env.events().all().events().len(), before);
    assert!(!client.is_nonce_consumed(&String::from_str(&env, "absent")));
}
