# Testnet Deployment

Testnet deployment is an explicit release operation. It never creates or funds identities and it never accepts secret keys or seed phrases through environment variables. Configure each signer as a Stellar CLI identity backed by the operating-system secure store or an approved hardware signer, fund every account with at least 2 testnet XLM, and assign distinct registry, web-auth fixture, and policy-account fixture administrators.

The deployment command fixes the RPC URL to `https://soroban-testnet.stellar.org` and the network passphrase to `Test SDF Network ; September 2015`. It refuses a dirty checkout, requires the declared release to resolve to the checked-out commit, verifies supplied WASM against the canonical specification manifest, and checks all signer accounts through Horizon before submitting a transaction. Release deployments use the Linux WASM artifact produced by the contract quality workflow.

Set these values to CLI identity aliases, never secret material:

```text
RAMPSPEC_TESTNET_DEPLOYER_IDENTITY
RAMPSPEC_TESTNET_REGISTRY_ADMIN_IDENTITY
RAMPSPEC_TESTNET_WEB_AUTH_ADMIN_IDENTITY
RAMPSPEC_TESTNET_POLICY_ADMIN_IDENTITY
RAMPSPEC_TESTNET_ATTESTOR_IDENTITY
RAMPSPEC_SOURCE_RELEASE
RAMPSPEC_CONFIRM_TESTNET=deploy-reviewed-artifacts-to-testnet
```

Run `node scripts/deploy-testnet.mjs` from a clean release checkout. The script deploys and initializes all three contracts, registers the attestor, exercises pause and unpause writes, reads the schema and attestor, confirms both fixtures expose their test-only label, fetches deployed WASM for byte-level hashing, and queries RPC events for transaction hashes and ledger evidence.

The ignored `deployments/testnet/runtime/manifest.json` is written only after every check passes. `journey-events.json` preserves the RPC event evidence consumed during verification. Review both files before providing the manifest to a backend environment; fixture contract IDs must remain test-only and must never be configured as production policy contracts.

## Verified v0.2.0 deployment

The tracked `deployments/testnet/manifest.v0.2.0.json` passed live RPC verification on 2026-11-09. Its contract IDs are:

- evidence registry: `CCM73G2Q5A5MVRZHYW3WRCAL6SVG6WLVE3H476N3OJDSG65BR4XMXVPN`
- policy-account fixture: `CCI45FLB47JHLHZF4Y4EQC3JPQRT7NQQAXIEZHE6GCDYCAXYPPALFMZ5`
- web-auth fixture: `CDS5V7X45CXGUVYAYXEWS4MCPUZRNKSFVMTJ5QYWNE2U4TGRV77ITHO6`

Verify the published evidence with `node scripts/verify-deployment-manifest.mjs deployments/testnet/manifest.v0.2.0.json --live --environment testnet`.

The command is intentionally not a faucet client, secret manager, signer bootstrapper, or pubnet deployer. A failed network, funding, signer, hash, read, write, fixture-label, event, or release check exits without claiming a verified deployment.
