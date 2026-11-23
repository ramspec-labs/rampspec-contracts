# Repository Ownership

This repository owns Soroban source, shared contract types, contract tests, generated specifications and TypeScript bindings, deterministic WASM artifacts, checksums, deployment tooling, and deployment manifests.

It does not own the hosted API, orchestration, full reports, frontend pages, customer identity, KYC data, secrets, target URLs, or fund movement. On-chain evidence is a hash commitment and verification aid, not an endorsement, certification, or legal-compliance determination.

## Evidence Registry Authorization Model

The evidence registry enforces a capability-based authorization model with distinct authority roles:

### Administrator Authority

The administrator has exclusive authority over:
- **Attestor management**: `register_attestor`, `set_attestor_enabled`, `maintain_attestor`
- **Pause control**: `set_paused`
- **Admin transfer**: `propose_admin`, `cancel_admin_transfer`
- **Upgrade control**: `upgrade`
- **Evidence maintenance**: `maintain_evidence`
- **Evidence supersession**: Can supersede any evidence (with restrictions)
- **Evidence revocation**: Can revoke any evidence

The administrator is set during initialization and can only be changed through a two-step transfer process requiring both the current and pending administrator's authorization.

### Publisher Authority

Each registered attestor (publisher) has authority over:
- **Evidence publication**: `publish_evidence` for their own attestor address
- **Evidence revocation**: Can revoke evidence they published
- **Evidence supersession**: Can supersede evidence they published (with restrictions)

Publishers cannot supersede their own evidence with a different publisher unless authorized by the administrator.

### Pending Administrator Authority

A pending administrator (after `propose_admin` is called) has authority to:
- **Accept transfer**: `accept_admin` to complete the administrator transfer

### Authorization Guarantees

All privileged operations:
- Require exactly one authorized address (no unexpected additional signers)
- Have no nested contract invocations that broaden the authorization scope
- Fail atomically without partial state changes when authorization is missing
- Preserve storage and TTL state on authorization failure

Read-only operations (`schema_version`, `get_evidence`, `get_attestor`, `is_active`) require no authorization and are callable by any address.

### Authorization Verification

The authorization model is verified through comprehensive test coverage in `contracts/evidence-registry/tests/authorization.rs`, which validates:
- Correct authorized address for each privileged operation
- Rejection of unauthorized callers
- Absence of unexpected nested invocations
- Authorization scope boundaries between admin and publisher roles
- State preservation on authorization failures

