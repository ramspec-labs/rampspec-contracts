#![no_std]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use soroban_sdk::{
    Address, BytesN, Env, Map, String, Symbol, contract, contracterror, contractevent,
    contractimpl, contracttype,
};

pub const SEP_45_VERSION: &str = "0.1.1";

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WebAuthError {
    MissingArgument = 1,
    MalformedArgument = 2,
    NotInitialized = 3,
    AlreadyInitialized = 4,
    InvalidTimeWindow = 5,
    NotYetValid = 6,
    Expired = 7,
    Replay = 8,
    Rejected = 9,
    UnknownNonce = 10,
}

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureMode {
    Accept,
    Reject,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureConfig {
    pub admin: Address,
    pub mode: FixtureMode,
    pub valid_from_ledger: u32,
    pub expires_at_ledger: u32,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Config,
    Nonce(BytesN<32>),
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureConfigured {
    #[topic]
    pub admin: Address,
    pub mode: FixtureMode,
    pub valid_from_ledger: u32,
    pub expires_at_ledger: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticationVerified {
    #[topic]
    pub account: Address,
    #[topic]
    pub nonce_hash: BytesN<32>,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureNonceReset {
    #[topic]
    pub admin: Address,
    #[topic]
    pub nonce_hash: BytesN<32>,
}

#[contract]
pub struct WebAuthFixture;

fn required(args: &Map<Symbol, String>, env: &Env, name: &str) -> Result<String, WebAuthError> {
    args.get(Symbol::new(env, name))
        .ok_or(WebAuthError::MissingArgument)
}

fn nonce_hash(env: &Env, nonce: &String) -> BytesN<32> {
    env.crypto().sha256(&nonce.to_bytes()).to_bytes()
}

fn validate_text(value: &String, max_length: u32) -> Result<(), WebAuthError> {
    if value.is_empty() || value.len() > max_length {
        return Err(WebAuthError::MalformedArgument);
    }
    Ok(())
}

fn validate_address(value: &String, prefix: u8) -> Result<(), WebAuthError> {
    let bytes = value.to_bytes();
    if bytes.len() != 56 || bytes.get(0).unwrap() != prefix {
        return Err(WebAuthError::MalformedArgument);
    }
    Ok(())
}

fn validate_window(valid_from_ledger: u32, expires_at_ledger: u32) -> Result<(), WebAuthError> {
    if valid_from_ledger > expires_at_ledger {
        return Err(WebAuthError::InvalidTimeWindow);
    }
    Ok(())
}

fn read_config(env: &Env) -> Result<FixtureConfig, WebAuthError> {
    env.storage()
        .instance()
        .get(&DataKey::Config)
        .ok_or(WebAuthError::NotInitialized)
}

struct ParsedArgs {
    account: Address,
    server: Address,
    client_domain_account: Option<Address>,
    nonce: String,
}

fn parse_args(env: &Env, args: &Map<Symbol, String>) -> Result<ParsedArgs, WebAuthError> {
    let account = required(args, env, "account")?;
    let home_domain = required(args, env, "home_domain")?;
    let web_auth_domain = required(args, env, "web_auth_domain")?;
    let server = required(args, env, "web_auth_domain_account")?;
    let nonce = required(args, env, "nonce")?;
    let client_domain = args.get(Symbol::new(env, "client_domain"));
    let client_domain_account = args.get(Symbol::new(env, "client_domain_account"));

    validate_address(&account, b'C')?;
    validate_address(&server, b'G')?;
    validate_text(&home_domain, 253)?;
    validate_text(&web_auth_domain, 253)?;
    validate_text(&nonce, 128)?;

    if client_domain.is_some() != client_domain_account.is_some() {
        return Err(WebAuthError::MalformedArgument);
    }
    if let Some(domain) = client_domain {
        validate_text(&domain, 253)?;
    }
    if let Some(domain_account) = &client_domain_account {
        validate_address(domain_account, b'G')?;
    }

    Ok(ParsedArgs {
        account: Address::from_string(&account),
        server: Address::from_string(&server),
        client_domain_account: client_domain_account.map(|value| Address::from_string(&value)),
        nonce,
    })
}

#[contractimpl]
impl WebAuthFixture {
    pub fn initialize(
        env: Env,
        admin: Address,
        mode: FixtureMode,
        valid_from_ledger: u32,
        expires_at_ledger: u32,
    ) -> Result<(), WebAuthError> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(WebAuthError::AlreadyInitialized);
        }
        validate_window(valid_from_ledger, expires_at_ledger)?;
        admin.require_auth();
        let config = FixtureConfig {
            admin: admin.clone(),
            mode,
            valid_from_ledger,
            expires_at_ledger,
        };
        env.storage().instance().set(&DataKey::Config, &config);
        FixtureConfigured {
            admin,
            mode,
            valid_from_ledger,
            expires_at_ledger,
        }
        .publish(&env);
        Ok(())
    }

    pub fn configure(
        env: Env,
        mode: FixtureMode,
        valid_from_ledger: u32,
        expires_at_ledger: u32,
    ) -> Result<(), WebAuthError> {
        validate_window(valid_from_ledger, expires_at_ledger)?;
        let mut config = read_config(&env)?;
        config.admin.require_auth();
        config.mode = mode;
        config.valid_from_ledger = valid_from_ledger;
        config.expires_at_ledger = expires_at_ledger;
        env.storage().instance().set(&DataKey::Config, &config);
        FixtureConfigured {
            admin: config.admin,
            mode,
            valid_from_ledger,
            expires_at_ledger,
        }
        .publish(&env);
        Ok(())
    }

    pub fn reset_nonce(env: Env, nonce: String) -> Result<(), WebAuthError> {
        validate_text(&nonce, 128)?;
        let config = read_config(&env)?;
        config.admin.require_auth();
        let nonce_hash = nonce_hash(&env, &nonce);
        let key = DataKey::Nonce(nonce_hash.clone());
        if !env.storage().temporary().has(&key) {
            return Err(WebAuthError::UnknownNonce);
        }
        env.storage().temporary().remove(&key);
        FixtureNonceReset {
            admin: config.admin,
            nonce_hash,
        }
        .publish(&env);
        Ok(())
    }

    pub fn config(env: Env) -> Result<FixtureConfig, WebAuthError> {
        read_config(&env)
    }

    #[must_use]
    pub fn is_nonce_consumed(env: Env, nonce: String) -> bool {
        let key = DataKey::Nonce(nonce_hash(&env, &nonce));
        env.storage().temporary().has(&key)
    }

    #[must_use]
    pub fn schema_version(_env: Env) -> u32 {
        rampspec_shared_types::SCHEMA_VERSION
    }

    #[must_use]
    pub fn is_test_only(_env: Env) -> bool {
        true
    }

    #[must_use]
    pub fn sep_version(env: Env) -> String {
        String::from_str(&env, SEP_45_VERSION)
    }

    pub fn web_auth_verify(env: Env, args: Map<Symbol, String>) -> Result<(), WebAuthError> {
        let parsed = parse_args(&env, &args)?;
        let config = read_config(&env)?;
        let current_ledger = env.ledger().sequence();
        if current_ledger < config.valid_from_ledger {
            return Err(WebAuthError::NotYetValid);
        }
        if current_ledger > config.expires_at_ledger {
            return Err(WebAuthError::Expired);
        }
        if config.mode == FixtureMode::Reject {
            return Err(WebAuthError::Rejected);
        }
        let nonce_hash = nonce_hash(&env, &parsed.nonce);
        let nonce_key = DataKey::Nonce(nonce_hash.clone());
        if env.storage().temporary().has(&nonce_key) {
            return Err(WebAuthError::Replay);
        }

        parsed.account.require_auth();
        parsed.server.require_auth();
        if let Some(client_domain_account) = parsed.client_domain_account {
            client_domain_account.require_auth();
        }

        env.storage().temporary().set(&nonce_key, &true);
        AuthenticationVerified {
            account: parsed.account,
            nonce_hash,
        }
        .publish(&env);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{FixtureMode, SEP_45_VERSION, WebAuthError, WebAuthFixture, WebAuthFixtureClient};
    use soroban_sdk::{
        Address, BytesN, Env, IntoVal, Map, String, Symbol,
        address_payload::AddressPayload,
        testutils::{Address as _, AuthorizedFunction, Events as _, Ledger as _},
    };

    const CURRENT_LEDGER: u32 = 100;

    fn account_address(env: &Env, seed: u8) -> Address {
        AddressPayload::AccountIdPublicKeyEd25519(BytesN::from_array(env, &[seed; 32]))
            .to_address(env)
    }

    fn args(
        env: &Env,
        account: &Address,
        server: &Address,
        client_domain: Option<&Address>,
        nonce: &str,
    ) -> Map<Symbol, String> {
        let mut args = Map::new(env);
        args.set(Symbol::new(env, "account"), account.to_string());
        args.set(
            Symbol::new(env, "home_domain"),
            String::from_str(env, "anchor.example"),
        );
        args.set(
            Symbol::new(env, "web_auth_domain"),
            String::from_str(env, "auth.anchor.example"),
        );
        args.set(
            Symbol::new(env, "web_auth_domain_account"),
            server.to_string(),
        );
        args.set(Symbol::new(env, "nonce"), String::from_str(env, nonce));
        if let Some(client_domain) = client_domain {
            args.set(
                Symbol::new(env, "client_domain"),
                String::from_str(env, "wallet.example"),
            );
            args.set(
                Symbol::new(env, "client_domain_account"),
                client_domain.to_string(),
            );
        }
        args
    }

    fn setup(mode: FixtureMode) -> (Env, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(CURRENT_LEDGER);
        let contract_id = env.register(WebAuthFixture, ());
        let admin = Address::generate(&env);
        let account = Address::generate(&env);
        let server = account_address(&env, 1);
        WebAuthFixtureClient::new(&env, &contract_id).initialize(
            &admin,
            &mode,
            &(CURRENT_LEDGER - 1),
            &(CURRENT_LEDGER + 1),
        );
        (env, contract_id, account, server)
    }

    #[test]
    fn exposes_test_only_pinned_interface_metadata() {
        let env = Env::default();
        let contract_id = env.register(WebAuthFixture, ());
        let client = WebAuthFixtureClient::new(&env, &contract_id);

        assert!(client.is_test_only());
        assert_eq!(client.sep_version(), String::from_str(&env, SEP_45_VERSION));
    }

    #[test]
    fn requires_every_mandatory_argument() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(WebAuthFixture, ());
        let account = Address::generate(&env);
        let server = account_address(&env, 1);
        let client = WebAuthFixtureClient::new(&env, &contract_id);
        for name in [
            "account",
            "home_domain",
            "web_auth_domain",
            "web_auth_domain_account",
            "nonce",
        ] {
            let mut values = args(&env, &account, &server, None, "nonce");
            values.remove(Symbol::new(&env, name));
            assert_eq!(
                client.try_web_auth_verify(&values),
                Err(Ok(WebAuthError::MissingArgument))
            );
        }
    }

    #[test]
    fn authorization_roots_have_no_sub_invocations() {
        let (env, contract_id, account, server) = setup(FixtureMode::Accept);
        let client_domain = account_address(&env, 2);
        let values = args(&env, &account, &server, Some(&client_domain), "nonce");
        let client = WebAuthFixtureClient::new(&env, &contract_id);

        client.web_auth_verify(&values);

        let auths = env.auths();
        assert_eq!(auths.len(), 3);
        for (authorizer, invocation) in auths {
            assert!(authorizer == account || authorizer == server || authorizer == client_domain);
            assert!(invocation.sub_invocations.is_empty());
            assert_eq!(
                invocation.function,
                AuthorizedFunction::Contract((
                    contract_id.clone(),
                    Symbol::new(&env, "web_auth_verify"),
                    (values.clone(),).into_val(&env),
                ))
            );
        }
    }

    #[test]
    fn consumes_nonce_once_and_allows_admin_reset() {
        let (env, contract_id, account, server) = setup(FixtureMode::Accept);
        let client = WebAuthFixtureClient::new(&env, &contract_id);
        let nonce = String::from_str(&env, "one-time");
        let values = args(&env, &account, &server, None, "one-time");

        client.web_auth_verify(&values);
        assert!(client.is_nonce_consumed(&nonce));
        assert_eq!(
            client.try_web_auth_verify(&values),
            Err(Ok(WebAuthError::Replay))
        );

        client.reset_nonce(&nonce);
        assert!(!client.is_nonce_consumed(&nonce));
        client.web_auth_verify(&values);
        assert_eq!(env.events().all().events().len(), 1);
    }

    #[test]
    fn configurable_rejection_and_time_bounds_are_stable() {
        let (env, contract_id, account, server) = setup(FixtureMode::Reject);
        let client = WebAuthFixtureClient::new(&env, &contract_id);
        let values = args(&env, &account, &server, None, "bounded");
        assert_eq!(
            client.try_web_auth_verify(&values),
            Err(Ok(WebAuthError::Rejected))
        );

        client.configure(
            &FixtureMode::Accept,
            &(CURRENT_LEDGER + 1),
            &(CURRENT_LEDGER + 2),
        );
        assert_eq!(
            client.try_web_auth_verify(&values),
            Err(Ok(WebAuthError::NotYetValid))
        );

        env.ledger().set_sequence_number(CURRENT_LEDGER + 3);
        assert_eq!(
            client.try_web_auth_verify(&values),
            Err(Ok(WebAuthError::Expired))
        );
    }

    #[test]
    fn malformed_arguments_fail_before_authorization() {
        let (env, contract_id, account, server) = setup(FixtureMode::Accept);
        let client = WebAuthFixtureClient::new(&env, &contract_id);

        let mut empty_nonce = args(&env, &account, &server, None, "nonce");
        empty_nonce.set(Symbol::new(&env, "nonce"), String::from_str(&env, ""));
        assert_eq!(
            client.try_web_auth_verify(&empty_nonce),
            Err(Ok(WebAuthError::MalformedArgument))
        );

        let mut unpaired_domain = args(&env, &account, &server, None, "nonce");
        unpaired_domain.set(
            Symbol::new(&env, "client_domain"),
            String::from_str(&env, "wallet.example"),
        );
        assert_eq!(
            client.try_web_auth_verify(&unpaired_domain),
            Err(Ok(WebAuthError::MalformedArgument))
        );

        let mut wrong_account_kind = args(&env, &account, &server, None, "nonce");
        wrong_account_kind.set(Symbol::new(&env, "account"), server.to_string());
        assert_eq!(
            client.try_web_auth_verify(&wrong_account_kind),
            Err(Ok(WebAuthError::MalformedArgument))
        );
    }
}
