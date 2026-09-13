#![no_std]

use soroban_sdk::{Env, contract, contractimpl};

#[contract]
pub struct WebAuthFixture;

#[contractimpl]
impl WebAuthFixture {
    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }
}
