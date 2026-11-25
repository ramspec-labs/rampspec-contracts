use evidence_registry::{
    AdminChanged, AdminProposalCancelled, AdminProposed, AttestorSet, EvidencePublished,
    EvidenceRevoked, EvidenceSuperseded, Initialized, PauseChanged, Upgraded,
};
use soroban_sdk::{
    Address, BytesN, Env, FromVal, Map, Symbol, TryFromVal, Val, events::Event,
    testutils::Address as _,
};


use std::collections::{BTreeMap, BTreeSet};
use rampspec_shared_types::RegistryError;
use serde_json::Value;

fn assert_schema<E: Event>(
    env: &Env,
    event: &E,
    name: &str,
    topic_count: u32,
    data_fields: &[&str],
) {
    let topics = event.topics(env);
    assert_eq!(topics.len(), topic_count);
    assert_eq!(
        Symbol::from_val(env, &topics.get(0).unwrap()),
        Symbol::new(env, name)
    );
    let data = Map::<Symbol, Val>::try_from_val(env, &event.data(env)).unwrap();
    assert_eq!(data.len(), u32::try_from(data_fields.len()).unwrap());
    for field in data_fields {
        assert!(data.contains_key(Symbol::new(env, field)));
    }
}

#[test]
fn registry_error_discriminants_match_canonical_spec_snapshot() {
    let raw = std::fs::read_to_string("../../artifacts/specs/evidence_registry.json")
        .expect("artifacts/specs/evidence_registry.json must exist");
    let spec: Value = serde_json::from_str(&raw).expect("valid evidence_registry.json");

    // Extract RegistryError enum cases from the generated contract spec
    let spec_errors: BTreeMap<String, u32> = spec["errors"]
        .as_array()
        .or_else(|| spec["types"].as_array())
        .expect("spec must contain errors/types array")
        .iter()
        .find(|entry| entry["name"].as_str() == Some("RegistryError"))
        .and_then(|entry| entry["cases"].as_array())
        .expect("RegistryError cases must be present in evidence_registry.json")
        .iter()
        .map(|case| {
            let name = case["name"].as_str().expect("error case name").to_string();
            let value = case["value"].as_u64().expect("error case u32 value") as u32;
            (name, value)
        })
        .collect();

    let mut seen_codes = BTreeSet::new();
    let mut rust_errors = BTreeMap::new();
    for &(name, code) in RegistryError::SNAPSHOT_DISCRIMINANTS {
        assert!(
            seen_codes.insert(code),
            "duplicate RegistryError discriminant {code} for {name}"
        );
        rust_errors.insert(name.to_string(), code);
    }

    assert_eq!(
        rust_errors, spec_errors,
        "RegistryError discriminants drifted from artifacts/specs/evidence_registry.json"
    );
}

#[test]
fn administrative_event_schemas_are_stable() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let next_admin = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[1; 32]);

    assert_schema(
        &env,
        &Initialized {
            admin: admin.clone(),
            schema_version: 1,
        },
        "initialized",
        2,
        &["schema_version"],
    );
    assert_schema(
        &env,
        &PauseChanged {
            admin: admin.clone(),
            paused: true,
        },
        "pause_changed",
        2,
        &["paused"],
    );
    assert_schema(
        &env,
        &AdminProposed {
            current_admin: admin.clone(),
            pending_admin: next_admin.clone(),
        },
        "admin_proposed",
        3,
        &[],
    );
    assert_schema(
        &env,
        &AdminChanged {
            old_admin: admin.clone(),
            new_admin: next_admin.clone(),
        },
        "admin_changed",
        3,
        &[],
    );
    assert_schema(
        &env,
        &AdminProposalCancelled {
            current_admin: admin,
            cancelled_admin: next_admin,
        },
        "admin_proposal_cancelled",
        3,
        &[],
    );
    assert_schema(
        &env,
        &Upgraded {
            old_wasm_hash: BytesN::from_array(&env, &[0; 32]),
            new_wasm_hash: hash,
        },
        "upgraded",
        2,
        &["old_wasm_hash"],
    );
}

#[test]
fn evidence_event_schemas_are_stable() {
    let env = Env::default();
    let publisher = Address::generate(&env);
    let id = BytesN::from_array(&env, &[1; 32]);
    let next_id = BytesN::from_array(&env, &[2; 32]);
    let hash = BytesN::from_array(&env, &[3; 32]);

    assert_schema(
        &env,
        &AttestorSet {
            attestor: publisher.clone(),
            enabled: true,
            metadata_hash: hash.clone(),
        },
        "attestor_set",
        2,
        &["enabled", "metadata_hash"],
    );
    assert_schema(
        &env,
        &EvidencePublished {
            id: id.clone(),
            publisher: publisher.clone(),
            report_hash: hash.clone(),
            target_hash: hash.clone(),
            suite_hash: hash.clone(),
        },
        "evidence_published",
        3,
        &["report_hash", "target_hash", "suite_hash"],
    );
    assert_schema(
        &env,
        &EvidenceRevoked {
            id: id.clone(),
            revoker: publisher.clone(),
            reason_hash: hash,
        },
        "evidence_revoked",
        3,
        &["reason_hash"],
    );
    assert_schema(
        &env,
        &EvidenceSuperseded {
            old_id: id,
            new_id: next_id,
            authorizer: publisher,
        },
        "evidence_superseded",
        3,
        &["authorizer"],
    );
}
