# RampSpec Contracts Implementation Plan

## Purpose

This plan delivers the complete Soroban evidence registry and test fixture contracts described in `RAMPSPEC_FULL_PROJECT_DOCUMENTATION.md`. It includes deterministic builds, authorization, storage, TTL, upgrades, generated interfaces, deployment, security review, and long-term operation.

The contracts repository owns on-chain code, specifications, bindings, WASM, checksums, and deployment manifests. It must not store full reports, domains, URLs, secrets, KYC data, hosted API state, or customer funds.

## Execution Rules

- Complete one narrow contract behavior or delivery capability per phase.
- Pin Rust, Stellar CLI, Soroban SDK, target, container, and dependencies.
- Use integer and fixed-size representations; never use floating-point values.
- Require explicit authorization on every privileged or publishing path.
- Keep reads available while paused and preserve immutable ledger history.
- Generate specs and TypeScript bindings from source; never hand-edit generated artifacts.
- Test local behavior before testnet and require an explicit reviewed decision before any pubnet deployment.
- A phase is complete only when format, lint, unit/integration tests, generated artifacts, and resource impact pass where applicable.

## Phase 01 - Repository and Governance Foundation

**Outcome:** An independent public contracts repository is ready for security-sensitive development.

**Parts:** Add Apache-2.0, README, security policy, contribution/conduct/maintainer files, issue/PR templates, responsible disclosure, ownership boundaries, and release policy.

**Depends on:** RampSpec name and organization confirmation.

**Exit check:** Review requirements and the prohibition on funds, PII, secrets, and certification claims are explicit.

## Phase 02 - Pinned Soroban Workspace

**Outcome:** All contract crates build under one reproducible workspace.

**Parts:** Pin stable Rust, target, Stellar CLI, Soroban SDK, lock file, format/lint settings, shared profiles, and crates for evidence registry, web-auth fixture, policy-account fixture, shared types, and test support.

**Depends on:** Phase 01.

**Exit check:** Clean local and container builds produce the same crate graph.

## Phase 03 - Contract CI

**Outcome:** Every pull request receives deterministic contract checks.

**Parts:** Add format, Clippy with warnings denied, unit/integration/property/fuzz compile, WASM build, spec/binding drift, checksum repeatability, dependency, security, and SBOM jobs.

**Depends on:** Phase 02.

**Exit check:** Linux and Windows generated-output checks are semantically identical.

## Phase 04 - Deterministic WASM Build

**Outcome:** Release WASM can be reproduced and verified.

**Parts:** Build in a pinned container, normalize environment inputs, optimize deterministically, record source/toolchain/container digests, calculate SHA-256, and compare repeated builds.

**Depends on:** Phases 02-03.

**Exit check:** Two clean builds yield identical optimized WASM and checksums.

## Phase 05 - Shared Types and Versioning

**Outcome:** Contract-visible types have stable encodings and explicit schema versions.

**Parts:** Define `NetworkKind`, `EvidenceStatus`, hashes, counts, protocol bitmap, input/record/attestor types, storage keys, semantic constraints, and schema version constants.

**Depends on:** Phase 02.

**Exit check:** Serialization vectors and boundary values match the released contract specification.

## Phase 06 - Error Contract

**Outcome:** Failure behavior is stable and machine-consumable.

**Parts:** Implement the documented numeric errors 1-15 for initialization, authorization, pause, input, attestor, evidence, supersession, admin, and upgrade failures.

**Depends on:** Phase 05.

**Exit check:** Each error number has a focused test and generated-spec assertion.

## Phase 07 - Evidence Registry Storage Layout

**Outcome:** Instance and persistent storage boundaries are explicit.

**Parts:** Store admin/pending admin/pause/schema version in instance state; evidence and attestors in persistent state; reserve versioned keys; document layout and additive-change policy.

**Depends on:** Phase 05.

**Exit check:** Storage snapshots and schema-version tests detect incompatible interpretation changes.

## Phase 08 - Initialize Once

**Outcome:** The evidence registry has exactly one secure initialization path.

**Parts:** Implement `initialize(admin, schema_version)`, validate inputs, set initial pause/admin state, extend TTL, emit `initialized`, and reject every second call.

**Depends on:** Phases 06-07.

**Exit check:** Authorized first initialization and all repeat/invalid paths pass.

## Phase 09 - Attestor Registration

**Outcome:** The admin can register an attestor and its metadata commitment.

**Parts:** Implement `register_attestor`, require admin auth, validate address/hash, create/update the record under defined policy, extend TTL, and emit `attestor_set`.

**Depends on:** Phase 08.

**Exit check:** Authorized, unauthorized, invalid, duplicate/update, event, and storage tests pass.

## Phase 10 - Attestor Enable and Disable

**Outcome:** Publication authority can be removed without deleting history.

**Parts:** Implement `set_attestor_enabled`, require admin auth, preserve metadata/history, emit state changes, and define repeated-operation behavior.

**Depends on:** Phase 09.

**Exit check:** Missing, disabled, re-enabled, unauthorized, and readback cases pass.

## Phase 11 - Evidence Input Validation

**Outcome:** Invalid commitments fail before storage changes.

**Parts:** Validate nonzero fixed hashes, network discriminant, protocol bitmap policy, score 0-10,000, count overflow/consistency, supersedes field, and schema compatibility.

**Depends on:** Phases 05-06.

**Exit check:** Boundary and arbitrary invalid inputs map to deterministic errors with no writes/events.

## Phase 12 - Deterministic Evidence ID

**Outcome:** Every publisher/report/network tuple has one canonical ID.

**Parts:** Encode the `rampspec-evidence-v1` domain separator, publisher address XDR, report hash, and network discriminant; hash with SHA-256; publish cross-language vectors.

**Depends on:** Phase 05.

**Exit check:** Rust and backend vectors match byte for byte across networks and addresses.

## Phase 13 - Evidence Publication

**Outcome:** An enabled attestor can publish one valid immutable record.

**Parts:** Implement `publish_evidence`, require publisher auth, verify enabled attestor/input/ID uniqueness, store Active record with ledger, extend TTL, and emit `evidence_published`.

**Depends on:** Phases 10-12.

**Exit check:** Success, disabled/missing attestor, invalid input, unauthorized, storage, and event tests pass.

## Phase 14 - Duplicate Publication Protection

**Outcome:** Repeated publication never creates multiple records.

**Parts:** Detect deterministic ID and publisher/report/network collisions, return `EvidenceAlreadyExists`, preserve original record/event count, and test retried submissions.

**Depends on:** Phase 13.

**Exit check:** Sequential and randomized duplicate attempts leave exactly one record.

## Phase 15 - Evidence Reads

**Outcome:** Evidence and attestor state can be verified without authorization.

**Parts:** Implement `get_evidence`, `get_attestor`, and `is_active`; define absent values; ensure read methods do not mutate state and remain available while paused.

**Depends on:** Phases 10 and 13.

**Exit check:** Active, missing, disabled-attestor, and paused-read tests pass.

## Phase 16 - Evidence Revocation

**Outcome:** A publisher or authorized administrator can irreversibly revoke active evidence.

**Parts:** Implement `revoke_evidence`, require correct auth, hash-only reason, Active-state requirement, status update, TTL extension, and `evidence_revoked` event.

**Depends on:** Phase 13.

**Exit check:** Publisher/admin success plus unauthorized, missing, repeated, and superseded cases pass.

## Phase 17 - Evidence Supersession

**Outcome:** A corrected report creates a new record linked from the old record.

**Parts:** Implement `supersede_evidence`, enforce same attestor or documented admin authority, validate replacement, create its deterministic ID, mark old record, link both states, and emit `evidence_superseded`.

**Depends on:** Phases 13-14.

**Exit check:** Valid replacement and all invalid ownership/state/duplicate cases pass atomically.

## Phase 18 - Supersession Cycle Prevention

**Outcome:** Replacement chains cannot become ambiguous or cyclic.

**Parts:** Enforce active-source requirement, one replacement, no self-reference, no reverse/cyclic link, and bounded lookup behavior.

**Depends on:** Phase 17.

**Exit check:** Property tests over arbitrary transition sequences preserve the supersession invariants.

## Phase 19 - Pause Controls

**Outcome:** Emergency pause blocks risky writes while preserving verification and revocation.

**Parts:** Implement `set_paused`, require admin auth, emit `pause_changed`, block publish/supersede/upgrade, allow reads and revoke, and define idempotent repeated state.

**Depends on:** Phases 15-17.

**Exit check:** Every method is tested in paused and unpaused states.

## Phase 20 - Two-Step Admin Transfer

**Outcome:** Administration cannot move in one accidental transaction.

**Parts:** Implement `propose_admin` and `accept_admin`, require current/proposed admin auth respectively, support replacement/rejection policy, clear pending state, and emit proposal/change events.

**Depends on:** Phase 08.

**Exit check:** Correct, wrong signer, missing proposal, changed proposal, and completed-transfer tests pass.

## Phase 21 - Upgrade Authorization

**Outcome:** Contract code upgrades are explicit, reviewed, and traceable.

**Parts:** Implement `upgrade(new_wasm_hash)`, require admin auth and unpaused state, validate hash/schema compatibility policy, record old/new build hashes, and emit `upgraded`.

**Depends on:** Phases 19-20.

**Exit check:** Authorized test upgrade preserves reads; unauthorized, paused, invalid, and incompatible cases fail.

## Phase 22 - TTL Extension Policy

**Outcome:** Live instance, attestor, and evidence records remain available predictably.

**Parts:** Define thresholds/targets, extend on relevant writes, expose approved maintenance calls if needed, calculate archival windows, and document service cadence.

**Depends on:** Phases 07 and 13.

**Exit check:** Ledger-advance tests cover extension, near-expiry, archive, and maintenance recovery behavior.

## Phase 23 - Event Schema Verification

**Outcome:** Ledger events are a reliable indexing and recovery source.

**Parts:** Fix topics/payloads for initialized, attestor, publication, supersession, revocation, pause, admin, and upgrade events; publish decoding vectors and ordering expectations.

**Depends on:** Phases 08-22.

**Exit check:** Exact topic/payload tests and backend indexer fixtures pass.

## Phase 24 - Evidence Invariant Property Tests

**Outcome:** Arbitrary valid operation sequences preserve every documented invariant.

**Parts:** Generate admins, attestors, inputs, pauses, transfers, publish/supersede/revoke sequences; assert uniqueness, bounds, terminal revocation, acyclic replacement, privacy, read availability, versioning, and admin events.

**Depends on:** Phases 13-23.

**Exit check:** Reproducible seeded property suite passes with retained failure cases.

## Phase 25 - Evidence Fuzz Harnesses

**Outcome:** Decoding and transition surfaces tolerate hostile inputs.

**Parts:** Add fuzz targets for input decoding, ID encoding, count/bitmap boundaries, event decoding, transition sequences, and upgrade/version inputs; bound memory and execution.

**Depends on:** Phase 24.

**Exit check:** CI smoke corpus passes and scheduled fuzzing records crashes as regression fixtures.

## Phase 26 - SEP-45 Web-Auth Fixture Interface

**Outcome:** A test-only contract exposes the pinned `web_auth_verify` shape.

**Parts:** Implement expected function/arguments, explicit test-only labeling, deterministic nonce inputs, no hidden sub-invocations, and stable error cases.

**Depends on:** Phase 05 and pinned SEP-45 snapshot.

**Exit check:** Generated spec matches backend expectations and invocation inspection tests pass.

## Phase 27 - Web-Auth Fixture Behavior

**Outcome:** Success, rejection, expiry, replay, and malformed authorization cases are reproducible.

**Parts:** Add configurable results, nonce consumption, time bounds, auth context validation, events/evidence needed by tests, and deterministic setup/reset helpers for local/testnet fixtures.

**Depends on:** Phase 26.

**Exit check:** Backend-compatible success and negative vectors pass without fund custody.

## Phase 28 - Policy Account: Ed25519 and Thresholds

**Outcome:** A contract account fixture exercises common SEP-45 signing policies.

**Parts:** Implement single Ed25519 signer, multisignature threshold, signer configuration, authorization checks, and deterministic valid/invalid signature fixtures.

**Depends on:** Phase 05 and pinned wallet interface.

**Exit check:** Single, threshold, missing, duplicate, and unauthorized signer cases pass.

## Phase 29 - Policy Account: Time, Replay, and Rejection

**Outcome:** Policy-controlled negative cases are deterministic.

**Parts:** Add time-bounded authorization, nonce replay rejection, intentional rejection, additional-signer requirement, and custom-auth context validation.

**Depends on:** Phase 28.

**Exit check:** Boundary ledger times and replay sequences produce exact expected results.

## Phase 30 - Policy Account: Passkey-Compatible Mode

**Outcome:** The fixture can test secp256r1/passkey behavior when supported by the selected stack.

**Parts:** Confirm toolchain/interface support, implement the bounded signature mode, add vectors, label unsupported environments, and avoid any production-wallet claim.

**Depends on:** Phase 29.

**Exit check:** Supported-mode vectors pass; unsupported mode fails explicitly rather than simulating success.

## Phase 31 - Fixture Contract Isolation Tests

**Outcome:** Test fixtures cannot be confused with production evidence or wallets.

**Parts:** Test deployment labels, network restrictions, no fund-custody functions, no hidden calls, distinct admin identities, and documentation warnings.

**Depends on:** Phases 27 and 30.

**Exit check:** Release automation rejects fixture manifests lacking test-only metadata.

## Phase 32 - Resource Budget Baselines

**Outcome:** CPU, memory, storage, event, and TTL costs are measured and bounded.

**Parts:** Snapshot every public method, set regression tolerances, test maximum valid inputs and chains, estimate testnet/pubnet storage growth, and document maintenance costs.

**Depends on:** Phases 24-31.

**Exit check:** Budget CI passes and every increase requires explicit review.

## Phase 33 - Generated Contract Specifications

**Outcome:** Contract specs are authoritative, canonical, and releaseable.

**Parts:** Generate JSON and XDR specs for every contract, canonicalize semantic aliases, ASCII-sort entries/keys, record source/WASM hashes, and fail on drift.

**Depends on:** Complete public interfaces.

**Exit check:** Windows and Linux generation yields semantically identical canonical artifacts.

## Phase 34 - Generated TypeScript Bindings

**Outcome:** Backend and frontend consume deterministic released clients.

**Parts:** Generate TypeScript bindings from Phase 33 specs, pin generator version, normalize cross-platform output, add smoke calls/decoding tests, and package without manual edits.

**Depends on:** Phase 33.

**Exit check:** Regeneration is clean and consumers pass contract tests.

## Phase 35 - Local Deployment Tooling

**Outcome:** All contracts deploy and verify on a disposable local network.

**Parts:** Build/deploy/initialize scripts, identities, fixture configuration, code-hash checks, read/write smoke tests, teardown, and local manifest generation.

**Depends on:** Phases 04 and 31-34.

**Exit check:** A clean local environment deploys and independently verifies every manifest field.

## Phase 36 - Testnet Deployment Tooling

**Outcome:** Reviewed artifacts deploy repeatably to Stellar testnet.

**Parts:** Add network/passphrase checks, funded deployment identity policy, deploy/init/configure scripts, attestor/admin setup, fixture labels, code-hash/read/write verification, and transaction/ledger capture.

**Depends on:** Phase 35.

**Exit check:** Fresh testnet deployment produces a verified manifest and backend-compatible journey evidence.

## Phase 37 - Manifest Schema and Verification

**Outcome:** Deployment state is machine-verifiable rather than a prose claim.

**Parts:** Define network, passphrase, contract IDs, WASM hashes, source release, ledger/transaction, schema version, admin/attestor policy, fixture status, and verification timestamp; add RPC verifier.

**Depends on:** Phases 35-36.

**Exit check:** Wrong network, unknown hash, stale contract, altered manifest, and unreachable RPC fail closed.

## Phase 38 - Upgrade Rehearsal

**Outcome:** The documented release procedure is proven before production use.

**Parts:** Build replacement WASM, run all tests/budgets, deploy locally/testnet, re-index copied events, pause only if required, upgrade, verify schema/reads/new publish, and exercise rollback/forward-fix decisions.

**Depends on:** Phases 21, 23, 32, and 37.

**Exit check:** Old evidence remains correctly readable and new evidence publishes after rehearsal.

## Phase 39 - External Security Review

**Outcome:** Evidence registry risk receives independent review before pubnet use.

**Parts:** Supply architecture, auth/storage/TTL/upgrade docs, threat model, source/build artifacts, tests, budgets, deployment evidence, and scope; triage findings by severity.

**Depends on:** Phases 24-38.

**Exit check:** Review report is recorded and no production/pubnet claim precedes remediation.

## Phase 40 - Security Remediation and Retest

**Outcome:** Review findings are fixed and independently retested.

**Parts:** Implement narrow fixes, add regression/property/fuzz cases, regenerate artifacts, redeploy testnet candidate, update threat/compatibility notes, and obtain closure evidence.

**Depends on:** Phase 39.

**Exit check:** No unresolved critical finding remains and accepted residual risks are documented.

## Phase 41 - Tagged Contract Release

**Outcome:** Consumers receive a complete verifiable release bundle.

**Parts:** Publish optimized WASM/checksums, specs JSON/XDR, TypeScript bindings, source/toolchain/container metadata, manifests, policies, migration notes, security references, budget report, SBOM, provenance, and signed tag.

**Depends on:** Phase 40.

**Exit check:** Release verification from a clean environment succeeds without repository-local state.

## Phase 42 - Pubnet Decision and Deployment

**Outcome:** Pubnet evidence registry exists only after an explicit risk and cost decision.

**Parts:** Review interoperability need, audit closure, admin/attestor custody, storage/TTL cost, monitoring, incident/upgrade policy, deployment transaction, manifest, and public verification; do not deploy fixtures unless separately justified.

**Depends on:** Phase 41 and production governance approval.

**Exit check:** Either a verified pubnet manifest exists or a documented no-deploy decision keeps all claims testnet-only.

## Phase 43 - Long-Term Contract Operations

**Outcome:** Evidence remains verifiable across maintenance and releases.

**Parts:** Operate TTL maintenance, event indexing checks, admin/attestor reviews, cost/budget monitoring, dependency/toolchain updates, incident drills, supported-version policy, and additive upgrade planning.

**Depends on:** Phase 41 or 42.

**Exit check:** Recurring owners, schedules, alerts, recovery checks, and compatibility reviews are active.

## Contracts Completion Gate

The contracts repository is complete for production evidence use only when all applicable phases are checked; all authorization, transition, invariant, event, TTL, upgrade, property, fuzz, and budget tests pass; builds and generated artifacts are deterministic across supported platforms; local and testnet manifests verify against RPC; external findings are remediated; released consumers pin the exact artifacts; and no record or event contains a report body, URL token, raw domain, customer field, secret, or fund-custody behavior.
