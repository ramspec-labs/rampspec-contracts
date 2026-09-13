const GATE_NAMES = [
  "interoperabilityNeed", "securityReview", "remediation", "signedRelease", "adminCustody",
  "attestorCustody", "ttlCostBudget", "monitoring", "incidentAndUpgradePolicy",
];
const GATE_STATES = new Set([
  "not-approved", "not-started", "awaiting-review", "absent", "not-active", "documented",
  "approved", "completed", "closed", "active", "present",
]);
const DEPLOY_GATE_STATES = {
  interoperabilityNeed: "approved",
  securityReview: "completed",
  remediation: "closed",
  signedRelease: "present",
  adminCustody: "approved",
  attestorCustody: "approved",
  ttlCostBudget: "approved",
  monitoring: "active",
  incidentAndUpgradePolicy: "approved",
};

export function validatePubnetDecision(decision, securityCleared) {
  const fields = [
    "schemaVersion", "decision", "scope", "currentClaim", "maximumClaimUntilReview",
    "fixturesAllowed", "pubnetManifest", "deploymentTransaction", "decidedAt",
    "nextReviewAfter", "gates", "reasons",
  ];
  if (!decision || JSON.stringify(Object.keys(decision).sort()) !== JSON.stringify(fields.sort())) throw new Error("pubnet decision fields do not match schema");
  if (decision.schemaVersion !== 1 || !['no-deploy', 'deploy'].includes(decision.decision) || decision.scope !== "evidence-registry") {
    throw new Error("pubnet decision is invalid");
  }
  if (decision.fixturesAllowed !== false) throw new Error("test fixtures are prohibited on pubnet");
  if (!Number.isFinite(Date.parse(decision.decidedAt)) || !Number.isFinite(Date.parse(decision.nextReviewAfter))) throw new Error("pubnet decision timestamps are invalid");
  if (Date.parse(decision.nextReviewAfter) <= Date.parse(decision.decidedAt)) throw new Error("pubnet decision review date must follow its decision date");
  if (!Array.isArray(decision.reasons) || decision.reasons.length === 0 || decision.reasons.some((reason) => typeof reason !== "string" || !reason)) {
    throw new Error("pubnet decision requires reasons");
  }
  if (!decision.gates || JSON.stringify(Object.keys(decision.gates).sort()) !== JSON.stringify([...GATE_NAMES].sort())) {
    throw new Error("pubnet gate inventory is incomplete");
  }
  if (Object.values(decision.gates).some((value) => !GATE_STATES.has(value))) throw new Error("pubnet gate state is invalid");
  if (decision.decision === "no-deploy") {
    if (decision.currentClaim === "pubnet" || decision.maximumClaimUntilReview !== "testnet-only") throw new Error("no-deploy decision cannot permit pubnet claims");
    if (decision.pubnetManifest !== null || decision.deploymentTransaction !== null) throw new Error("no-deploy decision cannot claim deployment evidence");
    return { deploy: false, reason: decision.reasons.join(" ") };
  }
  if (!securityCleared) throw new Error("pubnet deployment requires security clearance");
  if (decision.currentClaim !== "pubnet" || decision.maximumClaimUntilReview !== "pubnet") throw new Error("approved deployment claim fields are inconsistent");
  if (!decision.pubnetManifest || !/^[0-9a-f]{64}$/.test(decision.deploymentTransaction ?? "")) throw new Error("pubnet deployment evidence is incomplete");
  if (Object.entries(DEPLOY_GATE_STATES).some(([name, expected]) => decision.gates[name] !== expected)) {
    throw new Error("pubnet governance gates are not all satisfied");
  }
  return { deploy: true, reason: null };
}
