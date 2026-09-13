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

Reproducible builds, canonical specs, generated clients, locked dependencies, fixture manifests, test-only ABI labels, deployment seals, released-WASM allowlisting, live code fetches, schema reads, event transaction capture, and secret scanning protect the supply and deployment paths. Property and fuzz tests cover state-machine and hostile-input behavior.

## Residual Risks

An authorized attestor can commit false off-chain claims; consumers must verify the underlying report. Administrator or attestor key compromise remains an operational risk. Ledger retention and fee changes can affect availability and cost. The bounded supersession chain constrains on-chain traversal but indexers still need complete event history. Upgrade correctness depends on external review, rehearsal, signer custody, and monitoring. Cryptography is delegated to Soroban host functions and inherits platform risk. Test fixtures intentionally model only pinned scenarios and are unsafe as production identity policy.

## Required Independent Tests

Reviewers should construct unauthorized and cross-attestor transitions, storage corruption assumptions, extreme ledger/TTL boundaries, malformed contract values, event/index replay, dependency and build reproduction, wrong-network and wrong-code manifests, repeated/failed upgrades, and fixture leakage attempts. Findings must distinguish exploitable contract behavior, deployment/operations weaknesses, and out-of-scope application concerns.
