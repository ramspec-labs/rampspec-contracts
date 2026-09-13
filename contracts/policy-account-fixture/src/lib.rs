#![no_std]

use soroban_sdk::{Env, contract, contractimpl};

#[contract]
pub struct PolicyAccountFixture;

#[contractimpl]
impl PolicyAccountFixture {
    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }
}
