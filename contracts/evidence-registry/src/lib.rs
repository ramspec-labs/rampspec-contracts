#![no_std]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

mod storage;

use rampspec_shared_types::{AttestorRecord, ContractError, SCHEMA_VERSION};
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Address, testutils::Address as _, testutils::Ledger as _};

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(EvidenceRegistry, ());
        let admin = Address::generate(&env);
        (env, contract_id, admin)
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
}
