# Release Bundles

A RampSpec contract release is a verifiable directory containing optimized WASM and build metadata, canonical JSON/XDR specifications, packed TypeScript clients, deployment/security/policy evidence, migration notes, source archive, SPDX 2.3 SBOM, provenance, and a checksum inventory. `release-manifest.json` binds the version and source revision to that inventory.

Build a non-publishable candidate while external review is pending:

```text
node scripts/build-release.mjs --version 0.1.0-review.1 --allow-uncleared-candidate
node scripts/verify-release.mjs artifacts/releases/v0.1.0-review.1
```

Candidate status is visibly distinct from a release and cannot create a tag. `--allow-dirty-candidate` is available only for local packaging development; its provenance records `sourceDirty: true` and it must never be distributed as release evidence.

After independent review and remediation are cleared and a live testnet manifest passes RPC verification, build the release from a clean checkout with `--testnet-manifest path --sign-tag`. The command refuses missing gates, verifies the completed bundle, and creates a signed `v<version>` Git tag. Publish only after `node scripts/verify-release.mjs <bundle> --require-release` succeeds in a clean environment and the signed tag is independently verified.

Checksums cover every payload file and reject additions as well as modifications or omissions. `npm-audit.json` records every generated-client advisory; release status rejects any unresolved moderate, high, or critical result, while a candidate preserves the evidence for remediation. Consumers must verify the bundle before extracting `source.tar`, installing a client package, or trusting a WASM hash. Fixture client packages remain test-only even when included for reproducible interoperability tests.
