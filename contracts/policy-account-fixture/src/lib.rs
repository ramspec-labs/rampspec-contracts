#![no_std]
#![allow(
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::used_underscore_binding,
    clippy::large_enum_variant
)]

use p256::{
    AffinePoint, EncodedPoint, ecdsa::Signature as P256Signature,
    elliptic_curve::sec1::FromEncodedPoint,
};
use soroban_sdk::{
    Address, Bytes, BytesN, Env, Symbol, Vec,
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractevent, contractimpl, contracttype,
    crypto::Hash,
    xdr::ToXdr,
};

const MAX_SIGNERS: u32 = 16;
const MIN_AUTHENTICATOR_DATA_LENGTH: u32 = 37;
const MAX_AUTHENTICATOR_DATA_LENGTH: u32 = 512;

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
    InvalidTimeWindow = 8,
    NotYetValid = 9,
    Expired = 10,
    Rejected = 11,
    Replay = 12,
    RequiredSignerMissing = 13,
    InvalidContext = 14,
    WrongSignatureMode = 15,
    InvalidPasskey = 16,
    TooManySignatures = 17,
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
pub struct PasskeySigner {
    pub public_key: BytesN<65>,
    pub weight: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasskeySignature {
    pub public_key: BytesN<65>,
    pub authenticator_data: Bytes,
    pub client_data_hash: BytesN<32>,
    pub challenge: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccountSignature {
    Ed25519(Ed25519Signature),
    Passkey(PasskeySignature),
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignatureMode {
    Ed25519,
    PasskeyCompatible,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyConfig {
    pub admin: Address,
    pub signers: Vec<Ed25519Signer>,
    pub passkey_signers: Vec<PasskeySigner>,
    pub threshold: u32,
    pub signature_mode: SignatureMode,
    pub controls: PolicyControls,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyControls {
    pub valid_from_ledger: u32,
    pub expires_at_ledger: u32,
    pub reject_all: bool,
    pub additional_signer: Option<BytesN<32>>,
    pub required_contract: Option<Address>,
    pub required_function: Option<Symbol>,
}

#[contracttype]
enum DataKey {
    Config,
    ConfigVersion,
    UsedPayload(BytesN<32>),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyConfigurationChanged {
    #[topic]
    pub change: Symbol,
    pub version: u32,
    pub policy_commitment: BytesN<32>,
    pub signature_mode: SignatureMode,
    pub signer_count: u32,
    pub threshold: u32,
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

fn validate_passkey_policy(
    signers: &Vec<PasskeySigner>,
    threshold: u32,
) -> Result<(), PolicyError> {
    if signers.is_empty() || signers.len() > MAX_SIGNERS || threshold == 0 {
        return Err(PolicyError::InvalidPolicy);
    }
    let mut total_weight = 0_u32;
    let mut seen = Vec::<BytesN<65>>::new(signers.env());
    for signer in signers.iter() {
        let encoded = EncodedPoint::from_bytes(signer.public_key.to_array());
        let valid_point = encoded
            .ok()
            .is_some_and(|point| bool::from(AffinePoint::from_encoded_point(&point).is_some()));
        if signer.weight == 0 || !valid_point || seen.contains(&signer.public_key) {
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

/// Ed25519 signers in canonical order: ascending by public key bytes. Equivalent policies listed
/// in any order then store the same configuration, so they share one commitment and a reordered
/// update is a no-op. Keys are unique by the time this runs (`validate_policy`).
fn canonical_signers(signers: &Vec<Ed25519Signer>) -> Vec<Ed25519Signer> {
    let mut sorted = Vec::<Ed25519Signer>::new(signers.env());
    for signer in signers.iter() {
        let key = signer.public_key.to_array();
        let mut at = sorted.len();
        while at > 0 && sorted.get_unchecked(at - 1).public_key.to_array() > key {
            at -= 1;
        }
        sorted.insert(at, signer);
    }
    sorted
}

/// Passkey signers in canonical order: ascending by uncompressed public key bytes, as for
/// `canonical_signers`.
fn canonical_passkey_signers(signers: &Vec<PasskeySigner>) -> Vec<PasskeySigner> {
    let mut sorted = Vec::<PasskeySigner>::new(signers.env());
    for signer in signers.iter() {
        let key = signer.public_key.to_array();
        let mut at = sorted.len();
        while at > 0 && sorted.get_unchecked(at - 1).public_key.to_array() > key {
            at -= 1;
        }
        sorted.insert(at, signer);
    }
    sorted
}

fn read_config(env: &Env) -> Result<PolicyConfig, PolicyError> {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(PolicyError::NotInitialized)
}

fn store_config(env: &Env, config: &PolicyConfig, change: &str) -> Result<(), PolicyError> {
    let version = env
        .storage()
        .instance()
        .get::<_, u32>(&DataKey::ConfigVersion)
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(PolicyError::InvalidPolicy)?;
    let signer_count = match config.signature_mode {
        SignatureMode::Ed25519 => config.signers.len(),
        SignatureMode::PasskeyCompatible => config.passkey_signers.len(),
    };
    let policy_commitment = env.crypto().sha256(&config.clone().to_xdr(env)).to_bytes();
    env.storage().instance().set(&DataKey::Config, config);
    env.storage()
        .instance()
        .set(&DataKey::ConfigVersion, &version);
    PolicyConfigurationChanged {
        change: Symbol::new(env, change),
        version,
        policy_commitment,
        signature_mode: config.signature_mode,
        signer_count,
        threshold: config.threshold,
    }
    .publish(env);
    Ok(())
}

fn default_controls() -> PolicyControls {
    PolicyControls {
        valid_from_ledger: 0,
        expires_at_ledger: u32::MAX,
        reject_all: false,
        additional_signer: None,
        required_contract: None,
        required_function: None,
    }
}

fn validate_controls(controls: &PolicyControls) -> Result<(), PolicyError> {
    if controls.valid_from_ledger > controls.expires_at_ledger {
        return Err(PolicyError::InvalidTimeWindow);
    }
    if controls.required_contract.is_some() != controls.required_function.is_some() {
        return Err(PolicyError::InvalidContext);
    }
    Ok(())
}

fn validate_contexts(
    contexts: &Vec<Context>,
    controls: &PolicyControls,
) -> Result<(), PolicyError> {
    if contexts.is_empty() {
        return Err(PolicyError::InvalidContext);
    }
    for context in contexts.iter() {
        if !matches!(context, Context::Contract(_)) {
            return Err(PolicyError::InvalidContext);
        }
    }
    if let (Some(required_contract), Some(required_function)) =
        (&controls.required_contract, &controls.required_function)
    {
        if contexts.len() != 1 {
            return Err(PolicyError::InvalidContext);
        }
        match contexts.get(0).unwrap() {
            Context::Contract(context)
                if context.contract == *required_contract
                    && context.fn_name == *required_function => {}
            _ => return Err(PolicyError::InvalidContext),
        }
    }
    Ok(())
}

fn signer_weight(config: &PolicyConfig, public_key: &BytesN<32>) -> Option<u32> {
    config
        .signers
        .iter()
        .find(|signer| signer.public_key == *public_key)
        .map(|signer| signer.weight)
}

fn passkey_signer_weight(config: &PolicyConfig, public_key: &BytesN<65>) -> Option<u32> {
    config
        .passkey_signers
        .iter()
        .find(|signer| signer.public_key == *public_key)
        .map(|signer| signer.weight)
}

fn validate_canonical_passkey_signature(signature: &BytesN<64>) -> Result<(), PolicyError> {
    let parsed = P256Signature::from_slice(&signature.to_array())
        .map_err(|_| PolicyError::InvalidPasskey)?;
    if parsed.normalize_s().is_some() {
        return Err(PolicyError::InvalidPasskey);
    }
    Ok(())
}

#[contractimpl]
impl PolicyAccountFixture {
    pub fn initialize(
        env: Env,
        admin: Address,
        signers: Vec<Ed25519Signer>,
        threshold: u32,
    ) -> Result<(), PolicyError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(PolicyError::AlreadyInitialized);
        }
        validate_policy(&signers, threshold)?;
        admin.require_auth();
        store_config(
            &env,
            &PolicyConfig {
                admin,
                signers: canonical_signers(&signers),
                passkey_signers: Vec::new(&env),
                threshold,
                signature_mode: SignatureMode::Ed25519,
                controls: default_controls(),
            },
            "initialize",
        )
    }

    pub fn set_signers(
        env: Env,
        signers: Vec<Ed25519Signer>,
        threshold: u32,
    ) -> Result<(), PolicyError> {
        validate_policy(&signers, threshold)?;
        let config = read_config(&env)?;
        config.admin.require_auth();
        let next = PolicyConfig {
            admin: config.admin.clone(),
            signers: canonical_signers(&signers),
            passkey_signers: Vec::new(&env),
            threshold,
            signature_mode: SignatureMode::Ed25519,
            controls: config.controls.clone(),
        };
        if next == config {
            return Ok(());
        }
        store_config(&env, &next, "set_signers")
    }

    pub fn set_passkeys(
        env: Env,
        signers: Vec<PasskeySigner>,
        threshold: u32,
    ) -> Result<(), PolicyError> {
        validate_passkey_policy(&signers, threshold)?;
        let config = read_config(&env)?;
        config.admin.require_auth();
        let mut controls = config.controls.clone();
        controls.additional_signer = None;
        let next = PolicyConfig {
            admin: config.admin.clone(),
            signers: Vec::new(&env),
            passkey_signers: canonical_passkey_signers(&signers),
            threshold,
            signature_mode: SignatureMode::PasskeyCompatible,
            controls,
        };
        if next == config {
            return Ok(());
        }
        store_config(&env, &next, "set_passkeys")
    }

    pub fn set_controls(env: Env, controls: PolicyControls) -> Result<(), PolicyError> {
        validate_controls(&controls)?;
        let mut config = read_config(&env)?;
        config.admin.require_auth();
        if config.controls == controls {
            return Ok(());
        }
        config.controls = controls;
        store_config(&env, &config, "set_controls")
    }

    pub fn config(env: Env) -> Result<PolicyConfig, PolicyError> {
        read_config(&env)
    }

    pub fn config_version(env: Env) -> Result<u32, PolicyError> {
        read_config(&env)?;
        Ok(env
            .storage()
            .instance()
            .get(&DataKey::ConfigVersion)
            .unwrap_or(0))
    }

    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }

    #[must_use]
    pub fn is_test_only(_env: Env) -> bool {
        true
    }

    #[must_use]
    pub fn passkey_supported(_env: Env) -> bool {
        true
    }
}

#[contractimpl]
impl CustomAccountInterface for PolicyAccountFixture {
    type Error = PolicyError;
    type Signature = Vec<AccountSignature>;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signatures: Vec<AccountSignature>,
        auth_contexts: Vec<Context>,
    ) -> Result<(), PolicyError> {
        let config = read_config(&env)?;
        let current_ledger = env.ledger().sequence();
        if current_ledger < config.controls.valid_from_ledger {
            return Err(PolicyError::NotYetValid);
        }
        if current_ledger > config.controls.expires_at_ledger {
            return Err(PolicyError::Expired);
        }
        if config.controls.reject_all {
            return Err(PolicyError::Rejected);
        }
        validate_contexts(&auth_contexts, &config.controls)?;
        let payload_hash = signature_payload.to_bytes();
        let used_key = DataKey::UsedPayload(payload_hash.clone());
        if env.storage().temporary().has(&used_key) {
            return Err(PolicyError::Replay);
        }
        if signatures.is_empty() {
            return Err(PolicyError::MissingSignature);
        }
        if signatures.len() > MAX_SIGNERS {
            return Err(PolicyError::TooManySignatures);
        }

        match config.signature_mode {
            SignatureMode::Ed25519 => {
                let mut seen = Vec::<BytesN<32>>::new(&env);
                let mut accepted_weight = 0_u32;
                let message = Bytes::from_array(&env, &signature_payload.to_bytes().to_array());
                for envelope in signatures.iter() {
                    let AccountSignature::Ed25519(signature) = envelope else {
                        return Err(PolicyError::WrongSignatureMode);
                    };
                    if seen.contains(&signature.public_key) {
                        return Err(PolicyError::DuplicateSigner);
                    }
                    seen.push_back(signature.public_key.clone());
                    let weight = signer_weight(&config, &signature.public_key)
                        .ok_or(PolicyError::UnauthorizedSigner)?;
                    env.crypto().ed25519_verify(
                        &signature.public_key,
                        &message,
                        &signature.signature,
                    );
                    accepted_weight = accepted_weight
                        .checked_add(weight)
                        .ok_or(PolicyError::InsufficientWeight)?;
                }
                if accepted_weight < config.threshold {
                    return Err(PolicyError::InsufficientWeight);
                }
                if let Some(required_signer) = config.controls.additional_signer
                    && !seen.contains(&required_signer)
                {
                    return Err(PolicyError::RequiredSignerMissing);
                }
            }
            SignatureMode::PasskeyCompatible => {
                let mut seen = Vec::<BytesN<65>>::new(&env);
                let mut accepted_weight = 0_u32;
                for envelope in signatures.iter() {
                    let AccountSignature::Passkey(signature) = envelope else {
                        return Err(PolicyError::WrongSignatureMode);
                    };
                    if seen.contains(&signature.public_key) {
                        return Err(PolicyError::DuplicateSigner);
                    }
                    seen.push_back(signature.public_key.clone());
                    if signature.challenge != payload_hash
                        || signature.authenticator_data.len() < MIN_AUTHENTICATOR_DATA_LENGTH
                        || signature.authenticator_data.len() > MAX_AUTHENTICATOR_DATA_LENGTH
                    {
                        return Err(PolicyError::InvalidPasskey);
                    }
                    validate_canonical_passkey_signature(&signature.signature)?;
                    let weight = passkey_signer_weight(&config, &signature.public_key)
                        .ok_or(PolicyError::UnauthorizedSigner)?;
                    let mut signed_message = signature.authenticator_data.clone();
                    signed_message.append(&Bytes::from_array(
                        &env,
                        &signature.client_data_hash.to_array(),
                    ));
                    let digest = env.crypto().sha256(&signed_message);
                    env.crypto().secp256r1_verify(
                        &signature.public_key,
                        &digest,
                        &signature.signature,
                    );
                    accepted_weight = accepted_weight
                        .checked_add(weight)
                        .ok_or(PolicyError::InsufficientWeight)?;
                }
                if accepted_weight < config.threshold {
                    return Err(PolicyError::InsufficientWeight);
                }
            }
        }
        env.storage().temporary().set(&used_key, &true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{
        AccountSignature, Ed25519Signature, Ed25519Signer, MAX_SIGNERS, PasskeySignature,
        PasskeySigner, PolicyAccountFixture, PolicyAccountFixtureClient, PolicyControls,
        PolicyError,
    };
    use ed25519_dalek::{Signer as _, SigningKey};
    use p256::ecdsa::{
        Signature as P256Signature, SigningKey as P256SigningKey, signature::hazmat::PrehashSigner,
    };
    use proptest::prelude::*;
    use soroban_sdk::{
        Address, Bytes, BytesN, Env, IntoVal, Symbol, Vec,
        auth::{Context, ContractContext},
        testutils::{Address as _, Events as _, Ledger as _},
        vec,
    };

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signer(env: &Env, key: &SigningKey, weight: u32) -> Ed25519Signer {
        Ed25519Signer {
            public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
            weight,
        }
    }

    fn signature(env: &Env, key: &SigningKey, payload: &[u8; 32]) -> AccountSignature {
        AccountSignature::Ed25519(Ed25519Signature {
            public_key: BytesN::from_array(env, key.verifying_key().as_bytes()),
            signature: BytesN::from_array(env, &key.sign(payload).to_bytes()),
        })
    }

    fn passkey_key(seed: u8) -> P256SigningKey {
        P256SigningKey::from_bytes((&[seed; 32]).into()).unwrap()
    }

    fn passkey_signer(env: &Env, key: &P256SigningKey, weight: u32) -> PasskeySigner {
        let encoded = key.verifying_key().to_encoded_point(false);
        let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();
        PasskeySigner {
            public_key: BytesN::from_array(env, &public_key),
            weight,
        }
    }

    fn passkey_signature(env: &Env, key: &P256SigningKey, payload: &[u8; 32]) -> AccountSignature {
        passkey_signature_with_authenticator_length(env, key, payload, 37)
    }

    fn passkey_signature_with_authenticator_length(
        env: &Env,
        key: &P256SigningKey,
        payload: &[u8; 32],
        authenticator_data_length: u32,
    ) -> AccountSignature {
        let mut authenticator_data = Bytes::new(env);
        for _ in 0..authenticator_data_length {
            authenticator_data.push_back(0xA5);
        }
        let client_data_hash = [0xB6; 32];
        let mut signed_message = authenticator_data.clone();
        signed_message.append(&Bytes::from_array(env, &client_data_hash));
        let digest = env.crypto().sha256(&signed_message).to_array();
        let signature: P256Signature = key.sign_prehash(&digest).unwrap();
        let signature = signature.normalize_s().unwrap_or(signature);
        let signature_bytes: [u8; 64] = signature.to_bytes().into();
        let encoded = key.verifying_key().to_encoded_point(false);
        let public_key: [u8; 65] = encoded.as_bytes().try_into().unwrap();
        AccountSignature::Passkey(PasskeySignature {
            public_key: BytesN::from_array(env, &public_key),
            authenticator_data,
            client_data_hash: BytesN::from_array(env, &client_data_hash),
            challenge: BytesN::from_array(env, payload),
            signature: BytesN::from_array(env, &signature_bytes),
        })
    }

    fn high_s_signature(signature: &[u8; 64]) -> [u8; 64] {
        const P256_ORDER: [u8; 32] = [
            0xff, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xbc, 0xe6, 0xfa, 0xad, 0xa7, 0x17, 0x9e, 0x84, 0xf3, 0xb9, 0xca, 0xc2,
            0xfc, 0x63, 0x25, 0x51,
        ];
        let mut high = [0_u8; 64];
        high[..32].copy_from_slice(&signature[..32]);
        let mut borrow = 0_i16;
        for index in (0..32).rev() {
            let difference =
                i16::from(P256_ORDER[index]) - i16::from(signature[32 + index]) - borrow;
            if difference < 0 {
                high[32 + index] = u8::try_from(difference + 256).unwrap();
                borrow = 1;
            } else {
                high[32 + index] = u8::try_from(difference).unwrap();
                borrow = 0;
            }
        }
        high
    }

    fn check(
        env: &Env,
        contract_id: &Address,
        payload: &[u8; 32],
        signatures: &Vec<AccountSignature>,
    ) -> Result<(), Result<PolicyError, soroban_sdk::InvokeError>> {
        let contexts = contract_contexts(env, contract_id, "web_auth_verify");
        check_with_contexts(env, contract_id, payload, signatures, &contexts)
    }

    fn contract_contexts(env: &Env, target: &Address, function: &str) -> Vec<Context> {
        vec![
            env,
            Context::Contract(ContractContext {
                contract: target.clone(),
                fn_name: Symbol::new(env, function),
                args: Vec::new(env),
            }),
        ]
    }

    fn check_with_contexts(
        env: &Env,
        contract_id: &Address,
        payload: &[u8; 32],
        signatures: &Vec<AccountSignature>,
        contexts: &Vec<Context>,
    ) -> Result<(), Result<PolicyError, soroban_sdk::InvokeError>> {
        env.try_invoke_contract_check_auth::<PolicyError>(
            contract_id,
            &BytesN::from_array(env, payload),
            signatures.clone().into_val(env),
            contexts,
        )
    }

    fn controls() -> PolicyControls {
        PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        }
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

    #[test]
    fn successful_payload_cannot_be_replayed() {
        let env = Env::default();
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        PolicyAccountFixtureClient::new(&env, &contract_id).initialize(&admin, &signers, &1);
        let payload = [4; 32];
        let signatures = vec![&env, signature(&env, &first, &payload)];

        assert_eq!(check(&env, &contract_id, &payload, &signatures), Ok(()));
        assert_eq!(
            check(&env, &contract_id, &payload, &signatures),
            Err(Ok(PolicyError::Replay))
        );
    }

    #[test]
    fn time_bounds_and_intentional_rejection_are_exact() {
        let env = Env::default();
        env.ledger().set_sequence_number(100);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &signers, &1);
        let mut policy = controls();
        policy.valid_from_ledger = 101;
        policy.expires_at_ledger = 102;
        client.set_controls(&policy);

        let early_payload = [10; 32];
        assert_eq!(
            check(
                &env,
                &contract_id,
                &early_payload,
                &vec![&env, signature(&env, &first, &early_payload)]
            ),
            Err(Ok(PolicyError::NotYetValid))
        );

        env.ledger().set_sequence_number(101);
        let boundary_payload = [11; 32];
        assert_eq!(
            check(
                &env,
                &contract_id,
                &boundary_payload,
                &vec![&env, signature(&env, &first, &boundary_payload)]
            ),
            Ok(())
        );

        env.ledger().set_sequence_number(103);
        let expired_payload = [12; 32];
        assert_eq!(
            check(
                &env,
                &contract_id,
                &expired_payload,
                &vec![&env, signature(&env, &first, &expired_payload)]
            ),
            Err(Ok(PolicyError::Expired))
        );

        policy.valid_from_ledger = 0;
        policy.expires_at_ledger = u32::MAX;
        policy.reject_all = true;
        client.set_controls(&policy);
        let rejected_payload = [13; 32];
        assert_eq!(
            check(
                &env,
                &contract_id,
                &rejected_payload,
                &vec![&env, signature(&env, &first, &rejected_payload)]
            ),
            Err(Ok(PolicyError::Rejected))
        );
    }

    #[test]
    fn additional_signer_is_required_beyond_threshold() {
        let env = Env::default();
        let first = key(1);
        let second = key(2);
        let signers = vec![&env, signer(&env, &first, 1), signer(&env, &second, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &signers, &1);
        let mut policy = controls();
        policy.additional_signer =
            Some(BytesN::from_array(&env, second.verifying_key().as_bytes()));
        client.set_controls(&policy);
        let payload = [14; 32];
        let first_signature = signature(&env, &first, &payload);

        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, first_signature.clone()]
            ),
            Err(Ok(PolicyError::RequiredSignerMissing))
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
    fn exact_contract_context_can_be_required() {
        let env = Env::default();
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let target = Address::generate(&env);
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &signers, &1);
        let mut policy = controls();
        policy.required_contract = Some(target.clone());
        policy.required_function = Some(Symbol::new(&env, "web_auth_verify"));
        client.set_controls(&policy);
        let payload = [15; 32];
        let signatures = vec![&env, signature(&env, &first, &payload)];

        assert_eq!(
            check_with_contexts(&env, &contract_id, &payload, &signatures, &Vec::new(&env)),
            Err(Ok(PolicyError::InvalidContext))
        );
        assert_eq!(
            check_with_contexts(
                &env,
                &contract_id,
                &payload,
                &signatures,
                &contract_contexts(&env, &target, "wrong_function")
            ),
            Err(Ok(PolicyError::InvalidContext))
        );
        assert_eq!(
            check_with_contexts(
                &env,
                &contract_id,
                &payload,
                &signatures,
                &contract_contexts(&env, &target, "web_auth_verify")
            ),
            Ok(())
        );
    }

    #[test]
    fn passkey_compatible_mode_verifies_p256_assertions() {
        let env = Env::default();
        let bootstrap = key(1);
        let bootstrap_signers = vec![&env, signer(&env, &bootstrap, 1)];
        let passkey = passkey_key(3);
        let passkeys = vec![&env, passkey_signer(&env, &passkey, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &bootstrap_signers, &1);
        client.set_passkeys(&passkeys, &1);
        assert!(client.passkey_supported());
        let payload = [16; 32];

        env.cost_estimate().budget().reset_tracker();
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, passkey_signature(&env, &passkey, &payload)]
            ),
            Ok(())
        );
        assert!(env.cost_estimate().budget().cpu_instruction_cost() <= 30_000_000);
        assert!(env.cost_estimate().budget().memory_bytes_cost() <= 24_000_000);
    }

    #[test]
    fn passkey_mode_rejects_wrong_envelope_and_challenge() {
        let env = Env::default();
        let bootstrap = key(1);
        let bootstrap_signers = vec![&env, signer(&env, &bootstrap, 1)];
        let passkey = passkey_key(3);
        let passkeys = vec![&env, passkey_signer(&env, &passkey, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &bootstrap_signers, &1);
        client.set_passkeys(&passkeys, &1);
        let payload = [17; 32];

        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, signature(&env, &bootstrap, &payload)]
            ),
            Err(Ok(PolicyError::WrongSignatureMode))
        );

        let mut altered = passkey_signature(&env, &passkey, &payload);
        if let AccountSignature::Passkey(assertion) = &mut altered {
            assertion.challenge = BytesN::from_array(&env, &[18; 32]);
        }
        assert_eq!(
            check(&env, &contract_id, &payload, &vec![&env, altered]),
            Err(Ok(PolicyError::InvalidPasskey))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, passkey_signature(&env, &passkey, &payload)]
            ),
            Ok(())
        );
    }

    #[test]
    fn passkey_mode_rejects_high_s_and_malformed_scalars_without_replay_markers() {
        let env = Env::default();
        let bootstrap = key(1);
        let bootstrap_signers = vec![&env, signer(&env, &bootstrap, 1)];
        let passkey = passkey_key(3);
        let passkeys = vec![&env, passkey_signer(&env, &passkey, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &bootstrap_signers, &1);
        client.set_passkeys(&passkeys, &1);

        let high_s_payload = [19; 32];
        let mut high_s = passkey_signature(&env, &passkey, &high_s_payload);
        if let AccountSignature::Passkey(assertion) = &mut high_s {
            assertion.signature =
                BytesN::from_array(&env, &high_s_signature(&assertion.signature.to_array()));
        }
        assert_eq!(
            check(&env, &contract_id, &high_s_payload, &vec![&env, high_s]),
            Err(Ok(PolicyError::InvalidPasskey))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &high_s_payload,
                &vec![&env, passkey_signature(&env, &passkey, &high_s_payload)]
            ),
            Ok(())
        );

        let malformed_payload = [20; 32];
        let mut malformed = passkey_signature(&env, &passkey, &malformed_payload);
        if let AccountSignature::Passkey(assertion) = &mut malformed {
            assertion.signature = BytesN::from_array(&env, &[0; 64]);
        }
        assert_eq!(
            check(
                &env,
                &contract_id,
                &malformed_payload,
                &vec![&env, malformed]
            ),
            Err(Ok(PolicyError::InvalidPasskey))
        );
        assert_eq!(
            check(
                &env,
                &contract_id,
                &malformed_payload,
                &vec![&env, passkey_signature(&env, &passkey, &malformed_payload)]
            ),
            Ok(())
        );
    }

    #[test]
    fn passkey_policy_rejects_malformed_and_off_curve_public_keys() {
        let env = Env::default();
        let bootstrap = key(1);
        let bootstrap_signers = vec![&env, signer(&env, &bootstrap, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &bootstrap_signers, &1);

        for bytes in [[0_u8; 65], [4_u8; 65]] {
            let invalid = vec![
                &env,
                PasskeySigner {
                    public_key: BytesN::from_array(&env, &bytes),
                    weight: 1,
                },
            ];
            assert_eq!(
                client.try_set_passkeys(&invalid, &1),
                Err(Ok(PolicyError::InvalidPolicy))
            );
            assert_eq!(
                client.config().signature_mode,
                super::SignatureMode::Ed25519
            );
        }
    }

    #[test]
    fn passkey_authenticator_data_enforces_exact_bounds_before_replay() {
        let env = Env::default();
        let bootstrap = key(1);
        let bootstrap_signers = vec![&env, signer(&env, &bootstrap, 1)];
        let passkey = passkey_key(3);
        let passkeys = vec![&env, passkey_signer(&env, &passkey, 1)];
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &bootstrap_signers, &1);
        client.set_passkeys(&passkeys, &1);
        let config_before = client.config();

        for (marker, length) in [(21_u8, 36_u32), (22, 513)] {
            let payload = [marker; 32];
            let bounded =
                passkey_signature_with_authenticator_length(&env, &passkey, &payload, length);
            assert_eq!(
                check(&env, &contract_id, &payload, &vec![&env, bounded]),
                Err(Ok(PolicyError::InvalidPasskey))
            );
            assert_eq!(client.config(), config_before);
            assert_eq!(
                check(
                    &env,
                    &contract_id,
                    &payload,
                    &vec![
                        &env,
                        passkey_signature_with_authenticator_length(&env, &passkey, &payload, 37,),
                    ],
                ),
                Ok(())
            );
        }

        for (marker, length) in [(23_u8, 37_u32), (24, 512)] {
            let payload = [marker; 32];
            assert_eq!(
                check(
                    &env,
                    &contract_id,
                    &payload,
                    &vec![
                        &env,
                        passkey_signature_with_authenticator_length(
                            &env, &passkey, &payload, length,
                        ),
                    ],
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn ed25519_signature_count_is_bounded_before_iteration() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let payload = [21; 32];
        let mut signers = Vec::new(&env);
        let mut signatures = Vec::new(&env);
        for seed in 1..=MAX_SIGNERS {
            let signing_key = key(u8::try_from(seed).unwrap());
            signers.push_back(signer(&env, &signing_key, 1));
            signatures.push_back(signature(&env, &signing_key, &payload));
        }
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &signers, &MAX_SIGNERS);

        let mut over_limit = signatures.clone();
        over_limit.push_back(signatures.get(0).unwrap());
        assert_eq!(
            check(&env, &contract_id, &payload, &over_limit),
            Err(Ok(PolicyError::TooManySignatures))
        );
        assert_eq!(check(&env, &contract_id, &payload, &signatures), Ok(()));
    }

    #[test]
    fn passkey_signature_count_is_bounded_before_iteration() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let bootstrap = key(1);
        let payload = [22; 32];
        let mut signers = Vec::new(&env);
        let mut signatures = Vec::new(&env);
        for seed in 1..=MAX_SIGNERS {
            let signing_key = passkey_key(u8::try_from(seed).unwrap());
            signers.push_back(passkey_signer(&env, &signing_key, 1));
            signatures.push_back(passkey_signature(&env, &signing_key, &payload));
        }
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &vec![&env, signer(&env, &bootstrap, 1)], &1);
        client.set_passkeys(&signers, &MAX_SIGNERS);

        let mut over_limit = signatures.clone();
        over_limit.push_back(signatures.get(0).unwrap());
        assert_eq!(
            check(&env, &contract_id, &payload, &over_limit),
            Err(Ok(PolicyError::TooManySignatures))
        );
        assert_eq!(check(&env, &contract_id, &payload, &signatures), Ok(()));
    }

    #[test]
    fn passkey_duplicates_are_rejected_adjacent_separated_and_when_malformed() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let bootstrap = key(1);
        let first = passkey_key(2);
        let second = passkey_key(3);
        let third = passkey_key(4);
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);
        client.initialize(&admin, &vec![&env, signer(&env, &bootstrap, 1)], &1);
        client.set_passkeys(
            &vec![
                &env,
                passkey_signer(&env, &first, 1),
                passkey_signer(&env, &second, 1),
                passkey_signer(&env, &third, 1),
            ],
            &2,
        );

        for (payload, separated) in [([23; 32], false), ([24; 32], true)] {
            let first_signature = passkey_signature(&env, &first, &payload);
            let signatures = if separated {
                vec![
                    &env,
                    first_signature.clone(),
                    passkey_signature(&env, &second, &payload),
                    first_signature,
                ]
            } else {
                vec![&env, first_signature.clone(), first_signature]
            };
            assert_eq!(
                check(&env, &contract_id, &payload, &signatures),
                Err(Ok(PolicyError::DuplicateSigner))
            );
            assert_eq!(
                check(
                    &env,
                    &contract_id,
                    &payload,
                    &vec![
                        &env,
                        passkey_signature(&env, &first, &payload),
                        passkey_signature(&env, &second, &payload),
                    ],
                ),
                Ok(())
            );
        }

        let payload = [25; 32];
        let first_signature = passkey_signature(&env, &first, &payload);
        let mut malformed_duplicate = first_signature.clone();
        if let AccountSignature::Passkey(assertion) = &mut malformed_duplicate {
            assertion.challenge = BytesN::from_array(&env, &[0; 32]);
        }
        assert_eq!(
            check(
                &env,
                &contract_id,
                &payload,
                &vec![&env, first_signature, malformed_duplicate],
            ),
            Err(Ok(PolicyError::DuplicateSigner))
        );
    }

    #[test]
    fn configuration_events_are_versioned_atomic_and_idempotent() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PolicyAccountFixture, ());
        let admin = Address::generate(&env);
        let first = key(1);
        let signers = vec![&env, signer(&env, &first, 1)];
        let client = PolicyAccountFixtureClient::new(&env, &contract_id);

        client.initialize(&admin, &signers, &1);
        assert_eq!(env.events().all().events().len(), 1);
        assert_eq!(client.config_version(), 1);

        client.set_signers(&signers, &1);
        assert_eq!(env.events().all().events().len(), 0);
        assert_eq!(client.config_version(), 1);

        let before = client.config();
        assert_eq!(
            client.try_set_signers(&vec![&env, signer(&env, &first, 0)], &1),
            Err(Ok(PolicyError::InvalidPolicy))
        );
        assert_eq!(env.events().all().events().len(), 0);
        assert_eq!(client.config(), before);
        assert_eq!(client.config_version(), 1);

        let mut next_controls = controls();
        next_controls.reject_all = true;
        client.set_controls(&next_controls);
        assert_eq!(env.events().all().events().len(), 1);
        assert_eq!(client.config_version(), 2);
    }

    fn valid_weight_policy(weights: &[u32], threshold: u32) -> bool {
        if threshold == 0 || weights.iter().any(|weight| *weight == 0) {
            return false;
        }
        weights
            .iter()
            .try_fold(0_u32, |total, weight| total.checked_add(*weight))
            .is_some_and(|total| threshold <= total)
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

        #[test]
        fn signer_weight_boundaries_match_for_ed25519_and_passkeys(
            weights in prop::array::uniform3(prop_oneof![
                Just(0_u32), Just(1_u32), Just(u32::MAX), any::<u32>()
            ]),
            threshold in prop_oneof![Just(0_u32), Just(1_u32), Just(u32::MAX), any::<u32>()],
        ) {
            let expected = valid_weight_policy(&weights, threshold);

            let ed_env = Env::default();
            ed_env.mock_all_auths();
            let ed_id = ed_env.register(PolicyAccountFixture, ());
            let ed_admin = Address::generate(&ed_env);
            let ed_signers = vec![
                &ed_env,
                signer(&ed_env, &key(1), weights[0]),
                signer(&ed_env, &key(2), weights[1]),
                signer(&ed_env, &key(3), weights[2]),
            ];
            let ed_result = PolicyAccountFixtureClient::new(&ed_env, &ed_id)
                .try_initialize(&ed_admin, &ed_signers, &threshold);

            let passkey_env = Env::default();
            passkey_env.mock_all_auths();
            let passkey_id = passkey_env.register(PolicyAccountFixture, ());
            let passkey_admin = Address::generate(&passkey_env);
            let passkey_client = PolicyAccountFixtureClient::new(&passkey_env, &passkey_id);
            let bootstrap = key(9);
            passkey_client.initialize(
                &passkey_admin,
                &vec![&passkey_env, signer(&passkey_env, &bootstrap, 1)],
                &1,
            );
            let passkey_signers = vec![
                &passkey_env,
                passkey_signer(&passkey_env, &passkey_key(1), weights[0]),
                passkey_signer(&passkey_env, &passkey_key(2), weights[1]),
                passkey_signer(&passkey_env, &passkey_key(3), weights[2]),
            ];
            let before = passkey_client.config();
            let passkey_result = passkey_client.try_set_passkeys(&passkey_signers, &threshold);

            prop_assert_eq!(ed_result.is_ok(), expected);
            prop_assert_eq!(passkey_result.is_ok(), expected);
            if !expected {
                prop_assert_eq!(passkey_client.config(), before);
            }
        }
    }
}
