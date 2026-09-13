#![no_std]

use soroban_sdk::{Address, BytesN, contracttype};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_SCORE_BPS: u32 = 10_000;
pub const EVIDENCE_ID_DOMAIN: &[u8; 20] = b"rampspec-evidence-v1";

pub type Hash32 = BytesN<32>;
pub type EvidenceId = BytesN<32>;

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
pub enum StorageKey {
    Admin,
    PendingAdmin,
    Paused,
    SchemaVersion,
    CurrentWasmHash,
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
        assert_eq!(MAX_SCORE_BPS, 10_000);
        assert_eq!(EVIDENCE_ID_DOMAIN, b"rampspec-evidence-v1");
    }
}
