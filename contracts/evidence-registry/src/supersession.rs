use rampspec_shared_types::{ContractError, EvidenceStatus};
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
    // Include the terminal lookup so exactly MAX_CHAIN_LENGTH links are valid.
    for _ in 0..=MAX_CHAIN_LENGTH {
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

pub(crate) fn maintenance_chain_length(
    env: &Env,
    first_id: &BytesN<32>,
) -> Result<u32, ContractError> {
    let mut current_id = first_id.clone();
    let mut current_record =
        storage::evidence(env, &current_id).ok_or(ContractError::EvidenceNotFound)?;

    for length in 0..MAX_CHAIN_LENGTH {
        let Some(next_id) = storage::superseded_by(env, &current_id) else {
            if length == 0 || current_record.status == EvidenceStatus::Superseded {
                return Err(ContractError::InvalidSupersession);
            }
            return Ok(length);
        };

        if current_record.status != EvidenceStatus::Superseded || next_id == current_id {
            return Err(ContractError::InvalidSupersession);
        }

        let next_record =
            storage::evidence(env, &next_id).ok_or(ContractError::EvidenceNotFound)?;
        if next_record.supersedes != Some(current_id.clone()) {
            return Err(ContractError::InvalidSupersession);
        }

        current_id = next_id;
        current_record = next_record;
    }

    Err(ContractError::InvalidSupersession)
}
