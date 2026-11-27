use evidence_registry::{
    AdminChanged, AdminProposalCancelled, AdminProposed, AttestorSet, EvidencePublished,
    EvidenceRevoked, EvidenceSuperseded, Initialized, PauseChanged, Upgraded,
};
use soroban_sdk::{
    Address, BytesN, Env, FromVal, Map, Symbol, TryFromVal, Val, events::Event,
    testutils::Address as _,
};

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

fn assert_only_allowed_data_fields<E: Event>(env: &Env, event: &E, allowed_fields: &[&str]) {
    let data = Map::<Symbol, Val>::try_from_val(env, &event.data(env)).unwrap();
    for key in data.keys() {
        let field_name = key.to_string();
        assert!(
            allowed_fields.contains(&field_name.as_str()),
            "Unexpected field '{}' in event data. Only allowed: {:?}",
            field_name,
            allowed_fields
        );
    }
}

const PROHIBITED_FIELDS: &[&str] = &[
    "report",
    "report_body",
    "report_content",
    "report_data",
    "url",
    "domain",
    "api_key",
    "password",
    "token",
    "access_token",
    "credential",
    "secret",
    "customer_id",
    "customer_name",
    "customer_email",
    "customer_data",
    "kyc_data",
    "personal_information",
    "pii",
];

fn assert_no_prohibited_fields<E: Event>(env: &Env, event: &E) {
    let data = Map::<Symbol, Val>::try_from_val(env, &event.data(env)).unwrap();
    for key in data.keys() {
        let field_name = key.to_string();
        for prohibited in PROHIBITED_FIELDS {
            assert!(
                !field_name.contains(prohibited),
                "Event contains prohibited field pattern '{}' in field '{}'",
                prohibited,
                field_name
            );
        }
    }
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
    assert_only_allowed_data_fields(
        &env,
        &Initialized {
            admin: admin.clone(),
            schema_version: 1,
        },
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
    assert_only_allowed_data_fields(
        &env,
        &PauseChanged {
            admin: admin.clone(),
            paused: true,
        },
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
    assert_only_allowed_data_fields(
        &env,
        &AdminProposed {
            current_admin: admin.clone(),
            pending_admin: next_admin.clone(),
        },
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
    assert_only_allowed_data_fields(
        &env,
        &AdminChanged {
            old_admin: admin.clone(),
            new_admin: next_admin.clone(),
        },
        &[],
    );

    assert_schema(
        &env,
        &AdminProposalCancelled {
            current_admin: admin.clone(),
            cancelled_admin: next_admin.clone(),
        },
        "admin_proposal_cancelled",
        3,
        &[],
    );
    assert_only_allowed_data_fields(
        &env,
        &AdminProposalCancelled {
            current_admin: admin,
            cancelled_admin: next_admin,
        },
        &[],
    );

    assert_schema(
        &env,
        &Upgraded {
            old_wasm_hash: BytesN::from_array(&env, &[0; 32]),
            new_wasm_hash: hash.clone(),
        },
        "upgraded",
        2,
        &["old_wasm_hash"],
    );
    assert_only_allowed_data_fields(
        &env,
        &Upgraded {
            old_wasm_hash: BytesN::from_array(&env, &[0; 32]),
            new_wasm_hash: hash,
        },
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
    assert_only_allowed_data_fields(
        &env,
        &AttestorSet {
            attestor: publisher.clone(),
            enabled: true,
            metadata_hash: hash.clone(),
        },
        &["enabled", "metadata_hash"],
    );

    let evidence_published = EvidencePublished {
        id: id.clone(),
        publisher: publisher.clone(),
        report_hash: hash.clone(),
        target_hash: hash.clone(),
        suite_hash: hash.clone(),
    };
    assert_schema(
        &env,
        &evidence_published,
        "evidence_published",
        3,
        &["report_hash", "target_hash", "suite_hash"],
    );
    assert_only_allowed_data_fields(
        &env,
        &evidence_published,
        &["report_hash", "target_hash", "suite_hash"],
    );
    assert_no_prohibited_fields(&env, &evidence_published);

    let evidence_revoked = EvidenceRevoked {
        id: id.clone(),
        revoker: publisher.clone(),
        reason_hash: hash.clone(),
    };
    assert_schema(
        &env,
        &evidence_revoked,
        "evidence_revoked",
        3,
        &["reason_hash"],
    );
    assert_only_allowed_data_fields(&env, &evidence_revoked, &["reason_hash"]);
    assert_no_prohibited_fields(&env, &evidence_revoked);

    let evidence_superseded = EvidenceSuperseded {
        old_id: id,
        new_id: next_id,
        authorizer: publisher,
    };
    assert_schema(
        &env,
        &evidence_superseded,
        "evidence_superseded",
        3,
        &["authorizer"],
    );
    assert_only_allowed_data_fields(&env, &evidence_superseded, &["authorizer"]);
    assert_no_prohibited_fields(&env, &evidence_superseded);
}

#[test]
fn events_contain_only_approved_public_information() {
    let env = Env::default();
    let publisher = Address::generate(&env);
    let id = BytesN::from_array(&env, &[1; 32]);
    let hash = BytesN::from_array(&env, &[3; 32]);

    let evidence_published = EvidencePublished {
        id: id.clone(),
        publisher: publisher.clone(),
        report_hash: hash.clone(),
        target_hash: hash.clone(),
        suite_hash: hash.clone(),
    };
    assert_no_prohibited_fields(&env, &evidence_published);

    let evidence_revoked = EvidenceRevoked {
        id,
        revoker: publisher,
        reason_hash: hash,
    };
    assert_no_prohibited_fields(&env, &evidence_revoked);
}
