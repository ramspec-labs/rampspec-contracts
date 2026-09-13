# Contract Migration Notes

The initial `0.1.x` evidence-registry line uses storage schema version `1`. It has no predecessor data migration. Consumers must pin the exact WASM and generated-client hashes in the release manifest and treat unknown schema versions, errors, event shapes, or contract IDs as incompatible.

Future releases must describe storage reads/writes, event and error compatibility, indexer replay requirements, binding changes, TTL effects, and rollback constraints here before packaging. Additive source changes are not automatically storage-compatible. A release that changes contract behavior requires a fresh testnet deployment manifest and successful upgrade rehearsal even when the public ABI is unchanged.

The web-auth and policy-account packages are test fixtures. Their bindings may be used only in deterministic test environments and must not be migrated into pubnet authentication or custody logic.
