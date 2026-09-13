use rampspec_shared_types::{
    AttestorRecord, ContractError, EvidenceRecord, InstanceKeyV1, NetworkKind, PersistentKeyV1,
};
use soroban_sdk::{Address, BytesN, Env};

pub(crate) const INSTANCE_TTL_THRESHOLD: u32 = 103_680;
pub(crate) const INSTANCE_TTL_TARGET: u32 = 2_073_600;
pub(crate) const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
pub(crate) const PERSISTENT_TTL_TARGET: u32 = 3_110_400;

fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_TARGET);
}

fn extend_persistent_ttl(env: &Env, key: &PersistentKeyV1) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_TARGET);
}

pub(crate) fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&InstanceKeyV1::SchemaVersion)
}

pub(crate) fn schema_version(env: &Env) -> Result<u32, ContractError> {
    env.storage()
        .instance()
        .get(&InstanceKeyV1::SchemaVersion)
        .ok_or(ContractError::NotInitialized)
}

pub(crate) fn admin(env: &Env) -> Result<Address, ContractError> {
    env.storage()
        .instance()
        .get(&InstanceKeyV1::Admin)
        .ok_or(ContractError::NotInitialized)
}

pub(crate) fn set_admin(env: &Env, admin: &Address) {
    let storage = env.storage().instance();
    storage.set(&InstanceKeyV1::Admin, admin);
    extend_instance_ttl(env);
}

pub(crate) fn pending_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&InstanceKeyV1::PendingAdmin)
}

pub(crate) fn set_pending_admin(env: &Env, pending_admin: &Address) {
    let storage = env.storage().instance();
    storage.set(&InstanceKeyV1::PendingAdmin, pending_admin);
    extend_instance_ttl(env);
}

pub(crate) fn clear_pending_admin(env: &Env) {
    env.storage()
        .instance()
        .remove(&InstanceKeyV1::PendingAdmin);
}

pub(crate) fn current_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage()
        .instance()
        .get(&InstanceKeyV1::CurrentWasmHash)
}

pub(crate) fn set_current_wasm_hash(env: &Env, hash: &BytesN<32>) {
    let storage = env.storage().instance();
    storage.set(&InstanceKeyV1::CurrentWasmHash, hash);
    extend_instance_ttl(env);
}

pub(crate) fn set_instance_state(env: &Env, admin: &Address, schema_version: u32) {
    let storage = env.storage().instance();
    storage.set(&InstanceKeyV1::Admin, admin);
    storage.set(&InstanceKeyV1::Paused, &false);
    storage.set(&InstanceKeyV1::SchemaVersion, &schema_version);
    extend_instance_ttl(env);
}

pub(crate) fn is_paused(env: &Env) -> Result<bool, ContractError> {
    env.storage()
        .instance()
        .get(&InstanceKeyV1::Paused)
        .ok_or(ContractError::NotInitialized)
}

pub(crate) fn set_paused(env: &Env, paused: bool) {
    let storage = env.storage().instance();
    storage.set(&InstanceKeyV1::Paused, &paused);
    extend_instance_ttl(env);
}

pub(crate) fn attestor(env: &Env, address: &Address) -> Option<AttestorRecord> {
    env.storage()
        .persistent()
        .get(&PersistentKeyV1::Attestor(address.clone()))
}

pub(crate) fn set_attestor(env: &Env, record: &AttestorRecord) {
    let key = PersistentKeyV1::Attestor(record.attestor.clone());
    let storage = env.storage().persistent();
    storage.set(&key, record);
    extend_persistent_ttl(env, &key);
}

pub(crate) fn evidence(env: &Env, id: &BytesN<32>) -> Option<EvidenceRecord> {
    env.storage()
        .persistent()
        .get(&PersistentKeyV1::Evidence(id.clone()))
}

pub(crate) fn set_evidence(env: &Env, record: &EvidenceRecord) {
    let key = PersistentKeyV1::Evidence(record.id.clone());
    let storage = env.storage().persistent();
    storage.set(&key, record);
    extend_persistent_ttl(env, &key);
}

pub(crate) fn active_id(
    env: &Env,
    publisher: &Address,
    report_hash: &BytesN<32>,
    network: &NetworkKind,
) -> Option<BytesN<32>> {
    env.storage()
        .persistent()
        .get(&PersistentKeyV1::ActiveReport(
            publisher.clone(),
            report_hash.clone(),
            network.clone(),
        ))
}

pub(crate) fn set_active_id(
    env: &Env,
    publisher: &Address,
    report_hash: &BytesN<32>,
    network: &NetworkKind,
    id: &BytesN<32>,
) {
    let key =
        PersistentKeyV1::ActiveReport(publisher.clone(), report_hash.clone(), network.clone());
    let storage = env.storage().persistent();
    storage.set(&key, id);
    extend_persistent_ttl(env, &key);
}

pub(crate) fn remove_active_id(
    env: &Env,
    publisher: &Address,
    report_hash: &BytesN<32>,
    network: &NetworkKind,
) {
    env.storage()
        .persistent()
        .remove(&PersistentKeyV1::ActiveReport(
            publisher.clone(),
            report_hash.clone(),
            network.clone(),
        ));
}

pub(crate) fn superseded_by(env: &Env, id: &BytesN<32>) -> Option<BytesN<32>> {
    env.storage()
        .persistent()
        .get(&PersistentKeyV1::SupersededBy(id.clone()))
}

pub(crate) fn set_superseded_by(env: &Env, old_id: &BytesN<32>, new_id: &BytesN<32>) {
    let key = PersistentKeyV1::SupersededBy(old_id.clone());
    let storage = env.storage().persistent();
    storage.set(&key, new_id);
    extend_persistent_ttl(env, &key);
}

pub(crate) fn maintain_attestor(env: &Env, address: &Address) -> Result<(), ContractError> {
    if attestor(env, address).is_none() {
        return Err(ContractError::AttestorNotRegistered);
    }
    extend_persistent_ttl(env, &PersistentKeyV1::Attestor(address.clone()));
    extend_instance_ttl(env);
    Ok(())
}

pub(crate) fn maintain_evidence(env: &Env, id: &BytesN<32>) -> Result<(), ContractError> {
    let record = evidence(env, id).ok_or(ContractError::EvidenceNotFound)?;
    extend_persistent_ttl(env, &PersistentKeyV1::Evidence(id.clone()));
    if record.status == rampspec_shared_types::EvidenceStatus::Active {
        extend_persistent_ttl(
            env,
            &PersistentKeyV1::ActiveReport(record.publisher, record.report_hash, record.network),
        );
    }
    if superseded_by(env, id).is_some() {
        extend_persistent_ttl(env, &PersistentKeyV1::SupersededBy(id.clone()));
    }
    extend_instance_ttl(env);
    Ok(())
}
