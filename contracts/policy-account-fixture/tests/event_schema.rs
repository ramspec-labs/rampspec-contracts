use policy_account_fixture::{PolicyConfigurationChanged, PolicyError, SignatureMode};
use soroban_sdk::{BytesN, Env, FromVal, Map, Symbol, TryFromVal, Val, events::Event};

#[test]
fn policy_configuration_event_schema_is_stable_and_privacy_safe() {
    let env = Env::default();
    let event = PolicyConfigurationChanged {
        change: Symbol::new(&env, "set_signers"),
        version: 2,
        policy_commitment: BytesN::from_array(&env, &[7; 32]),
        signature_mode: SignatureMode::Ed25519,
        signer_count: 3,
        threshold: 2,
    };
    let topics = event.topics(&env);
    assert_eq!(topics.len(), 2);
    assert_eq!(
        Symbol::from_val(&env, &topics.get(0).unwrap()),
        Symbol::new(&env, "policy_configuration_changed")
    );
    assert_eq!(
        Symbol::from_val(&env, &topics.get(1).unwrap()),
        Symbol::new(&env, "set_signers")
    );
    let data = Map::<Symbol, Val>::try_from_val(&env, &event.data(&env)).unwrap();
    assert_eq!(data.len(), 5);
    for field in [
        "version",
        "policy_commitment",
        "signature_mode",
        "signer_count",
        "threshold",
    ] {
        assert!(data.contains_key(Symbol::new(&env, field)));
    }
    assert!(!data.contains_key(Symbol::new(&env, "signers")));
    assert!(!data.contains_key(Symbol::new(&env, "signature")));
}

#[test]
fn policy_error_discriminants_are_stable_and_exhaustive() {
    assert_eq!(PolicyError::NotInitialized as u32, 1);
    assert_eq!(PolicyError::AlreadyInitialized as u32, 2);
    assert_eq!(PolicyError::InvalidPolicy as u32, 3);
    assert_eq!(PolicyError::MissingSignature as u32, 4);
    assert_eq!(PolicyError::DuplicateSigner as u32, 5);
    assert_eq!(PolicyError::UnauthorizedSigner as u32, 6);
    assert_eq!(PolicyError::InsufficientWeight as u32, 7);
    assert_eq!(PolicyError::InvalidTimeWindow as u32, 8);
    assert_eq!(PolicyError::NotYetValid as u32, 9);
    assert_eq!(PolicyError::Expired as u32, 10);
    assert_eq!(PolicyError::Rejected as u32, 11);
    assert_eq!(PolicyError::Replay as u32, 12);
    assert_eq!(PolicyError::RequiredSignerMissing as u32, 13);
    assert_eq!(PolicyError::InvalidContext as u32, 14);
    assert_eq!(PolicyError::WrongSignatureMode as u32, 15);
    assert_eq!(PolicyError::InvalidPasskey as u32, 16);
    assert_eq!(PolicyError::TooManySignatures as u32, 17);
}

#[test]
fn policy_commitment_vectors_are_deterministic_and_reproducible() {
    use policy_account_fixture::{Ed25519Signer, PasskeySigner, PolicyConfig, PolicyControls};
    use soroban_sdk::{Address, BytesN, Env, String, Vec, xdr::ToXdr};

    let env = Env::default();

    let admin_str = String::from_str(
        &env,
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
    );
    let config_a = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 1,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let encoded_a1 = config_a.clone().to_xdr(&env);
    let encoded_a2 = config_a.clone().to_xdr(&env);
    assert_eq!(
        encoded_a1, encoded_a2,
        "Identical configs must encode identically"
    );

    let commitment_a1 = env.crypto().sha256(&encoded_a1).to_bytes();
    let commitment_a2 = env.crypto().sha256(&encoded_a2).to_bytes();
    assert_eq!(
        commitment_a1, commitment_a2,
        "Identical configs must produce identical commitments"
    );

    let config_b = PolicyConfig {
        threshold: 2,
        ..config_a.clone()
    };
    let commitment_b = env.crypto().sha256(&config_b.to_xdr(&env)).to_bytes();
    assert_ne!(
        commitment_a1, commitment_b,
        "Threshold change must change commitment"
    );

    let config_ordered_1 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            },
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[2; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 2,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let config_ordered_2 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[2; 32]),
                weight: 1,
            },
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[1; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 2,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let commitment_ordered_1 = env
        .crypto()
        .sha256(&config_ordered_1.to_xdr(&env))
        .to_bytes();
    let commitment_ordered_2 = env
        .crypto()
        .sha256(&config_ordered_2.to_xdr(&env))
        .to_bytes();
    assert_ne!(
        commitment_ordered_1, commitment_ordered_2,
        "Signer order must affect commitment"
    );

    let config_ed25519 = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: soroban_sdk::vec![
            &env,
            Ed25519Signer {
                public_key: BytesN::from_array(&env, &[3; 32]),
                weight: 1,
            }
        ],
        passkey_signers: Vec::new(&env),
        threshold: 1,
        signature_mode: SignatureMode::Ed25519,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let config_passkey = PolicyConfig {
        admin: Address::from_string(&admin_str),
        signers: Vec::new(&env),
        passkey_signers: soroban_sdk::vec![
            &env,
            PasskeySigner {
                public_key: BytesN::from_array(&env, &{
                    let mut bytes = [0_u8; 65];
                    bytes[0] = 4;
                    for index in 1..65 {
                        bytes[index] = 3;
                    }
                    bytes
                }),
                weight: 1,
            }
        ],
        threshold: 1,
        signature_mode: SignatureMode::PasskeyCompatible,
        controls: PolicyControls {
            valid_from_ledger: 0,
            expires_at_ledger: u32::MAX,
            reject_all: false,
            additional_signer: None,
            required_contract: None,
            required_function: None,
        },
    };

    let commitment_ed25519 = env.crypto().sha256(&config_ed25519.to_xdr(&env)).to_bytes();
    let commitment_passkey = env.crypto().sha256(&config_passkey.to_xdr(&env)).to_bytes();
    assert_ne!(
        commitment_ed25519, commitment_passkey,
        "Signature mode must affect commitment"
    );
}
