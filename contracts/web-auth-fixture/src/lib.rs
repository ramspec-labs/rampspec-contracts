#![no_std]
#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use soroban_sdk::{Address, Env, Map, String, Symbol, contract, contracterror, contractimpl};

pub const SEP_45_VERSION: &str = "0.1.1";

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WebAuthError {
    MissingArgument = 1,
}

#[contract]
pub struct WebAuthFixture;

fn required(args: &Map<Symbol, String>, env: &Env, name: &str) -> Result<String, WebAuthError> {
    args.get(Symbol::new(env, name))
        .ok_or(WebAuthError::MissingArgument)
}

#[contractimpl]
impl WebAuthFixture {
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
        let account = required(&args, &env, "account")?;
        required(&args, &env, "home_domain")?;
        required(&args, &env, "web_auth_domain")?;
        let web_auth_domain_account = required(&args, &env, "web_auth_domain_account")?;
        required(&args, &env, "nonce")?;

        Address::from_string(&account).require_auth();
        Address::from_string(&web_auth_domain_account).require_auth();

        if let Some(client_domain_account) = args.get(Symbol::new(&env, "client_domain_account")) {
            Address::from_string(&client_domain_account).require_auth();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::{SEP_45_VERSION, WebAuthError, WebAuthFixture, WebAuthFixtureClient};
    use soroban_sdk::{
        Address, Env, IntoVal, Map, String, Symbol,
        testutils::{Address as _, AuthorizedFunction},
    };

    fn args(
        env: &Env,
        account: &Address,
        server: &Address,
        client_domain: Option<&Address>,
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
        args.set(
            Symbol::new(env, "nonce"),
            String::from_str(env, "deterministic-fixture-nonce"),
        );
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
        let server = Address::generate(&env);
        let client = WebAuthFixtureClient::new(&env, &contract_id);
        let names = [
            "account",
            "home_domain",
            "web_auth_domain",
            "web_auth_domain_account",
            "nonce",
        ];

        for name in names {
            let mut values = args(&env, &account, &server, None);
            values.remove(Symbol::new(&env, name));
            assert_eq!(
                client.try_web_auth_verify(&values),
                Err(Ok(WebAuthError::MissingArgument))
            );
        }
    }

    #[test]
    fn authorization_roots_have_no_sub_invocations() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(WebAuthFixture, ());
        let account = Address::generate(&env);
        let server = Address::generate(&env);
        let client_domain = Address::generate(&env);
        let values = args(&env, &account, &server, Some(&client_domain));
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
}
