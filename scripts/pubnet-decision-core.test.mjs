import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { validatePubnetDecision } from "./pubnet-decision-core.mjs";

const decision = JSON.parse(await readFile("deployments/pubnet/decision.json", "utf8"));

test("accepts the documented no-deploy decision", () => {
  assert.equal(validatePubnetDecision(decision, false).deploy, false);
});

test("no-deploy state cannot claim a pubnet manifest or fixtures", () => {
  assert.throws(() => validatePubnetDecision({ ...decision, pubnetManifest: "manifest.json" }, false), /cannot claim deployment/);
  assert.throws(() => validatePubnetDecision({ ...decision, fixturesAllowed: true }, false), /fixtures are prohibited/);
  assert.throws(() => validatePubnetDecision({ ...decision, currentClaim: "pubnet" }, false), /cannot permit pubnet/);
  assert.throws(() => validatePubnetDecision({ ...decision, gates: { ...decision.gates, monitoring: "maybe" } }, false), /gate state is invalid/);
});

test("deploy state requires security and every governance gate", () => {
  const deploy = {
    ...decision,
    decision: "deploy",
    currentClaim: "pubnet",
    maximumClaimUntilReview: "pubnet",
    pubnetManifest: "deployments/pubnet/manifest.json",
    deploymentTransaction: "a".repeat(64),
    gates: {
      interoperabilityNeed: "approved", securityReview: "completed", remediation: "closed",
      signedRelease: "present", adminCustody: "approved", attestorCustody: "approved",
      ttlCostBudget: "approved", monitoring: "active", incidentAndUpgradePolicy: "approved",
    },
  };
  assert.throws(() => validatePubnetDecision(deploy, false), /security clearance/);
  assert.equal(validatePubnetDecision(deploy, true).deploy, true);
  deploy.gates.monitoring = "approved";
  assert.throws(() => validatePubnetDecision(deploy, true), /not all satisfied/);
});
