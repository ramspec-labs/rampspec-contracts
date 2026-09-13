#![no_std]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

mod evidence_id;
mod storage;
mod validation;

use rampspec_shared_types::{
    AttestorRecord, ContractError, EvidenceInput, EvidenceRecord, EvidenceStatus, SCHEMA_VERSION,
};
use soroban_sdk::{Address, BytesN, Env, contract, contractevent, contractimpl};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    #[topic]
    pub admin: Address,
    pub schema_version: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorSet {
    #[topic]
    pub attestor: Address,
    pub enabled: bool,
    pub metadata_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidencePublished {
    #[topic]
    pub id: BytesN<32>,
    #[topic]
    pub publisher: Address,
    pub report_hash: BytesN<32>,
    pub target_hash: BytesN<32>,
    pub suite_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRevoked {
    #[topic]
    pub id: BytesN<32>,
    #[topic]
    pub revoker: Address,
    pub reason_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSuperseded {
    #[topic]
    pub old_id: BytesN<32>,
    #[topic]
    pub new_id: BytesN<32>,
    pub authorizer: Address,
}

#[contract]
pub struct EvidenceRegistry;

#[contractimpl]
impl EvidenceRegistry {
    pub fn schema_version(env: Env) -> Result<u32, ContractError> {
        storage::schema_version(&env)
    }

    pub fn initialize(env: Env, admin: Address, schema_version: u32) -> Result<(), ContractError> {
        if storage::is_initialized(&env) {
            return Err(ContractError::AlreadyInitialized);
        }
        if schema_version != SCHEMA_VERSION {
            return Err(ContractError::UpgradeNotAllowed);
        }

        admin.require_auth();
        storage::set_instance_state(&env, &admin, schema_version);
        Initialized {
            admin,
            schema_version,
        }
        .publish(&env);
        Ok(())
    }

    pub fn register_attestor(
        env: Env,
        attestor: Address,
        metadata_hash: BytesN<32>,
    ) -> Result<(), ContractError> {
        storage::admin(&env)?.require_auth();
        if metadata_hash.to_array() == [0; 32] {
            return Err(ContractError::InvalidHash);
        }

        let ledger = env.ledger().sequence();
        let registered_ledger =
            storage::attestor(&env, &attestor).map_or(ledger, |record| record.registered_ledger);
        storage::set_attestor(
            &env,
            &AttestorRecord {
                attestor: attestor.clone(),
                metadata_hash: metadata_hash.clone(),
                enabled: true,
                registered_ledger,
                updated_ledger: ledger,
            },
        );
        AttestorSet {
            attestor,
            enabled: true,
            metadata_hash,
        }
        .publish(&env);
        Ok(())
    }

    pub fn set_attestor_enabled(
        env: Env,
        attestor: Address,
        enabled: bool,
    ) -> Result<(), ContractError> {
        storage::admin(&env)?.require_auth();
        let mut record =
            storage::attestor(&env, &attestor).ok_or(ContractError::AttestorNotRegistered)?;
        if record.enabled == enabled {
            return Ok(());
        }

        record.enabled = enabled;
        record.updated_ledger = env.ledger().sequence();
        storage::set_attestor(&env, &record);
        AttestorSet {
            attestor,
            enabled,
            metadata_hash: record.metadata_hash,
        }
        .publish(&env);
        Ok(())
    }

    pub fn publish_evidence(env: Env, input: EvidenceInput) -> Result<BytesN<32>, ContractError> {
        let schema_version = storage::schema_version(&env)?;
        validation::validate_input(&input, None, schema_version)?;
        let attestor = storage::attestor(&env, &input.publisher)
            .ok_or(ContractError::AttestorNotRegistered)?;
        if !attestor.enabled {
            return Err(ContractError::AttestorDisabled);
        }
        input.publisher.require_auth();

        let id = evidence_id::derive(&env, &input.publisher, &input.report_hash, &input.network);
        if storage::evidence(&env, &id).is_some()
            || storage::active_id(&env, &input.publisher, &input.report_hash, &input.network)
                .is_some()
        {
            return Err(ContractError::EvidenceAlreadyExists);
        }
        let record = EvidenceRecord {
            id: id.clone(),
            publisher: input.publisher.clone(),
            report_hash: input.report_hash.clone(),
            target_hash: input.target_hash.clone(),
            suite_hash: input.suite_hash.clone(),
            specs_hash: input.specs_hash,
            artifact_root: input.artifact_root,
            network: input.network,
            protocol_bitmap: input.protocol_bitmap,
            score_bps: input.score_bps,
            passed: input.passed,
            failed: input.failed,
            warnings: input.warnings,
            skipped: input.skipped,
            created_ledger: env.ledger().sequence(),
            status: EvidenceStatus::Active,
            supersedes: None,
        };
        storage::set_evidence(&env, &record);
        storage::set_active_id(
            &env,
            &record.publisher,
            &record.report_hash,
            &record.network,
            &record.id,
        );
        EvidencePublished {
            id: id.clone(),
            publisher: record.publisher,
            report_hash: record.report_hash,
            target_hash: record.target_hash,
            suite_hash: record.suite_hash,
        }
        .publish(&env);
        Ok(id)
    }

    #[must_use]
    pub fn get_evidence(env: Env, id: BytesN<32>) -> Option<EvidenceRecord> {
        storage::evidence(&env, &id)
    }

    #[must_use]
    pub fn get_attestor(env: Env, attestor: Address) -> Option<AttestorRecord> {
        storage::attestor(&env, &attestor)
    }

    #[must_use]
    pub fn is_active(env: Env, id: BytesN<32>) -> bool {
        storage::evidence(&env, &id).is_some_and(|record| record.status == EvidenceStatus::Active)
    }

    pub fn revoke_evidence(
        env: Env,
        id: BytesN<32>,
        reason_hash: BytesN<32>,
        revoker: Address,
    ) -> Result<(), ContractError> {
        if reason_hash.to_array() == [0; 32] {
            return Err(ContractError::InvalidHash);
        }
        let mut record = storage::evidence(&env, &id).ok_or(ContractError::EvidenceNotFound)?;
        if record.status != EvidenceStatus::Active {
            return Err(ContractError::EvidenceNotActive);
        }
        let admin = storage::admin(&env)?;
        if revoker != record.publisher && revoker != admin {
            return Err(ContractError::Unauthorized);
        }
        revoker.require_auth();

        record.status = EvidenceStatus::Revoked;
        storage::set_evidence(&env, &record);
        EvidenceRevoked {
            id,
            revoker,
            reason_hash,
        }
        .publish(&env);
        Ok(())
    }

    pub fn supersede_evidence(
        env: Env,
        old_id: BytesN<32>,
        replacement: EvidenceInput,
        authorizer: Address,
    ) -> Result<BytesN<32>, ContractError> {
        let schema_version = storage::schema_version(&env)?;
        validation::validate_input(&replacement, Some(&old_id), schema_version)?;
        let mut old = storage::evidence(&env, &old_id).ok_or(ContractError::EvidenceNotFound)?;
        if old.status != EvidenceStatus::Active {
            return Err(ContractError::EvidenceNotActive);
        }
        let admin = storage::admin(&env)?;
        if authorizer != old.publisher && authorizer != admin {
            return Err(ContractError::Unauthorized);
        }
        if authorizer != admin && replacement.publisher != old.publisher {
            return Err(ContractError::InvalidSupersession);
        }
        let replacement_attestor = storage::attestor(&env, &replacement.publisher)
            .ok_or(ContractError::AttestorNotRegistered)?;
        if !replacement_attestor.enabled {
            return Err(ContractError::AttestorDisabled);
        }
        authorizer.require_auth();

        let new_id = evidence_id::derive(
            &env,
            &replacement.publisher,
            &replacement.report_hash,
            &replacement.network,
        );
        if new_id == old_id {
            return Err(ContractError::InvalidSupersession);
        }
        if storage::evidence(&env, &new_id).is_some()
            || storage::active_id(
                &env,
                &replacement.publisher,
                &replacement.report_hash,
                &replacement.network,
            )
            .is_some()
        {
            return Err(ContractError::EvidenceAlreadyExists);
        }

        let new_record = EvidenceRecord {
            id: new_id.clone(),
            publisher: replacement.publisher,
            report_hash: replacement.report_hash,
            target_hash: replacement.target_hash,
            suite_hash: replacement.suite_hash,
            specs_hash: replacement.specs_hash,
            artifact_root: replacement.artifact_root,
            network: replacement.network,
            protocol_bitmap: replacement.protocol_bitmap,
            score_bps: replacement.score_bps,
            passed: replacement.passed,
            failed: replacement.failed,
            warnings: replacement.warnings,
            skipped: replacement.skipped,
            created_ledger: env.ledger().sequence(),
            status: EvidenceStatus::Active,
            supersedes: Some(old_id.clone()),
        };
        old.status = EvidenceStatus::Superseded;
        storage::set_evidence(&env, &old);
        storage::set_evidence(&env, &new_record);
        storage::remove_active_id(&env, &old.publisher, &old.report_hash, &old.network);
        storage::set_active_id(
            &env,
            &new_record.publisher,
            &new_record.report_hash,
            &new_record.network,
            &new_id,
        );
        storage::set_superseded_by(&env, &old_id, &new_id);
        EvidenceSuperseded {
            old_id,
            new_id: new_id.clone(),
            authorizer,
        }
        .publish(&env);
        Ok(new_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rampspec_shared_types::{InstanceKeyV1, NetworkKind};
    use soroban_sdk::{
        Address, testutils::Address as _, testutils::Events as _, testutils::Ledger as _,
    };

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EvidenceRegistry, ());
        let admin = Address::generate(&env);
        (env, contract_id, admin)
    }

    fn valid_input(env: &Env, publisher: &Address) -> EvidenceInput {
        EvidenceInput {
            publisher: publisher.clone(),
            report_hash: BytesN::from_array(env, &[11; 32]),
            target_hash: BytesN::from_array(env, &[12; 32]),
            suite_hash: BytesN::from_array(env, &[13; 32]),
            specs_hash: BytesN::from_array(env, &[14; 32]),
            artifact_root: BytesN::from_array(env, &[15; 32]),
            network: NetworkKind::Testnet,
            protocol_bitmap: 1,
            score_bps: 10_000,
            passed: 1,
            failed: 0,
            warnings: 0,
            skipped: 0,
        }
    }

    fn register_publisher(env: &Env, contract_id: &Address, admin: &Address) -> Address {
        let client = EvidenceRegistryClient::new(env, contract_id);
        client.initialize(admin, &SCHEMA_VERSION);
        let publisher = Address::generate(env);
        client.register_attestor(&publisher, &BytesN::from_array(env, &[9; 32]));
        publisher
    }

    #[test]
    fn initializes_exactly_once() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        assert_eq!(client.schema_version(), SCHEMA_VERSION);
        assert_eq!(
            client.try_initialize(&admin, &SCHEMA_VERSION),
            Err(Ok(ContractError::AlreadyInitialized))
        );
    }

    #[test]
    fn rejects_an_unsupported_schema() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        assert_eq!(
            client.try_initialize(&admin, &(SCHEMA_VERSION + 1)),
            Err(Ok(ContractError::UpgradeNotAllowed))
        );
    }

    #[test]
    fn reads_require_initialization() {
        let (env, contract_id, _admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        assert_eq!(
            client.try_schema_version(),
            Err(Ok(ContractError::NotInitialized))
        );
    }

    #[test]
    fn registers_and_updates_an_attestor() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let attestor = Address::generate(&env);
        let first_hash = BytesN::from_array(&env, &[1; 32]);
        let second_hash = BytesN::from_array(&env, &[2; 32]);

        client.register_attestor(&attestor, &first_hash);
        let first = env.as_contract(&contract_id, || storage::attestor(&env, &attestor).unwrap());
        env.ledger()
            .set_sequence_number(first.registered_ledger + 1);
        client.register_attestor(&attestor, &second_hash);
        let updated = env.as_contract(&contract_id, || storage::attestor(&env, &attestor).unwrap());

        assert_eq!(updated.metadata_hash, second_hash);
        assert_eq!(updated.registered_ledger, first.registered_ledger);
        assert!(updated.updated_ledger > updated.registered_ledger);
        assert!(updated.enabled);
    }

    #[test]
    fn rejects_an_empty_attestor_commitment() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let attestor = Address::generate(&env);
        let empty_hash = BytesN::from_array(&env, &[0; 32]);
        assert_eq!(
            client.try_register_attestor(&attestor, &empty_hash),
            Err(Ok(ContractError::InvalidHash))
        );
    }

    #[test]
    fn registration_requires_admin_auth() {
        let env = Env::default();
        let contract_id = env.register(EvidenceRegistry, ());
        let admin = Address::generate(&env);
        env.mock_all_auths();
        EvidenceRegistryClient::new(&env, &contract_id).initialize(&admin, &SCHEMA_VERSION);
        env.set_auths(&[]);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let attestor = Address::generate(&env);
        let metadata_hash = BytesN::from_array(&env, &[1; 32]);
        assert!(
            client
                .try_register_attestor(&attestor, &metadata_hash)
                .is_err()
        );
    }

    #[test]
    fn disables_and_reenables_an_attestor_without_losing_metadata() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let attestor = Address::generate(&env);
        let metadata_hash = BytesN::from_array(&env, &[3; 32]);
        client.register_attestor(&attestor, &metadata_hash);

        client.set_attestor_enabled(&attestor, &false);
        client.set_attestor_enabled(&attestor, &false);
        let disabled =
            env.as_contract(&contract_id, || storage::attestor(&env, &attestor).unwrap());
        assert!(!disabled.enabled);
        assert_eq!(disabled.metadata_hash, metadata_hash);

        client.set_attestor_enabled(&attestor, &true);
        let enabled = env.as_contract(&contract_id, || storage::attestor(&env, &attestor).unwrap());
        assert!(enabled.enabled);
        assert_eq!(enabled.metadata_hash, metadata_hash);
    }

    #[test]
    fn cannot_toggle_an_unknown_attestor() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let attestor = Address::generate(&env);
        assert_eq!(
            client.try_set_attestor_enabled(&attestor, &false),
            Err(Ok(ContractError::AttestorNotRegistered))
        );
    }

    #[test]
    fn toggling_requires_admin_auth() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let attestor = Address::generate(&env);
        let metadata_hash = BytesN::from_array(&env, &[4; 32]);
        client.register_attestor(&attestor, &metadata_hash);
        env.set_auths(&[]);
        assert!(client.try_set_attestor_enabled(&attestor, &false).is_err());
    }

    #[test]
    fn publishes_an_active_immutable_record() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let input = valid_input(&env, &publisher);
        let id = EvidenceRegistryClient::new(&env, &contract_id).publish_evidence(&input);
        assert_eq!(
            env.events()
                .all()
                .filter_by_contract(&contract_id)
                .events()
                .len(),
            1
        );
        let stored = env.as_contract(&contract_id, || storage::evidence(&env, &id).unwrap());

        assert_eq!(stored.id, id);
        assert_eq!(stored.publisher, publisher);
        assert_eq!(stored.report_hash, input.report_hash);
        assert_eq!(stored.status, EvidenceStatus::Active);
        assert_eq!(stored.supersedes, None);
    }

    #[test]
    fn publishing_requires_an_enabled_registered_attestor() {
        let (env, contract_id, admin) = setup();
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        client.initialize(&admin, &SCHEMA_VERSION);
        let missing = Address::generate(&env);
        assert_eq!(
            client.try_publish_evidence(&valid_input(&env, &missing)),
            Err(Ok(ContractError::AttestorNotRegistered))
        );

        client.register_attestor(&missing, &BytesN::from_array(&env, &[9; 32]));
        client.set_attestor_enabled(&missing, &false);
        assert_eq!(
            client.try_publish_evidence(&valid_input(&env, &missing)),
            Err(Ok(ContractError::AttestorDisabled))
        );
    }

    #[test]
    fn publishing_rejects_invalid_input_without_an_event() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let mut input = valid_input(&env, &publisher);
        input.score_bps = 10_001;
        assert_eq!(
            client.try_publish_evidence(&input),
            Err(Ok(ContractError::InvalidScore))
        );
        assert_eq!(env.events().all().events().len(), 0);
    }

    #[test]
    fn publishing_requires_publisher_auth() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        env.set_auths(&[]);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        assert!(
            client
                .try_publish_evidence(&valid_input(&env, &publisher))
                .is_err()
        );
    }

    #[test]
    fn duplicate_retries_preserve_the_original_record() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let input = valid_input(&env, &publisher);
        let id = client.publish_evidence(&input);

        for replacement_byte in 20..25 {
            let mut duplicate = input.clone();
            duplicate.target_hash = BytesN::from_array(&env, &[replacement_byte; 32]);
            assert_eq!(
                client.try_publish_evidence(&duplicate),
                Err(Ok(ContractError::EvidenceAlreadyExists))
            );
            assert_eq!(env.events().all().events().len(), 0);
        }

        let stored = env.as_contract(&contract_id, || storage::evidence(&env, &id).unwrap());
        let active = env.as_contract(&contract_id, || {
            storage::active_id(&env, &publisher, &input.report_hash, &input.network).unwrap()
        });
        assert_eq!(active, id);
        assert_eq!(stored.target_hash, input.target_hash);
    }

    #[test]
    fn reads_expose_active_and_absent_state_without_auth() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let id = client.publish_evidence(&valid_input(&env, &publisher));
        env.set_auths(&[]);

        assert_eq!(client.get_evidence(&id).unwrap().publisher, publisher);
        assert!(client.is_active(&id));
        assert!(client.get_attestor(&publisher).unwrap().enabled);
        let missing = BytesN::from_array(&env, &[99; 32]);
        assert_eq!(client.get_evidence(&missing), None);
        assert!(!client.is_active(&missing));
    }

    #[test]
    fn reads_remain_available_for_disabled_attestors_while_paused() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let id = client.publish_evidence(&valid_input(&env, &publisher));
        client.set_attestor_enabled(&publisher, &false);
        env.as_contract(&contract_id, || {
            env.storage().instance().set(&InstanceKeyV1::Paused, &true);
        });
        env.set_auths(&[]);

        assert_eq!(
            client.get_evidence(&id).unwrap().status,
            EvidenceStatus::Active
        );
        assert!(!client.get_attestor(&publisher).unwrap().enabled);
        assert!(client.is_active(&id));
        assert_eq!(env.events().all().events().len(), 0);
    }

    #[test]
    fn publisher_and_admin_can_irreversibly_revoke() {
        for use_admin in [false, true] {
            let (env, contract_id, admin) = setup();
            let publisher = register_publisher(&env, &contract_id, &admin);
            let client = EvidenceRegistryClient::new(&env, &contract_id);
            let id = client.publish_evidence(&valid_input(&env, &publisher));
            let revoker = if use_admin {
                admin.clone()
            } else {
                publisher.clone()
            };
            let reason = BytesN::from_array(&env, &[31; 32]);
            client.revoke_evidence(&id, &reason, &revoker);

            assert!(!client.is_active(&id));
            assert_eq!(
                client.get_evidence(&id).unwrap().status,
                EvidenceStatus::Revoked
            );
            assert_eq!(
                client.try_revoke_evidence(&id, &reason, &revoker),
                Err(Ok(ContractError::EvidenceNotActive))
            );
        }
    }

    #[test]
    fn revocation_rejects_missing_invalid_and_unauthorized_requests() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let id = client.publish_evidence(&valid_input(&env, &publisher));
        let reason = BytesN::from_array(&env, &[31; 32]);
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_revoke_evidence(&BytesN::from_array(&env, &[90; 32]), &reason, &admin),
            Err(Ok(ContractError::EvidenceNotFound))
        );
        assert_eq!(
            client.try_revoke_evidence(&id, &BytesN::from_array(&env, &[0; 32]), &admin),
            Err(Ok(ContractError::InvalidHash))
        );
        assert_eq!(
            client.try_revoke_evidence(&id, &reason, &stranger),
            Err(Ok(ContractError::Unauthorized))
        );
        assert!(client.is_active(&id));
    }

    #[test]
    fn superseded_evidence_cannot_be_revoked_again() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let id = client.publish_evidence(&valid_input(&env, &publisher));
        env.as_contract(&contract_id, || {
            let mut record = storage::evidence(&env, &id).unwrap();
            record.status = EvidenceStatus::Superseded;
            storage::set_evidence(&env, &record);
        });
        assert_eq!(
            client.try_revoke_evidence(&id, &BytesN::from_array(&env, &[31; 32]), &publisher),
            Err(Ok(ContractError::EvidenceNotActive))
        );
    }

    #[test]
    fn supersession_atomically_links_old_and_new_records() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let old_id = client.publish_evidence(&valid_input(&env, &publisher));
        let mut replacement = valid_input(&env, &publisher);
        replacement.report_hash = BytesN::from_array(&env, &[42; 32]);
        let new_id = client.supersede_evidence(&old_id, &replacement, &publisher);

        assert_eq!(
            client.get_evidence(&old_id).unwrap().status,
            EvidenceStatus::Superseded
        );
        let new_record = client.get_evidence(&new_id).unwrap();
        assert_eq!(new_record.status, EvidenceStatus::Active);
        assert_eq!(new_record.supersedes, Some(old_id.clone()));
        assert_eq!(
            env.as_contract(&contract_id, || storage::superseded_by(&env, &old_id)),
            Some(new_id)
        );
    }

    #[test]
    fn supersession_rejects_invalid_ownership_state_and_duplicates() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let old_id = client.publish_evidence(&valid_input(&env, &publisher));
        let mut replacement = valid_input(&env, &publisher);
        replacement.report_hash = BytesN::from_array(&env, &[43; 32]);
        let stranger = Address::generate(&env);
        assert_eq!(
            client.try_supersede_evidence(&old_id, &replacement, &stranger),
            Err(Ok(ContractError::Unauthorized))
        );
        assert!(client.is_active(&old_id));

        let duplicate_id = client.publish_evidence(&replacement);
        assert_eq!(
            client.try_supersede_evidence(&old_id, &replacement, &publisher),
            Err(Ok(ContractError::EvidenceAlreadyExists))
        );
        assert!(client.is_active(&old_id));
        assert!(client.is_active(&duplicate_id));

        client.revoke_evidence(&old_id, &BytesN::from_array(&env, &[31; 32]), &publisher);
        let mut another = replacement;
        another.report_hash = BytesN::from_array(&env, &[44; 32]);
        assert_eq!(
            client.try_supersede_evidence(&old_id, &another, &publisher),
            Err(Ok(ContractError::EvidenceNotActive))
        );
    }

    #[test]
    fn only_admin_may_move_a_chain_to_another_attestor() {
        let (env, contract_id, admin) = setup();
        let publisher = register_publisher(&env, &contract_id, &admin);
        let client = EvidenceRegistryClient::new(&env, &contract_id);
        let old_id = client.publish_evidence(&valid_input(&env, &publisher));
        let second_attestor = Address::generate(&env);
        client.register_attestor(&second_attestor, &BytesN::from_array(&env, &[8; 32]));
        let mut replacement = valid_input(&env, &second_attestor);
        replacement.report_hash = BytesN::from_array(&env, &[45; 32]);

        assert_eq!(
            client.try_supersede_evidence(&old_id, &replacement, &publisher),
            Err(Ok(ContractError::InvalidSupersession))
        );
        let new_id = client.supersede_evidence(&old_id, &replacement, &admin);
        assert_eq!(
            client.get_evidence(&new_id).unwrap().publisher,
            second_attestor
        );
    }
}
