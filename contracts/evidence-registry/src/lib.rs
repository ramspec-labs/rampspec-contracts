#![no_std]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

mod storage;

use rampspec_shared_types::{ContractError, SCHEMA_VERSION};
use soroban_sdk::{Address, Env, contract, contractevent, contractimpl};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Initialized {
    #[topic]
    pub admin: Address,
    pub schema_version: u32,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Address, testutils::Address as _};

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
}
