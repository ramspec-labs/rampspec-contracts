# Pubnet No-Deploy Decision

RampSpec will not deploy the evidence registry or either fixture contract to Stellar pubnet in this release state. Current evidence supports deterministic source, tests, WASM, clients, and a non-publishable release candidate only. It does not include an independent security review, closed remediation, verified testnet deployment, signed release, approved administrator/attestor custody, funded TTL budget, or active monitoring.

All public statements must remain limited to local verification. After a real testnet deployment is independently verified, statements may describe that exact testnet evidence but must not imply pubnet availability, production readiness, certification, or security approval.

The next decision review occurs only after the listed gates materially change, and no later than the date in `decision.json`. Approval requires a demonstrated interoperability need, completed independent review and retest, a clean signed release, separate administrator and attestor custody approvals, measured TTL and transaction costs with budget ownership, active event/health monitoring, and an exercised incident/upgrade runbook.

If governance approves deployment, replace the decision atomically with a `deploy` record referencing the verified pubnet manifest and deployment transaction. The pubnet manifest must contain only the evidence registry; fixture fields remain null and fixture deployment needs a separate written decision. Run `node scripts/verify-pubnet-decision.mjs --live` before making any pubnet claim.
