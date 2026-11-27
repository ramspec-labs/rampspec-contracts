# Authentication Fixtures

`web-auth-fixture` and `policy-account-fixture` are deterministic interoperability test contracts. They are not production wallets, evidence registries, custody contracts, or security endorsements.

The fixture release boundary is enforced by `fixtures/manifest.json` and CI:

- every fixture must set `testOnly` to `true`, `productionUse` to `false`, and `holdsFunds` to `false`;
- allowed deployments are limited to local networks and Stellar testnet;
- the two contracts use distinct administration roles and must be initialized with distinct addresses;
- fixture WASM must expose `is_test_only` and its expected authentication entrypoint;
- custody, token, upgrade, and production evidence-publication entrypoints are forbidden.

The web-auth fixture follows the pinned SEP-45 draft 0.1.1 `web_auth_verify(Map<Symbol, String>)` shape and has no contract sub-invocations. The policy-account fixture implements Soroban `__check_auth` for controlled Ed25519 and passkey-compatible tests. Passkey policy storage accepts only uncompressed, non-identity public keys that decode to valid P-256 curve points. Passkey mode accepts authenticator data from 37 through 512 bytes, rejects inputs outside those bounds before cryptographic work or replay storage, checks the P-256 assertion signature, and binds it to the authorization payload. It does not replace browser verification of origin, RP ID, user presence, or attestation.

Policy-account authorization is bounded to at most 16 signature envelopes per
attempt in both modes. A configured public key contributes weight at most once;
duplicate passkey keys are rejected before assertion validation and failed
attempts do not consume the payload replay marker.

Successful initialization and effective changes to signers, passkeys, or
controls publish `PolicyConfigurationChanged`. The event contains a monotonic
configuration version, signature mode, signer count, threshold, and SHA-256
commitment to the complete policy. It never includes public keys, signature
bytes, authenticator data, or client data. Invalid and no-op configuration
calls do not write storage, advance the version, or emit an event.

The configuration version is a `u32` that starts at 1 and advances by exactly one
per effective change, so `u32::MAX` is the last version a fixture can reach. An
effective change at that version fails with `InvalidPolicy` (error 3) before
anything is written: configuration, its commitment, the version, replay markers
and TTLs stay as they were, and no event is published. The version never wraps
or repeats. No-op calls still succeed at the bound, and authorization is checked
before the version, so an unauthorized caller gets an authorization failure
rather than the overflow error. `tests/versioning.rs` pins this behaviour.

Do not deploy either fixture to pubnet, place assets under its address, reuse production administrator identities, or describe fixture acceptance as proof that a production wallet is secure.
