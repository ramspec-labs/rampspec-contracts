use rampspec_shared_types::{
    ContractError, EvidenceInput, MAX_SCORE_BPS, SCHEMA_VERSION, SUPPORTED_PROTOCOL_BITMAP,
};
use soroban_sdk::BytesN;

pub(crate) fn validate_input(
    input: &EvidenceInput,
    supersedes: Option<&BytesN<32>>,
    schema_version: u32,
) -> Result<(), ContractError> {
    if schema_version != SCHEMA_VERSION {
        return Err(ContractError::UpgradeNotAllowed);
    }

    let hashes = [
        &input.report_hash,
        &input.target_hash,
        &input.suite_hash,
        &input.specs_hash,
        &input.artifact_root,
    ];
    if hashes.iter().any(|hash| hash.to_array() == [0; 32])
        || supersedes.is_some_and(|id| id.to_array() == [0; 32])
    {
        return Err(ContractError::InvalidHash);
    }
    if input.score_bps > MAX_SCORE_BPS {
        return Err(ContractError::InvalidScore);
    }
    if input.protocol_bitmap == 0 || input.protocol_bitmap & !SUPPORTED_PROTOCOL_BITMAP != 0 {
        return Err(ContractError::InvalidCounts);
    }

    let total = input
        .passed
        .checked_add(input.failed)
        .and_then(|value| value.checked_add(input.warnings))
        .and_then(|value| value.checked_add(input.skipped))
        .ok_or(ContractError::InvalidCounts)?;
    if total == 0 {
        return Err(ContractError::InvalidCounts);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rampspec_shared_types::NetworkKind;
    use soroban_sdk::{Address, Env, testutils::Address as _};

    fn valid_input(env: &Env) -> EvidenceInput {
        EvidenceInput {
            publisher: Address::generate(env),
            report_hash: BytesN::from_array(env, &[1; 32]),
            target_hash: BytesN::from_array(env, &[2; 32]),
            suite_hash: BytesN::from_array(env, &[3; 32]),
            specs_hash: BytesN::from_array(env, &[4; 32]),
            artifact_root: BytesN::from_array(env, &[5; 32]),
            network: NetworkKind::Testnet,
            protocol_bitmap: 1,
            score_bps: MAX_SCORE_BPS,
            passed: 1,
            failed: 0,
            warnings: 0,
            skipped: 0,
        }
    }

    #[test]
    fn accepts_documented_boundaries() {
        let env = Env::default();
        let mut input = valid_input(&env);
        assert_eq!(validate_input(&input, None, SCHEMA_VERSION), Ok(()));
        input.score_bps = 0;
        input.protocol_bitmap = SUPPORTED_PROTOCOL_BITMAP;
        assert_eq!(validate_input(&input, None, SCHEMA_VERSION), Ok(()));
    }

    #[test]
    fn rejects_each_zero_hash() {
        let env = Env::default();
        let zero = BytesN::from_array(&env, &[0; 32]);
        let mut cases = [
            valid_input(&env),
            valid_input(&env),
            valid_input(&env),
            valid_input(&env),
            valid_input(&env),
        ];
        cases[0].report_hash = zero.clone();
        cases[1].target_hash = zero.clone();
        cases[2].suite_hash = zero.clone();
        cases[3].specs_hash = zero.clone();
        cases[4].artifact_root = zero.clone();
        for input in cases {
            assert_eq!(
                validate_input(&input, None, SCHEMA_VERSION),
                Err(ContractError::InvalidHash)
            );
        }
        assert_eq!(
            validate_input(&valid_input(&env), Some(&zero), SCHEMA_VERSION),
            Err(ContractError::InvalidHash)
        );
    }

    #[test]
    fn rejects_score_bitmap_and_count_failures() {
        let env = Env::default();
        let mut score = valid_input(&env);
        score.score_bps = MAX_SCORE_BPS + 1;
        assert_eq!(
            validate_input(&score, None, SCHEMA_VERSION),
            Err(ContractError::InvalidScore)
        );

        for bitmap in [0, SUPPORTED_PROTOCOL_BITMAP + 1, u64::MAX] {
            let mut input = valid_input(&env);
            input.protocol_bitmap = bitmap;
            assert_eq!(
                validate_input(&input, None, SCHEMA_VERSION),
                Err(ContractError::InvalidCounts)
            );
        }

        let mut empty = valid_input(&env);
        empty.passed = 0;
        assert_eq!(
            validate_input(&empty, None, SCHEMA_VERSION),
            Err(ContractError::InvalidCounts)
        );
        let mut overflow = valid_input(&env);
        overflow.passed = u32::MAX;
        overflow.failed = 1;
        assert_eq!(
            validate_input(&overflow, None, SCHEMA_VERSION),
            Err(ContractError::InvalidCounts)
        );
    }

    #[test]
    fn rejects_an_incompatible_schema() {
        let env = Env::default();
        assert_eq!(
            validate_input(&valid_input(&env), None, SCHEMA_VERSION + 1),
            Err(ContractError::UpgradeNotAllowed)
        );
    }
}
