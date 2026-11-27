#![no_main]

use libfuzzer_sys::fuzz_target;
use policy_account_fixture::{
    Ed25519Signer, PasskeySigner, PolicyAccountFixture, PolicyAccountFixtureClient, PolicyControls,
    PolicyError, SignatureMode,
};
use soroban_sdk::{
    Address, BytesN, Env, Symbol, Vec,
    testutils::{Address as _, Events as _},
    xdr::{FromXdr, ToXdr},
};

fn byte(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or_default()
}

fn u32_at(data: &[u8], index: usize) -> u32 {
    u32::from_le_bytes([
        byte(data, index),
        byte(data, index + 1),
        byte(data, index + 2),
        byte(data, index + 3),
    ])
}

fn hash_at(env: &Env, data: &[u8], index: usize) -> BytesN<32> {
    let mut bytes = [0_u8; 32];
    for offset in 0..32 {
        bytes[offset] = byte(data, index + offset);
    }
    BytesN::from_array(env, &bytes)
}

fn hash65_at(env: &Env, data: &[u8], index: usize) -> BytesN<65> {
    let mut bytes = [0_u8; 65];
    for offset in 0..65 {
        bytes[offset] = byte(data, index + offset);
    }
    BytesN::from_array(env, &bytes)
}

fuzz_target!(|data: &[u8]| {
    let env = Env::default();

    let signer_count = (byte(data, 0) % 20).into();
    let threshold = u32_at(data, 1);
    let signature_mode = if byte(data, 5) % 2 == 0 {
        SignatureMode::Ed25519
    } else {
        SignatureMode::PasskeyCompatible
    };
    let controls = PolicyControls {
        valid_from_ledger: u32_at(data, 6),
        expires_at_ledger: u32_at(data, 10),
        reject_all: byte(data, 14) % 2 == 1,
        additional_signer: if byte(data, 15) % 2 == 0 {
            Some(hash_at(&env, data, 16))
        } else {
            None
        },
        required_contract: if byte(data, 48) % 3 == 0 {
            Some(Address::generate(&env))
        } else {
            None
        },
        required_function: if byte(data, 49) % 3 == 0 {
            Some(Symbol::new(&env, "invoke"))
        } else {
            None
        },
    };

    env.mock_all_auths();
    let contract_id = env.register(PolicyAccountFixture, ());
    let admin = Address::generate(&env);
    let bootstrap_key = hash_at(&env, data, 50);
    let bootstrap_signers = soroban_sdk::vec![
        &env,
        Ed25519Signer {
            public_key: bootstrap_key,
            weight: 1,
        }
    ];
    let client = PolicyAccountFixtureClient::new(&env, &contract_id);
    client.initialize(&admin, &bootstrap_signers, &1);

    let config_before = client.config();
    let version_before = client.config_version();
    let event_count_before = env.events().all().events().len();

    match signature_mode {
        SignatureMode::Ed25519 => {
            let mut signers = Vec::new(&env);
            for index in 0..signer_count {
                let offset = 82 + (index * 36);
                signers.push_back(Ed25519Signer {
                    public_key: hash_at(&env, data, offset),
                    weight: u32_at(data, offset + 32),
                });
            }

            let result = client.try_set_signers(&signers, &threshold);
            if result.is_err() {
                assert_eq!(client.config(), config_before);
                assert_eq!(client.config_version(), version_before);
                assert_eq!(
                    env.events().all().events().len() - event_count_before,
                    0
                );
            }
        }
        SignatureMode::PasskeyCompatible => {
            let mut signers = Vec::new(&env);
            for index in 0..signer_count {
                let offset = 82 + (index * 69);
                signers.push_back(PasskeySigner {
                    public_key: hash65_at(&env, data, offset),
                    weight: u32_at(data, offset + 65),
                });
            }

            let result = client.try_set_passkeys(&signers, &threshold);
            if result.is_err() {
                assert_eq!(client.config(), config_before);
                assert_eq!(client.config_version(), version_before);
                assert_eq!(
                    env.events().all().events().len() - event_count_before,
                    0
                );
            }
        }
    }

    let controls_result = client.try_set_controls(&controls);
    if controls_result.is_err() {
        let final_config = client.config();
        assert_eq!(final_config.controls, config_before.controls);
    }
});
