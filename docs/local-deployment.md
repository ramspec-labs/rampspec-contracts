# Local Deployment

Run `node scripts/deploy-local.mjs` from the repository root with Docker Desktop running. The harness starts a disposable Stellar local network, stores generated identities only under ignored `deployments/local/runtime`, builds optimized WASM, deploys all three contracts, and initializes each with a distinct administrator.

The smoke sequence registers an evidence attestor, toggles registry pause state, reads the schema, reads both fixture configurations, fetches each deployed WASM, and compares its SHA-256 with the local release artifact. A verified runtime manifest is written only after every check succeeds.

Set `RAMPSPEC_USE_EXISTING_LOCAL=1` to use an already running `local` network configured in the same runtime directory. Set `RAMPSPEC_LOCAL_CONTAINER` to change the disposable container name. Do not copy runtime identities into source control or reuse them outside this local network.

Run `node scripts/teardown-local.mjs` to stop the named container and remove the ignored identities, fetched WASM, and manifest. The operation is intentionally destructive only within `deployments/local/runtime`.
