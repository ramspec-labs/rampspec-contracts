use evidence_registry::{EvidenceRegistry, EvidenceRegistryClient};
use rampspec_shared_types::{
    EVIDENCE_ID_DOMAIN, EvidenceInput, NetworkKind, PersistentKeyV1, SCHEMA_VERSION,
    SUPPORTED_PROTOCOL_BITMAP,
};
use serde_json::Value;
use soroban_sdk::{
    Address, Bytes, BytesN, Env, Map, String, Symbol,
    testutils::{
        Address as _, Events as _, Ledger as _, storage::Instance as _, storage::Persistent as _,
    },
    xdr::ToXdr,
};
use web_auth_fixture::{WebAuthFixture, WebAuthFixtureClient};

fn input(env: &Env, publisher: &Address, marker: u8) -> EvidenceInput {
    EvidenceInput {
        publisher: publisher.clone(),
        report_hash: BytesN::from_array(env, &[marker; 32]),
        target_hash: BytesN::from_array(env, &[2; 32]),
        suite_hash: BytesN::from_array(env, &[3; 32]),
        specs_hash: BytesN::from_array(env, &[4; 32]),
        artifact_root: BytesN::from_array(env, &[5; 32]),
        network: NetworkKind::Testnet,
        protocol_bitmap: SUPPORTED_PROTOCOL_BITMAP,
        score_bps: 10_000,
        passed: u32::MAX,
        failed: 0,
        warnings: 0,
        skipped: 0,
    }
}

fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EvidenceRegistry, ());
    let admin = Address::generate(&env);
    let publisher = Address::generate(&env);
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    client.initialize(&admin, &SCHEMA_VERSION);
    client.register_attestor(&publisher, &BytesN::from_array(&env, &[9; 32]));
    (env, contract_id, publisher)
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

fn evidence_id(
    env: &Env,
    publisher: &Address,
    report_hash: &BytesN<32>,
    network: &NetworkKind,
) -> BytesN<32> {
    let mut preimage = Bytes::new(env);
    preimage.extend_from_slice(EVIDENCE_ID_DOMAIN);
    preimage.append(&publisher.clone().to_xdr(env));
    preimage.extend_from_slice(&report_hash.to_array());
    preimage.extend_from_slice(&network.discriminant().to_be_bytes());
    BytesN::from_array(env, &env.crypto().sha256(&preimage).to_array())
}

fn load_baselines() -> Value {
    let raw = std::fs::read_to_string("../../tests/resource-budgets/baselines.json")
        .expect("tests/resource-budgets/baselines.json must exist");
    serde_json::from_str(&raw).expect("valid baselines.json")
}

fn setup_web_auth(env: &Env) -> (WebAuthFixtureClient<'_>, Address, Address, Address) {
    let contract_id = env.register(WebAuthFixture, ());
    let client = WebAuthFixtureClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let server = Address::generate(env);
    let account = Address::generate(env);

    client.initialize(&admin, &SCHEMA_VERSION);
    client.configure(&admin, &server, &String::from_str(env, "example.org"));

    (client, admin, server, account)
}

fn build_web_auth_args(
    env: &Env,
    account: &Address,
    server: &Address,
    home_domain: &str,
    web_auth_domain: &str,
    nonce: &str,
    client_domain: Option<(&str, &Address)>,
) -> Map<Symbol, String> {
    let mut args = Map::new(env);
    args.set(Symbol::new(env, "account"), account.to_string());
    args.set(
        Symbol::new(env, "home_domain"),
        String::from_str(env, home_domain),
    );
    args.set(
        Symbol::new(env, "web_auth_domain"),
        String::from_str(env, web_auth_domain),
    );
    args.set(
        Symbol::new(env, "web_auth_domain_account"),
        server.to_string(),
    );
    args.set(Symbol::new(env, "nonce"), String::from_str(env, nonce));

    if let Some((domain, domain_account)) = client_domain {
        args.set(
            Symbol::new(env, "client_domain"),
            String::from_str(env, domain),
        );
        args.set(
            Symbol::new(env, "client_domain_account"),
            domain_account.to_string(),
        );
    }

    args
}

fn assert_web_auth_scenario(
    env: &Env,
    baselines: &Value,
    scenario_name: &str,
    client: &WebAuthFixtureClient<'_>,
    args: &Map<Symbol, String>,
    expected_auth_roots: usize,
) {
    let scenario = &baselines["webAuthVerifyScenarios"][scenario_name];
    let cpu_max = scenario["cpuMax"]
        .as_u64()
        .unwrap_or_else(|| panic!("missing cpuMax for {scenario_name}"));
    let memory_max = scenario["memoryMax"]
        .as_u64()
        .unwrap_or_else(|| panic!("missing memoryMax for {scenario_name}"));
    let writes_max = scenario["storageWritesMax"]
        .as_u64()
        .unwrap_or_else(|| panic!("missing storageWritesMax for {scenario_name}"))
        as u32;
    let events_max = scenario["eventsMax"]
        .as_u64()
        .unwrap_or_else(|| panic!("missing eventsMax for {scenario_name}"))
        as usize;

    env.cost_estimate().budget().reset_tracker();
    client.web_auth_verify(args);

    assert_budget(env, cpu_max, memory_max);

    let resources = env.cost_estimate().resources();
    assert!(
        resources.write_entries <= writes_max,
        "{scenario_name}: write_entries {} exceeds {writes_max}",
        resources.write_entries
    );
    assert!(
        env.events().all().len() as usize <= events_max,
        "{scenario_name}: emitted {} events, max is {events_max}",
        env.events().all().len()
    );
    assert!(
        env.auths().len() >= expected_auth_roots,
        "{scenario_name}: expected at least {expected_auth_roots} auth roots, got {}",
        env.auths().len()
    );

    let events_before_replay = env.events().all().len();
    assert!(
        client.try_web_auth_verify(args).is_err(),
        "{scenario_name}: replay must be rejected"
    );
    assert_eq!(
        env.events().all().len(),
        events_before_replay,
        "{scenario_name}: failed replay emitted ghost events"
    );
}

#[test]
fn maximum_valid_publication_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    env.cost_estimate().budget().reset_tracker();

    client.publish_evidence(&input(&env, &publisher, 1));

    assert_budget(&env, 20_000_000, 16_000_000);
}

#[test]
fn maximum_valid_supersession_walk_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let current = client.publish_evidence(&input(&env, &publisher, 1));
    let replacement = input(&env, &publisher, 2);
    let replacement_id = evidence_id(
        &env,
        &replacement.publisher,
        &replacement.report_hash,
        &replacement.network,
    );
    env.as_contract(&contract_id, || {
        let mut cursor = replacement_id;
        for marker in 1..=24_u8 {
            let next = BytesN::from_array(&env, &[100 + marker; 32]);
            env.storage()
                .persistent()
                .set(&PersistentKeyV1::SupersededBy(cursor), &next);
            cursor = next;
        }
    });
    env.cost_estimate().budget().reset_tracker();

    client.supersede_evidence(&current, &replacement, &publisher);

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 1_250_000);
    assert!(resources.mem_bytes <= 140_000);
    assert!(resources.memory_read_entries <= 40);
    assert!(resources.write_entries <= 7);
}

#[test]
fn long_replacement_chain_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let current = client.publish_evidence(&input(&env, &publisher, 1));
    let replacement = input(&env, &publisher, 2);
    let replacement_id = evidence_id(
        &env,
        &replacement.publisher,
        &replacement.report_hash,
        &replacement.network,
    );
    env.as_contract(&contract_id, || {
        let mut cursor = replacement_id;
        for marker in 1..=24_u8 {
            let next = BytesN::from_array(&env, &[100 + marker; 32]);
            env.storage()
                .persistent()
                .set(&PersistentKeyV1::SupersededBy(cursor), &next);
            cursor = next;
        }
    });
    env.cost_estimate().budget().reset_tracker();

    client.supersede_evidence(&current, &replacement, &publisher);

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 1_250_000);
    assert!(resources.mem_bytes <= 140_000);
    assert!(resources.memory_read_entries <= 40);
    assert!(resources.write_entries <= 7);
}

#[test]
fn revoked_evidence_maintenance_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    client.revoke_evidence(&id, &BytesN::from_array(&env, &[8; 32]), &publisher);

    env.ledger().set_sequence_number(1_500_000);
    client.set_paused(&false);
    env.ledger().set_sequence_number(2_600_000);
    env.cost_estimate().budget().reset_tracker();

    client.maintain_evidence(&id);

    assert_budget(&env, 10_000_000, 8_000_000);
}

#[test]
fn maximum_supersession_chain_maintenance_stays_within_reviewed_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let mut current = client.publish_evidence(&input(&env, &publisher, 1));
    let first = current.clone();
    for marker in 2..=24 {
        current = client.supersede_evidence(&current, &input(&env, &publisher, marker), &publisher);
    }
    env.cost_estimate().budget().reset_tracker();

    client.maintain_supersession_chain(&first);

    assert_budget(&env, 80_000_000, 64_000_000);
}

#[test]
fn successful_evidence_read_extends_only_instance_ttl_within_budget() {
    let (env, contract_id, publisher) = setup();
    let client = EvidenceRegistryClient::new(&env, &contract_id);
    let id = client.publish_evidence(&input(&env, &publisher, 1));
    env.ledger().set_sequence_number(2_000_000);
    let before = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id.clone())),
        )
    });

    assert!(client.get_evidence(&id).is_some());

    let resources = env.cost_estimate().resources();
    assert!(resources.instructions <= 5_000_000);
    assert!(resources.mem_bytes <= 5_000_000);
    assert!(resources.memory_read_entries <= 4);
    assert!(resources.write_entries <= 2);
    let after = env.as_contract(&contract_id, || {
        (
            env.storage().instance().get_ttl(),
            env.storage()
                .persistent()
                .get_ttl(&PersistentKeyV1::Evidence(id)),
        )
    });
    assert!(after.0 > before.0);
    assert_eq!(after.1, before.1);
}

#[test]
fn web_auth_verify_stays_within_resource_budgets() {
    let baselines = load_baselines();

    // 1. Mandatory-only SEP-45 args (account + server auth roots)
    {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, server, account) = setup_web_auth(&env);
        let args = build_web_auth_args(
            &env,
            &account,
            &server,
            "example.org",
            "auth.example.org",
            "nonce-mandatory-0001",
            None,
        );
        assert_web_auth_scenario(&env, &baselines, "mandatory_only", &client, &args, 2);
    }

    // 2. Client-domain authorization case (account + server + client_domain_account)
    {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, server, account) = setup_web_auth(&env);
        let client_domain_account = Address::generate(&env);
        let args = build_web_auth_args(
            &env,
            &account,
            &server,
            "example.org",
            "auth.example.org",
            "nonce-client-domain-0002",
            Some(("wallet.example.org", &client_domain_account)),
        );
        assert_web_auth_scenario(&env, &baselines, "with_client_domain", &client, &args, 3);
    }

    // 3. Boundary-sized inputs with client_domain
    {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _admin, server, account) = setup_web_auth(&env);
        let client_domain_account = Address::generate(&env);
        let max_nonce = "n".repeat(64);
        let max_web_auth_domain = format!("{}.example.org", "a".repeat(48));
        let max_client_domain = format!("{}.wallet.example.org", "b".repeat(40));
        let args = build_web_auth_args(
            &env,
            &account,
            &server,
            "example.org",
            &max_web_auth_domain,
            &max_nonce,
            Some((&max_client_domain, &client_domain_account)),
        );
        assert_web_auth_scenario(&env, &baselines, "boundary_sized", &client, &args, 3);
    }
}