# Authentication Fixtures

`web-auth-fixture` and `policy-account-fixture` are deterministic interoperability test contracts. They are not production wallets, evidence registries, custody contracts, or security endorsements.

The fixture release boundary is enforced by `fixtures/manifest.json` and CI:

- every fixture must set `testOnly` to `true`, `productionUse` to `false`, and `holdsFunds` to `false`;
- allowed deployments are limited to local networks and Stellar testnet;
- the two contracts use distinct administration roles and must be initialized with distinct addresses;
- fixture WASM must expose `is_test_only` and its expected authentication entrypoint;
- custody, token, upgrade, and production evidence-publication entrypoints are forbidden.

The web-auth fixture follows the pinned SEP-45 draft 0.1.1 `web_auth_verify(Map<Symbol, String>)` shape and has no contract sub-invocations. The policy-account fixture implements Soroban `__check_auth` for controlled Ed25519 and passkey-compatible tests. Passkey mode checks the P-256 assertion signature and binds it to the authorization payload, but it does not replace browser verification of origin, RP ID, user presence, or attestation.

Do not deploy either fixture to pubnet, place assets under its address, reuse production administrator identities, or describe fixture acceptance as proof that a production wallet is secure.
