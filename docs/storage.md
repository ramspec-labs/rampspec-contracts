# Evidence Registry Storage

Storage layout version: `1`.

## Instance State

The `InstanceKeyV1` namespace stores one administrator address, an optional pending administrator, the pause flag, the schema version, and the current WASM hash. These values share the contract instance lifetime and are never used as evidence lookup keys.

## Persistent State

The `PersistentKeyV1` namespace stores attestors by address, evidence by its 32-byte identifier, the active publisher/report/network uniqueness index, and the forward supersession link. Full reports, domains, URLs, credentials, customer fields, and authorization tokens are prohibited.

## Compatibility Policy

New optional indexes may use a newly named key variant. Existing key encodings and stored value meanings are immutable. A value-shape change requires an explicit new version and migration tooling; incompatible evidence interpretation requires a new contract deployment and migration index rather than in-place reinterpretation.

The `SCHEMA_VERSION` and `STORAGE_LAYOUT_VERSION` constants are separately asserted in tests. Contract release checks compare generated specifications before deployment.
