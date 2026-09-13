# Deployment Manifests

`deployments/manifest.schema.json` is the portable schema for local, testnet, and any separately approved pubnet deployment. A manifest binds the network passphrase and RPC endpoint to exact contract IDs, released WASM hashes, source revision, ledger interval, transaction hashes, administrator and attestor policy, test-only fixture status, completed checks, and verification time.

Every writer seals the manifest with `integritySha256`, calculated from recursively sorted JSON fields other than the seal itself. Offline verification rejects structural errors, a wrong network passphrase, altered content, invalid identifiers, incomplete checks, and any WASM hash not present in the committed contract-spec manifest. Local and testnet manifests require both fixtures; pubnet manifests require their contract, hash, and administrator fields to be `null` so a production release cannot silently deploy test-only policy code.

```text
node scripts/verify-deployment-manifest.mjs path/to/manifest.json --environment testnet
```

Add `--live` to fetch every contract from the manifest RPC, compare deployed code bytes, verify every schema version and fixture label, and read back the registered attestor. Connection failures and stale or incompatible contracts terminate with a nonzero exit. The network passphrase is supplied explicitly on every Stellar CLI call; a network alias cannot silently redirect verification.

Runtime manifests and fetched code remain ignored because identity context and deployment timing are environment state. Store reviewed manifests in the release evidence system only after offline and live commands both pass. A manifest proves the stated contract deployment checks; it does not prove backend configuration, frontend behavior, external security review, or production approval.
