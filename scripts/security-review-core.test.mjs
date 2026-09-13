import assert from "node:assert/strict";
import test from "node:test";
import { validateReviewStatus } from "./security-review-core.mjs";

const pending = {
  schemaVersion: 1, status: "not-started", reviewTargetRevision: null, provider: null,
  reportPath: null, reportSha256: null, startedAt: null, completedAt: null,
  productionClaimsAllowed: false, pubnetAllowed: false, updatedAt: "2026-09-13T00:00:00.000Z",
};

test("accepts an honest not-started review state", () => {
  assert.equal(validateReviewStatus(pending), pending);
});

test("pending review cannot authorize production claims", () => {
  assert.throws(() => validateReviewStatus({ ...pending, productionClaimsAllowed: true }), /cannot allow production/);
});

test("completed review requires report evidence and still awaits remediation", () => {
  const completed = {
    ...pending,
    status: "completed",
    reviewTargetRevision: "a".repeat(40),
    provider: "Independent Reviewer",
    reportPath: "security/reports/review.pdf",
    reportSha256: "b".repeat(64),
    startedAt: "2026-09-01T00:00:00.000Z",
    completedAt: "2026-09-10T00:00:00.000Z",
  };
  assert.doesNotThrow(() => validateReviewStatus(completed));
  assert.throws(() => validateReviewStatus({ ...completed, pubnetAllowed: true }), /cannot authorize production/);
});
