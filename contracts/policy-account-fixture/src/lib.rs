#![no_std]
#![allow(
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::used_underscore_binding
)]

use soroban_sdk::{
    Bytes, BytesN, Env, Vec,
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
};

const MAX_SIGNERS: u32 = 16;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PolicyError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InvalidPolicy = 3,
    MissingSignature = 4,
    DuplicateSigner = 5,
    UnauthorizedSigner = 6,
    InsufficientWeight = 7,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ed25519Signer {
    pub public_key: BytesN<32>,
    pub weight: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ed25519Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyConfig {
    pub admin: soroban_sdk::Address,
    pub signers: Vec<Ed25519Signer>,
    pub threshold: u32,
}

#[contracttype]
enum DataKey {
    Config,
}

#[contract]
pub struct PolicyAccountFixture;

fn validate_policy(signers: &Vec<Ed25519Signer>, threshold: u32) -> Result<(), PolicyError> {
    if signers.is_empty() || signers.len() > MAX_SIGNERS || threshold == 0 {
        return Err(PolicyError::InvalidPolicy);
    }
    let mut total_weight = 0_u32;
    let mut seen = Vec::<BytesN<32>>::new(signers.env());
    for signer in signers.iter() {
        if signer.weight == 0 || seen.contains(&signer.public_key) {
            return Err(PolicyError::InvalidPolicy);
        }
        seen.push_back(signer.public_key);
        total_weight = total_weight
            .checked_add(signer.weight)
            .ok_or(PolicyError::InvalidPolicy)?;
    }
    if threshold > total_weight {
        return Err(PolicyError::InvalidPolicy);
    }
    Ok(())
}

fn read_config(env: &Env) -> Result<PolicyConfig, PolicyError> {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(PolicyError::NotInitialized)
}

fn signer_weight(config: &PolicyConfig, public_key: &BytesN<32>) -> Option<u32> {
    config
        .signers
        .iter()
        .find(|signer| signer.public_key == *public_key)
        .map(|signer| signer.weight)
}

#[contractimpl]
impl PolicyAccountFixture {
    pub fn initialize(
        env: Env,
        admin: soroban_sdk::Address,
        signers: Vec<Ed25519Signer>,
        threshold: u32,
    ) -> Result<(), PolicyError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(PolicyError::AlreadyInitialized);
        }
        validate_policy(&signers, threshold)?;
        admin.require_auth();
        env.storage().instance().set(
            &DataKey::Config,
            &PolicyConfig {
                admin,
                signers,
                threshold,
            },
        );
        Ok(())
    }

    pub fn set_signers(
        env: Env,
        signers: Vec<Ed25519Signer>,
        threshold: u32,
    ) -> Result<(), PolicyError> {
        validate_policy(&signers, threshold)?;
        let mut config = read_config(&env)?;
        config.admin.require_auth();
        config.signers = signers;
        config.threshold = threshold;
        env.storage().instance().set(&DataKey::Config, &config);
        Ok(())
    }

    pub fn config(env: Env) -> Result<PolicyConfig, PolicyError> {
        read_config(&env)
    }

    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }

    #[must_use]
    pub fn is_test_only(_env: Env) -> bool {
        true
    }
}

#[contractimpl]
impl CustomAccountInterface for PolicyAccountFixture {
    type Error = PolicyError;
    type Signature = Vec<Ed25519Signature>;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Vec<Ed25519Signature>,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), PolicyError> {
        let config = read_config(&env)?;
        if signatures.is_empty() {
            return Err(PolicyError::MissingSignature);
        }

        let mut seen = Vec::<BytesN<32>>::new(&env);
        let mut accepted_weight = 0_u32;
        let message = Bytes::from_array(&env, &signature_payload.to_bytes().to_array());
        for signature in signatures.iter() {
            if seen.contains(&signature.public_key) {
                return Err(PolicyError::DuplicateSigner);
            }
            seen.push_back(signature.public_key.clone());
            let weight = signer_weight(&config, &signature.public_key)
                .ok_or(PolicyError::UnauthorizedSigner)?;
            env.crypto()
                .ed25519_verify(&signature.public_key, &message, &signature.signature);
            accepted_weight = accepted_weight
                .checked_add(weight)
                .ok_or(PolicyError::InsufficientWeight)?;
        }

        if accepted_weight < config.threshold {
            return Err(PolicyError::InsufficientWeight);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        Ed25519Signature, Ed25519Signer, PolicyAccountFixture, PolicyAccountFixtureClient,
        PolicyError,
    };
    use ed25519_dalek::{Signer as _, SigningKey};
    use soroban_sdk::{Address, BytesN, Env, IntoVal, Vec, testutils::Address as _, vec};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signer(env: &Env, key: &SigningKey, weight: u32) -> Ed25519Signer {
        Ed25519Signer {
            public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
            weight,
        }
    }

    fn signature(env: &Env, key: &SigningKey, payload: &[u8; 32]) -> Ed25519Signature {
        Ed25519Signature {
            public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
            signature: BytesN::from_array(env, &key.sign(payload).to_bytes()),
        }
    }

    fn check(
        env: &Env,
        contract_id: &Address,
        payload: &[u8; 32],
        signatures: &Vec<Ed25519Signature>,
    ) -> Result<(), Result<PolicyError, soroban_sdk::InvokeError>> {
        env.try_invoke_contract_check_auth::<PolicyError>(
            contract_id,
            &BytesN::from_array(env, payload),
            signatures.clone().into_val(env),
            &vec![env],
        )
    }

    #[test]
    fn single_signer_accepts_valid_signature() {
        let env = Env::default();
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);
        let payload = [9; 32];

        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, signature(&env, &first, &payload)]
            ),
            Ok(())
        );
    }

    #[test]
    fn threshold_requires_enough_distinct_authorized_weight() {
        let env = Env::default();
        let first = key(1);
        let second = key(2);
        let signers = vec![&env, signer(&env, &first, 1), signer(&env, &second, 2)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &3);
        let payload = [8; 32];
        let first_signature = signature(&env, &first, &payload);

        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, first_signature.clone()]
            ),
            Err(Ok(PolicyError::InsufficientWeight))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, first_signature, signature(&env, &second, &payload)]
            ),
            Ok(())
        );
    }

    #[test]
    fn missing_duplicate_and_unknown_signers_fail_closed() {
        let env = Env::default();
        let first = key(1);
        let unknown = key(3);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);
        let payload = [7; 32];
        let first_signature = signature(&env, &first, &payload);

        assert_eq!(
            check(&env, &contract_id, &payload, &Vec::new(&env)),
            Err(Ok(PolicyError::MissingSignature))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, first_signature.clone(), first_signature]
            ),
            Err(Ok(PolicyError::DuplicateSigner))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, signature(&env, &unknown, &payload)]
            ),
            Err(Ok(PolicyError::UnauthorizedSigner))
        );
    }

    #[test]
    fn invalid_signature_is_rejected_by_host_crypto() {
        let env = Env::default();
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);
        let payload = [6; 32];
        let signature_for_other_payload = signature(&env, &first, &[5; 32]);

        assert!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, signature_for_other_payload]
            )
            .is_err()
        );
    }
}
