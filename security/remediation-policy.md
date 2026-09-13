# Security Remediation and Retest

Copy every independent-review finding into `security/findings.json` without weakening its assigned severity. IDs use `RS-NNN`; each finding names an owner, affected paths, narrow fix commit, regression tests, and any property or fuzz coverage. Keep the register status aligned with actual work: `triage`, `remediation`, `retest`, then `closed` only after independent closure evidence exists.

Security fixes follow the same one-change audit discipline as contract work. Reproduce the finding, implement the narrow correction, add a regression that fails against the affected revision, run the full contract suite and fuzz checks, regenerate WASM/specs/bindings, recheck resource budgets, and deploy a fresh testnet candidate. Update the threat model and compatibility notes when assumptions, ABI, storage, errors, events, or operational behavior change.

A finding becomes `fixed` only when its commit, tests, regenerated artifacts, updated documentation, and verified testnet manifest are recorded. It becomes `retested` only when an independent provider and immutable closure evidence are recorded. Critical findings cannot be accepted as residual risk. High findings require retest; lower severities may be accepted only with a named approver, dated rationale, and a matching entry in `acceptedResidualRisks`.

Run `node scripts/verify-security-readiness.mjs --allow-pending` in ordinary CI to validate honest pending state. Release and pubnet jobs must run `node scripts/verify-security-readiness.mjs --require-cleared`, which fails unless the independent review is complete, its report hash matches the findings register, the register is closed, and no finding lacks required retest or acceptance evidence.
