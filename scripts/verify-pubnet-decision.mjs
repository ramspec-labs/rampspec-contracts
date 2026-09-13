import { readFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { validateFindingsRegister } from "./security-readiness-core.mjs";
import { validatePubnetDecision } from "./pubnet-decision-core.mjs";

const decision = JSON.parse(await readFile("deployments/pubnet/decision.json", "utf8"));
const review = JSON.parse(await readFile("security/review-status.json", "utf8"));
const findings = JSON.parse(await readFile("security/findings.json", "utf8"));
const security = validateFindingsRegister(findings, review);
if (decision.gates.securityReview !== review.status || decision.gates.remediation !== findings.status) {
  throw new Error("pubnet decision security gates do not match repository status");
}
const result = validatePubnetDecision(decision, security.cleared);
if (result.deploy) {
  if (!process.argv.includes("--live")) throw new Error("approved pubnet state requires --live manifest verification");
  execFileSync("node", [
    "scripts/verify-deployment-manifest.mjs", resolve(decision.pubnetManifest), "--live", "--environment", "pubnet",
  ], { stdio: "inherit" });
  process.stdout.write("pubnet deployment decision and manifest verified\n");
} else {
  process.stdout.write(`pubnet deployment blocked: ${result.reason}\n`);
}
