# Evidence Registry Storage

Storage layout version: `1`.

## Instance State

The `InstanceKeyV1` namespace stores one administrator address, an optional pending administrator, an optional pending administrator deadline (a `u32` ledger sequence number written atomically with every proposal and cleared on acceptance or cancellation), the pause flag, the schema version, and the current WASM hash. These values share the contract instance lifetime and are never used as evidence lookup keys.

## Persistent State

The `PersistentKeyV1` namespace stores attestors by address, evidence by its 32-byte identifier, the active publisher/report/network uniqueness index, and the forward supersession link. Full reports, domains, URLs, credentials, customer fields, and authorization tokens are prohibited.

A successful supersession commits the terminal old record, active replacement record, old-index removal, replacement-index insertion, and forward link before emitting exactly one `evidence_superseded` event. Its indexed topics are the old and new identifiers and its data contains the authorizer. Publisher- and administrator-authorized replacements use the same order. Any rejected replacement rolls the transaction back and emits no event.

Revocation removes the active lookup only when it still points to the record being revoked, so a conflicting record is never erased. Administrator-authorized evidence maintenance recreates a missing active lookup for an active record and rejects a conflicting lookup before extending any TTL.

## Administrator Proposal Lifetime

A pending administrator proposal expires after `ADMIN_PROPOSAL_TTL_LEDGERS` ledgers (≈1 day at 5 s/ledger on Mainnet). The deadline ledger is stored in `InstanceKeyV1::PendingAdminDeadline` and written atomically with `InstanceKeyV1::PendingAdmin` on every `propose_admin` call. `accept_admin` checks `current_ledger > deadline` and returns `AdminProposalExpired` without mutating any state or emitting an event when the check fails. Cancellation or a fresh proposal both clear the deadline key. A missing deadline key returns `AdminProposalMissing` to guard against proposals written before this feature was deployed.

## Compatibility Policy

New optional indexes may use a newly named key variant. Existing key encodings and stored value meanings are immutable. A value-shape change requires an explicit new version and migration tooling; incompatible evidence interpretation requires a new contract deployment and migration index rather than in-place reinterpretation.

The `SCHEMA_VERSION` and `STORAGE_LAYOUT_VERSION` constants are separately asserted in tests. Contract release checks compare generated specifications before deployment.
