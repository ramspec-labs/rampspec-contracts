# Storage TTL Policy

RampSpec assumes five-second ledgers for human-readable estimates. Ledger counts, not wall-clock timestamps, are authoritative.

| State | Extend below | Extend to | Approximate target |
| --- | ---: | ---: | ---: |
| Contract instance and code | 103,680 ledgers | 2,073,600 ledgers | 120 days |
| Attestors, evidence, and indexes | 518,400 ledgers | 3,110,400 ledgers | 180 days |

Every relevant write extends its state. The operator maintenance service must inspect and refresh live entries at least every 30 days using `maintain_attestor` and `maintain_evidence`; calls are administrator-authorized and fail for unknown entries. Evidence maintenance also refreshes the active uniqueness index or forward supersession link where applicable.

Maintenance cannot restore an archived entry from inside the contract. Operators must submit the network restore-footprint transaction first, then call maintenance. Contract events and signed release/deployment manifests are the recovery source if an indexer cache is lost. Alert at 45 days remaining, escalate at 30 days, and freeze new publication if the instance drops below 14 days without a successful extension.
