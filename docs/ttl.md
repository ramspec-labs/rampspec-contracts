# Storage TTL Policy

RampSpec assumes five-second ledgers for human-readable estimates. Ledger counts, not wall-clock timestamps, are authoritative.

| State | Extend below | Extend to | Approximate target |
| --- | ---: | ---: | ---: |
| Contract instance and code | 103,680 ledgers | 2,073,600 ledgers | 120 days |
| Attestors, evidence, and indexes | 518,400 ledgers | 3,110,400 ledgers | 180 days |

Every relevant write extends its state. Successful `schema_version`, `get_attestor`, `get_evidence`, and `is_active` reads extend only the contract instance TTL; missing attestor or evidence reads do not extend TTL or create state. This keeps frequently verified registry configuration and code live while leaving persistent-entry retention under explicit operator control. Callers pay the additional instance-rent bump when the threshold is reached.

The operator maintenance service must inspect and refresh live entries at least every 30 days using `maintain_attestor` and `maintain_evidence`; calls are administrator-authorized and fail for unknown entries. Evidence maintenance validates the active uniqueness index before any TTL change, repairs a missing index for an active record, rejects a conflicting index, and refreshes the forward supersession link where applicable.

Maintenance cannot restore an archived entry from inside the contract. Operators must submit the network restore-footprint transaction first, then call maintenance. Contract events and signed release/deployment manifests are the recovery source if an indexer cache is lost. Alert at 45 days remaining, escalate at 30 days, and freeze new publication if the instance drops below 14 days without a successful extension.

## Replay Marker TTL Policy

The `policy-account-fixture` contract uses temporary storage for replay markers (`UsedPayload` keys). Temporary entries in Soroban have a fixed TTL of 100 ledgers (approximately 8 minutes at 5-second ledger intervals).

- **Threat model**: Replay protection covers the authorization horizon for a single transaction submission. The 100-ledger window accounts for network latency, transaction queueing, and ledger finality.
- **TTL behavior**: A replay marker is created only after successful authorization (`env.storage().temporary().set(&used_key, &true)`). Failed authorization attempts (invalid signature, wrong context, insufficient weight, etc.) do not create a replay marker.
- **Extension**: Replay markers are never explicitly extended. They expire automatically after 100 ledgers. This bounds the replay protection window and prevents storage bloat from accumulated nonces.
- **Compatibility**: The TTL is a Soroban protocol constant and cannot be changed by the contract. Consumers must not rely on replay markers persisting beyond a single ledger window.
