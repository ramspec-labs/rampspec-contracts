use rampspec_shared_types::ContractError;
use soroban_sdk::{BytesN, Env};

use crate::storage;

const MAX_CHAIN_LENGTH: u32 = 24;

pub(crate) fn validate_link(
    env: &Env,
    old_id: &BytesN<32>,
    new_id: &BytesN<32>,
) -> Result<(), ContractError> {
    if old_id == new_id || storage::superseded_by(env, old_id).is_some() {
        return Err(ContractError::InvalidSupersession);
    }

    let mut cursor = new_id.clone();
    for _ in 0..MAX_CHAIN_LENGTH {
        if cursor == *old_id {
            return Err(ContractError::InvalidSupersession);
        }
        let Some(next) = storage::superseded_by(env, &cursor) else {
            return Ok(());
        };
        cursor = next;
    }
    Err(ContractError::InvalidSupersession)
}
