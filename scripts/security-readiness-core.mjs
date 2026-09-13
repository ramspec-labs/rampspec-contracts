import { validateReviewStatus } from "./security-review-core.mjs";

const HASH = /^[0-9a-f]{64}$/;
const REVISION = /^[0-9a-f]{40}$/;
const SEVERITIES = new Set(["critical", "high", "moderate", "low", "informational"]);
const STATUSES = new Set(["open", "accepted", "fixed", "retested"]);
const FINDING_FIELDS = [
  "id", "severity", "title", "status", "owner", "affectedPaths", "fixCommit",
  "regressionTests", "propertyOrFuzzCoverage", "artifactsRegenerated", "testnetManifest",
  "threatModelUpdated", "compatibilityUpdated", "retestProvider", "closureEvidence", "notes",
];

export function validateFindingsRegister(register, reviewStatus) {
  validateReviewStatus(reviewStatus);
  const fields = ["schemaVersion", "status", "reviewReportSha256", "findings", "acceptedResidualRisks", "updatedAt"];
  if (!register || JSON.stringify(Object.keys(register).sort()) !== JSON.stringify(fields.sort())) throw new Error("findings register fields do not match schema");
  if (register.schemaVersion !== 1 || !["awaiting-review", "triage", "remediation", "retest", "closed"].includes(register.status)) {
    throw new Error("findings register status is invalid");
  }
  if (!Array.isArray(register.findings) || !Array.isArray(register.acceptedResidualRisks)) throw new Error("findings and residual risks must be arrays");
  if (!Number.isFinite(Date.parse(register.updatedAt))) throw new Error("findings update timestamp is invalid");
  if (reviewStatus.status !== "completed") {
    if (register.status !== "awaiting-review" || register.reviewReportSha256 !== null || register.findings.length !== 0) {
      throw new Error("findings must remain empty and awaiting review until a report exists");
    }
    return { cleared: false, reason: "independent review is not complete" };
  }
  if (!HASH.test(register.reviewReportSha256 ?? "") || register.reviewReportSha256 !== reviewStatus.reportSha256) {
    throw new Error("findings register is not bound to the independent report");
  }
  if (register.status === "awaiting-review") throw new Error("completed review requires findings triage");
  const ids = new Set();
  const residuals = new Map(register.acceptedResidualRisks.map((risk) => [risk.findingId, risk]));
  if (residuals.size !== register.acceptedResidualRisks.length) throw new Error("residual risk entries are duplicated");
  for (const finding of register.findings) {
    if (!finding || JSON.stringify(Object.keys(finding).sort()) !== JSON.stringify([...FINDING_FIELDS].sort())) {
      throw new Error("finding fields do not match schema");
    }
    if (!/^RS-[0-9]{3}$/.test(finding.id) || ids.has(finding.id)) throw new Error("finding IDs are invalid or duplicated");
    ids.add(finding.id);
    if (!SEVERITIES.has(finding.severity) || !STATUSES.has(finding.status) || !finding.title) throw new Error(`${finding.id} classification is invalid`);
    for (const field of ["affectedPaths", "regressionTests", "propertyOrFuzzCoverage"]) {
      if (!Array.isArray(finding[field]) || finding[field].some((value) => typeof value !== "string" || !value)) {
        throw new Error(`${finding.id} ${field} is invalid`);
      }
    }
    if (finding.status === "accepted") {
      if (finding.severity === "critical" || finding.severity === "high") throw new Error(`${finding.id} severity cannot be accepted`);
      const risk = residuals.get(finding.id);
      if (!risk?.rationale || !risk.approver || !Number.isFinite(Date.parse(risk.approvedAt))) throw new Error(`${finding.id} residual risk is undocumented`);
      continue;
    }
    if (finding.status === "fixed" || finding.status === "retested") {
      if (!REVISION.test(finding.fixCommit ?? "") || !finding.owner || finding.regressionTests.length === 0
        || !finding.artifactsRegenerated || !finding.testnetManifest || !finding.threatModelUpdated
        || !finding.compatibilityUpdated) {
        throw new Error(`${finding.id} remediation evidence is incomplete`);
      }
    }
    if (finding.status === "retested" && (!finding.retestProvider || !finding.closureEvidence)) {
      throw new Error(`${finding.id} independent retest evidence is incomplete`);
    }
  }
  for (const [id, risk] of residuals) {
    if (!ids.has(id)) throw new Error(`residual risk references unknown finding ${id}`);
    const fields = ["findingId", "rationale", "approver", "approvedAt"];
    if (!risk || JSON.stringify(Object.keys(risk).sort()) !== JSON.stringify(fields.sort())) {
      throw new Error(`${id} residual risk fields do not match schema`);
    }
  }
  const unresolvedCritical = register.findings.some((finding) => finding.severity === "critical" && finding.status !== "retested");
  const unresolvedHigh = register.findings.some((finding) => finding.severity === "high" && finding.status !== "retested");
  const incomplete = register.findings.some((finding) => !["retested", "accepted"].includes(finding.status));
  const cleared = register.status === "closed" && !unresolvedCritical && !unresolvedHigh && !incomplete;
  return { cleared, reason: cleared ? null : "review findings are not independently closed" };
}
