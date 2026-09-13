# Independent Security Review Scope

## Objective

Review the evidence registry as a production-oriented Soroban contract before any pubnet deployment or production-security claim. Confirm that an authorized attestor can publish only valid immutable commitments, lifecycle transitions cannot corrupt or hide prior evidence, administrative actions are explicit and traceable, storage remains available under the documented TTL policy, and upgrades cannot bypass governance.

## In Scope

- All evidence-registry and shared-type source, generated contract specifications, bindings, errors, storage keys, events, authorization, pause behavior, administrator transfer, TTL maintenance, and upgrade behavior.
- Unit, event-schema, property, fuzz, resource-budget, reproducibility, deployment-manifest, and upgrade-rehearsal controls.
- Build inputs in `Cargo.lock`, `Cargo.toml`, and `rust-toolchain.toml`; committed artifact hashes and release packaging.
- Local and testnet deployment scripts, signer assumptions, RPC verification, event indexing, and manifest integrity.
- Fixture isolation controls. The web-auth and policy-account contracts are reviewed only to confirm deterministic test behavior and to prevent production use.

## Required Questions

1. Can any unregistered, disabled, or unauthenticated address publish or replace evidence?
2. Can a caller mutate immutable record content, revive revoked evidence, create a supersession cycle, or bypass the chain bound?
3. Can pause, administrator transfer, upgrade, or TTL behavior make valid evidence unavailable or permit an unauthorized state transition?
4. Are every identifier, hash preimage, event field, and network discriminant unambiguous and stable across clients?
5. Can malformed input, hostile authorization payloads, resource exhaustion, storage growth, or replay produce unsafe behavior?
6. Can test-only fixtures or secrets enter a production deployment or release bundle?

## Deliverables

The reviewer provides a revision-bound report with methodology, limitations, severity-rated findings, reproductions, and recommended remediation. The repository records the report path and SHA-256 without modifying the report. Each finding receives an owner and closure evidence in the remediation register. The same reviewer or another independent reviewer must retest security-relevant fixes.

## Exclusions and Boundaries

The registry does not custody funds, execute transfers, process KYC, store report bodies or customer data, certify compliance, or validate off-chain report truth. Backend signing custody, frontend presentation, off-chain storage, and organization processes are separate review scopes except where their assumptions affect contract safety. A review does not turn test fixtures into production authentication systems.

## Acceptance

Preparation is not review completion. `security/review-status.json` remains `not-started` until an independent provider is engaged, then `in-progress`, then `completed` only when a report is present and hash-bound. Production and pubnet flags remain false until remediation and independent retest are complete.
