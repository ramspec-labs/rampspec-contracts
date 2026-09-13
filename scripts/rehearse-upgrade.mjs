import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, join, resolve } from "node:path";
import { addGlobalArgs, validateDeploymentManifest, verifyKnownWasm } from "./deployment-manifest-core.mjs";
import { assertIdentityAlias, recoveryDecision } from "./upgrade-rehearsal-core.mjs";

const preflightOnly = process.argv.includes("--preflight-only");
const manifestArgument = process.argv.find((argument, index) => index > 1 && !argument.startsWith("--"));
const outputDirectory = resolve("artifacts/upgrade-rehearsals", new Date().toISOString().replaceAll(":", "-"));
const checks = {};

function run(command, args) {
  execFileSync(command, args, { stdio: "inherit" });
}

function record(name, callback) {
  callback();
  checks[name] = true;
}

record("format", () => run("cargo", ["fmt", "--all", "--", "--check"]));
record("clippy", () => run("cargo", ["clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]));
record("tests", () => run("cargo", ["test", "--locked", "--workspace"]));
if (process.platform === "win32") {
  record("reproducibility", () => run("powershell", ["-NoProfile", "-File", "scripts/verify-reproducible.ps1"]));
  record("generatedArtifacts", () => run("powershell", ["-NoProfile", "-File", "scripts/verify-generated.ps1"]));
} else {
  record("reproducibility", () => run("bash", ["scripts/verify-reproducible.sh"]));
  record("generatedArtifacts", () => run("bash", ["scripts/verify-generated.sh"]));
}
record("resourceBudgets", () => run("node", ["scripts/verify-resource-budgets.mjs", "artifacts/wasm"]));

if (preflightOnly) {
  process.stdout.write(`${JSON.stringify({ schemaVersion: 1, mode: "preflight", checks }, null, 2)}\n`);
  process.exit(0);
}
if (!manifestArgument) throw new Error("provide a local or testnet deployment manifest, or use --preflight-only");

const manifestPath = resolve(manifestArgument);
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const specs = JSON.parse(await readFile("artifacts/specs/manifest.json", "utf8"));
validateDeploymentManifest(manifest);
verifyKnownWasm(manifest, specs);
if (!['local', 'testnet'].includes(manifest.environment)) throw new Error("upgrade rehearsal is limited to local and testnet");
record("manifestOffline", () => run("node", ["scripts/verify-deployment-manifest.mjs", manifestPath, "--environment", manifest.environment]));
record("manifestLive", () => run("node", ["scripts/verify-deployment-manifest.mjs", manifestPath, "--live", "--environment", manifest.environment]));

const defaultAdmin = manifest.environment === "local" ? "registry-admin" : undefined;
const defaultAttestor = manifest.environment === "local" ? "attestor" : undefined;
const adminIdentity = assertIdentityAlias(process.env.RAMPSPEC_REHEARSAL_ADMIN_IDENTITY ?? defaultAdmin, "admin identity");
const attestorIdentity = assertIdentityAlias(process.env.RAMPSPEC_REHEARSAL_ATTESTOR_IDENTITY ?? defaultAttestor, "attestor identity");
if (manifest.environment === "testnet" && process.env.RAMPSPEC_CONFIRM_UPGRADE !== "rehearse-reviewed-upgrade-on-testnet") {
  throw new Error("RAMPSPEC_CONFIRM_UPGRADE must explicitly authorize the testnet rehearsal");
}
const configDirectory = process.env.RAMPSPEC_STELLAR_CONFIG_DIR;
const networkArgs = ["--rpc-url", manifest.rpcUrl, "--network-passphrase", manifest.networkPassphrase];
if (configDirectory) networkArgs.push("--config-dir", resolve(configDirectory));
function stellar(args) {
  return execFileSync("stellar", addGlobalArgs(args, networkArgs), { encoding: "utf8" }).trim();
}
const publicKey = (identity) => execFileSync(
  "stellar", addGlobalArgs(["keys", "public-key", identity], configDirectory ? ["--config-dir", resolve(configDirectory)] : []),
  { encoding: "utf8" },
).trim();
if (publicKey(adminIdentity) !== manifest.administrators.evidenceRegistry) throw new Error("admin identity does not match manifest");
if (publicKey(attestorIdentity) !== manifest.attestor) throw new Error("attestor identity does not match manifest");

await mkdir(outputDirectory, { recursive: true });
const registry = manifest.contracts.evidenceRegistry;
const startLedger = manifest.latestLedger;
const events = () => JSON.parse(stellar([
  "events", "--start-ledger", String(startLedger), "--count", "100", "--output", "json", "--id", registry,
]));
const invoke = (source, args) => stellar([
  "contract", "invoke", "--id", registry, "--source-account", source, "--", ...args,
]);
const specsHash = createHash("sha256").update(await readFile("artifacts/specs/manifest.json")).digest("hex");
const evidenceInput = (suffix) => JSON.stringify({
  publisher: manifest.attestor,
  report_hash: createHash("sha256").update(`report:${manifest.sourceRevision}:${suffix}`).digest("hex"),
  target_hash: createHash("sha256").update(`target:${manifest.sourceRevision}:${suffix}`).digest("hex"),
  suite_hash: createHash("sha256").update(`suite:${manifest.sourceRevision}`).digest("hex"),
  specs_hash: specsHash,
  artifact_root: manifest.wasmSha256.evidenceRegistry,
  network: manifest.environment === "testnet" ? "Testnet" : "Standalone",
  protocol_bitmap: 1,
  score_bps: 10000,
  passed: 1,
  failed: 0,
  warnings: 0,
  skipped: 0,
});

let integrityValid = true;
let oldEvidenceReadable = false;
let newEvidenceWritable = false;
let oldEvidenceId;
let newEvidenceId;
let failure;
const preUpgradeEvents = events();
await writeFile(join(outputDirectory, "pre-upgrade-events.json"), `${JSON.stringify(preUpgradeEvents, null, 2)}\n`, "utf8");
try {
  oldEvidenceId = invoke(attestorIdentity, ["publish_evidence", "--input", evidenceInput("before")]).replaceAll('"', "");
  const replacementHash = manifest.wasmSha256.evidenceRegistry;
  invoke(adminIdentity, ["upgrade", "--new_wasm_hash", replacementHash]);
  oldEvidenceReadable = invoke(adminIdentity, ["get_evidence", "--id", oldEvidenceId]).includes(manifest.attestor);
  if (!oldEvidenceReadable) throw new Error("old evidence was not readable after upgrade");
  checks.oldEvidenceRead = true;
  newEvidenceId = invoke(attestorIdentity, ["publish_evidence", "--input", evidenceInput("after")]).replaceAll('"', "");
  newEvidenceWritable = invoke(adminIdentity, ["get_evidence", "--id", newEvidenceId]).includes(manifest.attestor);
  if (!newEvidenceWritable) throw new Error("new evidence was not readable after upgrade");
  checks.newEvidenceWrite = true;
} catch (error) {
  failure = error instanceof Error ? error.message : String(error);
}
const postUpgradeEvents = events();
await writeFile(join(outputDirectory, "post-upgrade-events.json"), `${JSON.stringify(postUpgradeEvents, null, 2)}\n`, "utf8");
checks.eventReindex = postUpgradeEvents.length > preUpgradeEvents.length;
if (!checks.eventReindex && !failure) failure = "post-upgrade event re-index did not advance";
const decision = recoveryDecision({ integrityValid, oldEvidenceReadable, newEvidenceWritable });
const report = {
  schemaVersion: 1,
  manifest: basename(manifestPath),
  environment: manifest.environment,
  sourceRevision: manifest.sourceRevision,
  replacementWasmSha256: manifest.wasmSha256.evidenceRegistry,
  oldEvidenceId,
  newEvidenceId,
  checks,
  decision,
  failure: failure ?? null,
  completedAt: new Date().toISOString(),
};
await writeFile(join(outputDirectory, "report.json"), `${JSON.stringify(report, null, 2)}\n`, "utf8");
if (failure || decision !== "accept") throw new Error(`upgrade rehearsal requires ${decision}: ${failure ?? "checks incomplete"}`);
process.stdout.write(`${join(outputDirectory, "report.json")}\n`);
