# Upgrade Rehearsal

Every release candidate must pass `node scripts/rehearse-upgrade.mjs --preflight-only`. This runs formatting, clippy, the complete workspace test suite, two independent optimized builds, generated-spec and binding drift checks, and every reviewed resource budget. CI exposes the same command as a manually dispatchable rehearsal gate.

Run the stateful rehearsal only against a fresh local or testnet manifest whose exact source release and WASM are still available. Local identities default to `registry-admin` and `attestor`. For testnet, set `RAMPSPEC_REHEARSAL_ADMIN_IDENTITY`, `RAMPSPEC_REHEARSAL_ATTESTOR_IDENTITY`, and `RAMPSPEC_CONFIRM_UPGRADE=rehearse-reviewed-upgrade-on-testnet`. Set `RAMPSPEC_STELLAR_CONFIG_DIR` when identities are not in the default CLI configuration. Never place secret keys or seed phrases in these variables.

```text
node scripts/rehearse-upgrade.mjs deployments/local/runtime/manifest.json
node scripts/rehearse-upgrade.mjs deployments/testnet/runtime/manifest.json
```

The harness first verifies the manifest offline and against RPC. It publishes a pre-upgrade record, upgrades to the exact reviewed evidence-registry WASM already bound by the manifest, proves the old record remains readable, publishes and reads a new record, and saves event indexes from before and after the operation. Reports are written beneath ignored `artifacts/upgrade-rehearsals/`.

Do not pause the registry merely because an upgrade is scheduled. Pause blocks the upgrade entry point and should be used only to stop publication while an incident is assessed; unpause through reviewed administrator authorization before attempting an upgrade. Indexers must replay from the manifest ledger and retain event identity while rebuilding projections.

Recovery follows `upgrade/rehearsal-policy.json`: stop on integrity failure, roll back when old evidence cannot be read, prefer a forward fix when old evidence is intact but new writes fail, and accept only when every check passes. A rollback means deploying the last known compatible WASM through the same authorized path and rerunning reads before reopening publication. A forward fix requires a new reviewed artifact, regenerated clients and specs, fresh budget evidence, and another full rehearsal. Testnet evidence is not production approval.
