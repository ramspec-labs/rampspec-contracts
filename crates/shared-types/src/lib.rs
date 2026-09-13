#![no_std]

use soroban_sdk::{Address, BytesN, contracterror, contracttype};

pub const SCHEMA_VERSION: u32 = 1;
pub const STORAGE_LAYOUT_VERSION: u32 = 1;
pub const MAX_SCORE_BPS: u32 = 10_000;
pub const EVIDENCE_ID_DOMAIN: &[u8; 20] = b"rampspec-evidence-v1";

pub type Hash32 = BytesN<32>;
pub type EvidenceId = BytesN<32>;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    Paused = 4,
    InvalidHash = 5,
    InvalidCounts = 6,
    InvalidScore = 7,
    AttestorNotRegistered = 8,
    AttestorDisabled = 9,
    EvidenceAlreadyExists = 10,
    EvidenceNotFound = 11,
    EvidenceNotActive = 12,
    InvalidSupersession = 13,
    AdminProposalMissing = 14,
    UpgradeNotAllowed = 15,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkKind {
    Testnet,
    Futurenet,
    Pubnet,
    Standalone,
}

impl NetworkKind {
    #[must_use]
    pub const fn discriminant(&self) -> u32 {
        match self {
            Self::Testnet => 0,
            Self::Futurenet => 1,
            Self::Pubnet => 2,
            Self::Standalone => 3,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceStatus {
    Active,
    Superseded,
    Revoked,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceInput {
    pub publisher: Address,
    pub report_hash: BytesN<32>,
    pub target_hash: BytesN<32>,
    pub suite_hash: BytesN<32>,
    pub specs_hash: BytesN<32>,
    pub artifact_root: BytesN<32>,
    pub network: NetworkKind,
    pub protocol_bitmap: u64,
    pub score_bps: u32,
    pub passed: u32,
    pub failed: u32,
    pub warnings: u32,
    pub skipped: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    pub id: BytesN<32>,
    pub publisher: Address,
    pub report_hash: BytesN<32>,
    pub target_hash: BytesN<32>,
    pub suite_hash: BytesN<32>,
    pub specs_hash: BytesN<32>,
    pub artifact_root: BytesN<32>,
    pub network: NetworkKind,
    pub protocol_bitmap: u64,
    pub score_bps: u32,
    pub passed: u32,
    pub failed: u32,
    pub warnings: u32,
    pub skipped: u32,
    pub created_ledger: u32,
    pub status: EvidenceStatus,
    pub supersedes: Option<BytesN<32>>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorRecord {
    pub attestor: Address,
    pub metadata_hash: BytesN<32>,
    pub enabled: bool,
    pub registered_ledger: u32,
    pub updated_ledger: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum InstanceKeyV1 {
    Admin,
    PendingAdmin,
    Paused,
    SchemaVersion,
    CurrentWasmHash,
}

#[contracttype]
#[derive(Clone)]
pub enum PersistentKeyV1 {
    Attestor(Address),
    Evidence(BytesN<32>),
    ActiveReport(Address, BytesN<32>, NetworkKind),
    SupersededBy(BytesN<32>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, xdr::FromXdr, xdr::ToXdr};

    #[test]
    fn network_encoding_round_trips_and_is_unambiguous() {
        let env = Env::default();
        let values = [
            NetworkKind::Testnet,
            NetworkKind::Futurenet,
            NetworkKind::Pubnet,
            NetworkKind::Standalone,
        ];

        for (index, value) in values.iter().enumerate() {
            let encoded = value.clone().to_xdr(&env);
            let decoded = NetworkKind::from_xdr(&env, &encoded).unwrap();
            assert_eq!(decoded, *value);
            assert_eq!(value.discriminant(), u32::try_from(index).unwrap());

            for other in values.iter().skip(index + 1) {
                assert_ne!(encoded, other.clone().to_xdr(&env));
            }
        }
    }

    #[test]
    fn documented_boundaries_are_stable() {
        assert_eq!(SCHEMA_VERSION, 1);
        assert_eq!(STORAGE_LAYOUT_VERSION, 1);
        assert_eq!(MAX_SCORE_BPS, 10_000);
        assert_eq!(EVIDENCE_ID_DOMAIN, b"rampspec-evidence-v1");
    }

    #[test]
    fn storage_key_encodings_are_distinct() {
        let env = Env::default();
        let instance_keys = [
            InstanceKeyV1::Admin.to_xdr(&env),
            InstanceKeyV1::PendingAdmin.to_xdr(&env),
            InstanceKeyV1::Paused.to_xdr(&env),
            InstanceKeyV1::SchemaVersion.to_xdr(&env),
            InstanceKeyV1::CurrentWasmHash.to_xdr(&env),
        ];

        for (index, key) in instance_keys.iter().enumerate() {
            for other in instance_keys.iter().skip(index + 1) {
                assert_ne!(key, other);
            }
        }
    }

    #[test]
    fn error_codes_are_stable() {
        let cases = [
            (ContractError::AlreadyInitialized, 1),
            (ContractError::NotInitialized, 2),
            (ContractError::Unauthorized, 3),
            (ContractError::Paused, 4),
            (ContractError::InvalidHash, 5),
            (ContractError::InvalidCounts, 6),
            (ContractError::InvalidScore, 7),
            (ContractError::AttestorNotRegistered, 8),
            (ContractError::AttestorDisabled, 9),
            (ContractError::EvidenceAlreadyExists, 10),
            (ContractError::EvidenceNotFound, 11),
            (ContractError::EvidenceNotActive, 12),
            (ContractError::InvalidSupersession, 13),
            (ContractError::AdminProposalMissing, 14),
            (ContractError::UpgradeNotAllowed, 15),
        ];

        for (error, expected) in cases {
            assert_eq!(error as u32, expected);
        }
    }
}
