#![no_std]

use soroban_sdk::{Env, contract, contractimpl};

#[contract]
pub struct EvidenceRegistry;

#[contractimpl]
impl EvidenceRegistry {
    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }
}
