import assert from "node:assert/strict";
import test from "node:test";
import { validateFindingsRegister } from "./security-readiness-core.mjs";

const pendingReview = {
  schemaVersion: 1, status: "not-started", reviewTargetRevision: null, provider: null,
  reportPath: null, reportSha256: null, startedAt: null, completedAt: null,
  productionClaimsAllowed: false, pubnetAllowed: false, updatedAt: "2026-09-13T00:00:00Z",
};
const completedReview = {
  ...pendingReview, status: "completed", reviewTargetRevision: "a".repeat(40), provider: "Reviewer",
  reportPath: "security/reports/report.pdf", reportSha256: "b".repeat(64),
  startedAt: "2026-09-01T00:00:00Z", completedAt: "2026-09-10T00:00:00Z",
};
const pendingRegister = {
  schemaVersion: 1, status: "awaiting-review", reviewReportSha256: null,
  findings: [], acceptedResidualRisks: [], updatedAt: "2026-09-13T00:00:00Z",
};
const finding = {
  id: "RS-001", severity: "critical", title: "Authorization bypass", status: "retested", owner: "contracts",
  affectedPaths: ["contracts/evidence-registry/src/lib.rs"], fixCommit: "c".repeat(40),
  regressionTests: ["authorization_bypass_regression"], propertyOrFuzzCoverage: ["fuzz_publish_input"],
  artifactsRegenerated: true, testnetManifest: "deployments/testnet/retest.json",
  threatModelUpdated: true, compatibilityUpdated: true, retestProvider: "Independent Reviewer",
  closureEvidence: "security/reports/RS-001-retest.pdf", notes: "",
};

test("honest pending review is valid but uncleared", () => {
  assert.deepEqual(validateFindingsRegister(pendingRegister, pendingReview), {
    cleared: false, reason: "independent review is not complete",
  });
});

test("critical findings require independent retest", () => {
  const register = { ...pendingRegister, status: "closed", reviewReportSha256: "b".repeat(64), findings: [{ ...finding, status: "fixed", retestProvider: null, closureEvidence: null }] };
  assert.deepEqual(validateFindingsRegister(register, completedReview).cleared, false);
  assert.throws(() => validateFindingsRegister({ ...register, findings: [{ ...finding, status: "accepted" }] }, completedReview), /cannot be accepted/);
});

test("fully retested findings clear the security gate", () => {
  const register = { ...pendingRegister, status: "closed", reviewReportSha256: "b".repeat(64), findings: [finding] };
  assert.deepEqual(validateFindingsRegister(register, completedReview), { cleared: true, reason: null });
});

test("accepted lower risk requires explicit approval", () => {
  const accepted = { ...finding, severity: "low", status: "accepted", fixCommit: null };
  const register = { ...pendingRegister, status: "closed", reviewReportSha256: "b".repeat(64), findings: [accepted] };
  assert.throws(() => validateFindingsRegister(register, completedReview), /undocumented/);
  register.acceptedResidualRisks = [{ findingId: "RS-001", rationale: "Bounded impact", approver: "Security owner", approvedAt: "2026-09-12T00:00:00Z" }];
  assert.equal(validateFindingsRegister(register, completedReview).cleared, true);
});

test("malformed finding evidence fails with a validation error", () => {
  const malformed = { ...finding };
  delete malformed.regressionTests;
  const register = { ...pendingRegister, status: "retest", reviewReportSha256: "b".repeat(64), findings: [malformed] };
  assert.throws(() => validateFindingsRegister(register, completedReview), /fields do not match schema/);
});
