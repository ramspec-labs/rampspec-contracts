#![no_std]

pub const TEST_NETWORK: &str = "local";

use proptest::prelude::*;

#[derive(Clone, Debug)]
pub enum AttestorOp {
    Register { actor_idx: usize, meta_marker: u8 },
    SetEnabled { actor_idx: usize, enabled: bool },
    UnauthorizedSetEnabled { actor_idx: usize, enabled: bool },
    Maintain { actor_idx: usize },
}

#[derive(Clone, Debug)]
pub enum EvidenceIndexOp {
    Publish { publisher_idx: usize, report_marker: u8 },
    Revoke { record_idx: usize, authorized: bool },
    Supersede { record_idx: usize, replacement_marker: u8, authorized: bool },
}

#[derive(Clone, Debug)]
pub enum AdminOp {
    Propose { candidate_idx: usize, by_admin: bool },
    Cancel { by_admin: bool },
    Accept { caller_idx: usize },
}

pub fn attestor_op_strategy(pool_size: usize) -> impl Strategy<Value = AttestorOp> {
    prop_oneof![
        (0..pool_size, 1u8..=250u8).prop_map(|(actor_idx, meta_marker)| {
            AttestorOp::Register { actor_idx, meta_marker }
        }),
        (0..pool_size, any::<bool>()).prop_map(|(actor_idx, enabled)| {
            AttestorOp::SetEnabled { actor_idx, enabled }
        }),
        (0..pool_size, any::<bool>()).prop_map(|(actor_idx, enabled)| {
            AttestorOp::UnauthorizedSetEnabled { actor_idx, enabled }
        }),
        (0..pool_size).prop_map(|actor_idx| AttestorOp::Maintain { actor_idx }),
    ]
}

pub fn evidence_index_op_strategy(
    publisher_pool: usize,
) -> impl Strategy<Value = EvidenceIndexOp> {
    prop_oneof![
        (0..publisher_pool, 1u8..=16u8).prop_map(|(publisher_idx, report_marker)| {
            EvidenceIndexOp::Publish { publisher_idx, report_marker }
        }),
        (0usize..32, any::<bool>()).prop_map(|(record_idx, authorized)| {
            EvidenceIndexOp::Revoke { record_idx, authorized }
        }),
        (0usize..32, 1u8..=32u8, any::<bool>()).prop_map(
            |(record_idx, replacement_marker, authorized)| EvidenceIndexOp::Supersede {
                record_idx,
                replacement_marker,
                authorized,
            }
        ),
    ]
}

pub fn admin_op_strategy(pool_size: usize) -> impl Strategy<Value = AdminOp> {
    prop_oneof![
        (0..pool_size, any::<bool>()).prop_map(|(candidate_idx, by_admin)| {
            AdminOp::Propose { candidate_idx, by_admin }
        }),
        any::<bool>().prop_map(|by_admin| AdminOp::Cancel { by_admin }),
        (0..pool_size).prop_map(|caller_idx| AdminOp::Accept { caller_idx }),
    ]
}