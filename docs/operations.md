# Contract Operations

## Ownership and Schedule

`operations/schedule.json` is the operational source of truth. Role names identify accountable organization roles and must be mapped to named people in the private operations system before any deployment. Every task has a distinct backup. Scheduled CI validates due dates daily; overdue or malformed work fails the workflow and uses normal GitHub Actions failure notifications as the baseline alert channel.

Deployment-dependent work is `blocked-no-deployment` while the pubnet decision and testnet state remain unverified. This is not completion evidence. When a deployment is accepted, change `deploymentState`, assign the exact manifest, set the initial due dates, configure external paging, and run all deployment-dependent checks immediately.

## TTL and Recovery

Inspect instance/code TTL and every live attestor, evidence record, uniqueness index, and supersession link at least every 30 days. Alert at 45 days remaining, escalate at 30 days, and stop publication below 14 instance days. Use administrator-authorized `maintain_attestor` and `maintain_evidence`; never infer success from transaction submission alone. Read the entries again from RPC and retain transaction/ledger evidence. Archived entries require a reviewed restore-footprint transaction before maintenance. Verify restoration from the deployment manifest and event archive.

## Event Index Recovery

Compare RPC event cursor, latest finalized ledger, contract IDs, transaction hashes, and indexed lifecycle counts daily. On a gap, stop advancing the public cursor, preserve the last verified checkpoint, replay from the earlier ledger with overlapping boundaries, deduplicate by transaction/event identity, and compare reconstructed active/revoked/superseded state with direct contract reads. Do not rewrite immutable source events.

## Incident Response

Unexpected code, schema, administrator, attestor, pause, or upgrade state is critical. Preserve RPC responses and manifests, notify the security and contract owners, and stop publication only when continued writes increase impact. Reads and revocation remain available while paused. Rotate compromised off-chain signer access, disable affected attestors, and use two-step administrator transfer where appropriate. Any code correction follows independent review, release, testnet rehearsal, and the rollback/forward-fix matrix. Conduct the recovery drill every 90 days even without a production deployment, using disposable local state.

## Cost and Resource Budgets

Review ledger fees, storage rent, TTL extension volume, WASM size, CPU/memory, storage writes, and event counts every 30 days for an active deployment. Compare against `tests/resource-budgets/baselines.json`; a higher measured requirement needs documented justification and review, never an automatic baseline increase. Assign the payer and approved ceiling before enabling maintenance automation.

## Dependency and Toolchain Updates

Weekly supply-chain CI checks Rust advisories and emits an SBOM. Release packaging separately records npm advisories for generated clients and blocks moderate-or-higher findings. Time-bounded exceptions in `security/dependency-exceptions.json` are allowed only for non-vulnerability notices that cannot be removed without an upstream platform upgrade; CI verifies that every RustSec ignore is documented and unexpired. Update one toolchain or dependency family at a time, rebuild twice, regenerate specs and clients, rerun properties/fuzz/budgets, and document compatibility. Never relax a pin only to make an advisory disappear.

## Compatibility and Releases

Schema `1` is the only supported storage and manifest schema. Only the latest signed release is supported, but previously published evidence remains readable through compatible upgrades. Additive ABI changes still require generated diff, backend/frontend consumer tests, testnet deployment, upgrade rehearsal, and migration notes. Breaking storage, event, error, or ABI changes require a major version, new contract ID, migration and index replay plan, independent review, and consumer notice. Fixtures remain test-only and are never pubnet-compatible.

## Review Records

Record completion time and immutable evidence for each task, then advance `nextDueAt` by its declared interval. Reviews must cover administrator and attestor membership, signer custody and recovery, active deployment manifests, alert delivery, incident contacts, budget ownership, supported versions, and the pubnet decision. Run `node scripts/verify-operations.mjs --report artifacts/operations/report.json` after every update.
