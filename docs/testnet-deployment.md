# Testnet Deployment

Testnet deployment is an explicit release operation. It never creates or funds identities and it never accepts secret keys or seed phrases through environment variables. Configure each signer as a Stellar CLI identity backed by the operating-system secure store or an approved hardware signer, fund every account with at least 2 testnet XLM, and assign distinct registry, web-auth fixture, and policy-account fixture administrators.

The deployment command fixes the RPC URL to `https://soroban-testnet.stellar.org` and the network passphrase to `Test SDF Network ; September 2015`. It refuses a dirty checkout, requires the declared release to resolve to the checked-out commit, rebuilds deterministic WASM, and checks all signer accounts through Horizon before submitting a transaction.

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

The command is intentionally not a faucet client, secret manager, signer bootstrapper, or pubnet deployer. A failed network, funding, signer, hash, read, write, fixture-label, event, or release check exits without claiming a verified deployment.
