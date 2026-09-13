#![no_main]

use libfuzzer_sys::fuzz_target;
use rampspec_shared_types::{EvidenceInput, EvidenceRecord};
use soroban_sdk::{Bytes, Env, xdr::FromXdr};

fuzz_target!(|data: &[u8]| {
    let env = Env::default();
    let bytes = Bytes::from_slice(&env, data);
    let _ = EvidenceInput::from_xdr(&env, &bytes);
    let _ = EvidenceRecord::from_xdr(&env, &bytes);
});
