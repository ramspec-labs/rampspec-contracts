# Evidence Registry Threat Model

## Assets and Trust

The assets are immutable evidence commitments, active/revoked/superseded lifecycle truth, publisher identity, event history, administrator and attestor policy, and compatible code/spec hashes. The administrator is trusted only for attestor policy, pause, transfer, TTL maintenance, and reviewed upgrades. Each attestor is trusted to bind truthful off-chain evidence, but cannot alter another publisher's records. RPC, Horizon, indexers, generated clients, and deployment operators are untrusted until their outputs are verified against network passphrase, contract code, committed specifications, and manifest integrity.

## Adversaries

- An unauthenticated caller attempting publication, revocation, supersession, administration, or upgrade.
- A registered attestor publishing malformed, duplicate, oversized, misleading, or cross-network commitments.
- A compromised administrator attempting silent policy changes, fixture substitution, destructive upgrades, or availability loss.
- A malicious RPC or build environment returning a wrong network, stale code, altered spec, transaction omission, or nondeterministic artifact.
- An indexer or consumer dropping, reordering, or mis-decoding lifecycle events.
- A dependency or CI compromise changing build inputs or release contents.

## Controls

Explicit Soroban authorization protects every write; reads remain public. Versioned storage keys and fixed errors preserve compatibility. Validation rejects zero hashes, unsupported bitmaps, invalid scores/counts, duplicates, terminal-state reuse, ownership violations, cycles, and overlong replacement chains. Events have snapshot tests. Pause is fail-closed for publication, replacement, and upgrade while reads and revocation remain available. Administrator transfer is two-step. Upgrade requires administrator authorization, a nonzero unseen hash, and an unpaused registry. TTL maintenance is explicit and budgeted.

Web-auth fixture events publish the account and a nonce hash, or the admin and a nonce hash, and never the raw nonce, domain secrets, or rejected argument text. A rejected verification or an unknown-nonce reset returns before any write or event. Configuration changes bump a separate generation counter; an identical configure call and a rejected time window leave that counter and the event stream unchanged.

**Registry events are public and must contain only approved public information:** evidence identifiers, cryptographic commitments (report_hash, target_hash, suite_hash, reason_hash, metadata_hash), administrator/publisher/attestor/revoker/authorizer addresses, status flags, schema/version metadata, and authorization transition metadata. Events must never expose report bodies, raw report content, URLs, domains, API keys, passwords, access tokens, credentials, customer personal information, KYC data, or other private application data. Event schema and privacy tests enforce this boundary and fail if unapproved fields are introduced.

Reproducible builds, canonical specs, generated clients, locked dependencies, fixture manifests, test-only ABI labels, deployment seals, released-WASM allowlisting, live code fetches, schema reads, event transaction capture, and secret scanning protect the supply and deployment paths. Passkey authorization binds the challenge to the exact Soroban payload and rejects malformed or high-S P-256 signatures before verification or replay writes. Property and fuzz tests cover state-machine and hostile-input behavior.

## Policy Account Fixture Event Privacy

Policy configuration events are public and indexable but must never expose sensitive authentication material. The `PolicyConfigurationChanged` event contains only the change type, version, policy commitment hash, signature mode, signer count, and threshold. Raw Ed25519 public keys, Ed25519 signatures, passkey public keys, passkey signatures, authenticator data, client authentication payloads, and rejected inputs never appear in events. Events are published only after successful, validated, authorized, non-idempotent configuration mutations. Failed operations, invalid inputs, unauthorized attempts, and idempotent updates produce no events and no partial state changes. This prevents information leakage through failed mutations while enabling external indexers to track legitimate policy evolution.

## Residual Risks

An authorized attestor can commit false off-chain claims; consumers must verify the underlying report. Administrator or attestor key compromise remains an operational risk. Ledger retention and fee changes can affect availability and cost. The bounded supersession chain constrains on-chain traversal but indexers still need complete event history. Upgrade correctness depends on external review, rehearsal, signer custody, and monitoring. Cryptography is delegated to Soroban host functions and inherits platform risk. Test fixtures intentionally model only pinned scenarios and are unsafe as production identity policy.

## Required Independent Tests

Reviewers should construct unauthorized and cross-attestor transitions, storage corruption assumptions, extreme ledger/TTL boundaries, malformed contract values, event/index replay, dependency and build reproduction, wrong-network and wrong-code manifests, repeated/failed upgrades, and fixture leakage attempts. Findings must distinguish exploitable contract behavior, deployment/operations weaknesses, and out-of-scope application concerns.
