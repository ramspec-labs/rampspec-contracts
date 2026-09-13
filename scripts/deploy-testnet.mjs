import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { addGlobalArgs, sealDeploymentManifest } from "./deployment-manifest-core.mjs";

const PASSPHRASE = "Test SDF Network ; September 2015";
const RPC_URL = "https://soroban-testnet.stellar.org";
const HORIZON_URL = "https://horizon-testnet.stellar.org";
const runtimeDirectory = resolve(process.argv[2] ?? "deployments/testnet/runtime");
const artifactDirectory = resolve("artifacts/wasm");
const requiredConfirmation = "deploy-reviewed-artifacts-to-testnet";

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function identity(name) {
  const value = required(name);
  if (value.startsWith("S") || value.includes(" ") || value.includes("\n")) {
    throw new Error(`${name} must name a preconfigured Stellar CLI identity, not contain secret material`);
  }
  return value;
}

const identities = {
  deployer: identity("RAMPSPEC_TESTNET_DEPLOYER_IDENTITY"),
  registryAdmin: identity("RAMPSPEC_TESTNET_REGISTRY_ADMIN_IDENTITY"),
  webAuthAdmin: identity("RAMPSPEC_TESTNET_WEB_AUTH_ADMIN_IDENTITY"),
  policyAdmin: identity("RAMPSPEC_TESTNET_POLICY_ADMIN_IDENTITY"),
  attestor: identity("RAMPSPEC_TESTNET_ATTESTOR_IDENTITY"),
};
if (required("RAMPSPEC_CONFIRM_TESTNET") !== requiredConfirmation) {
  throw new Error(`RAMPSPEC_CONFIRM_TESTNET must equal ${requiredConfirmation}`);
}
const sourceRelease = required("RAMPSPEC_SOURCE_RELEASE");

const networkArgs = ["--rpc-url", RPC_URL, "--network-passphrase", PASSPHRASE];
function stellar(args, options = {}) {
  const command = addGlobalArgs(args, networkArgs);
  return execFileSync("stellar", command, {
    encoding: "utf8",
    stdio: options.capture ? ["ignore", "pipe", "inherit"] : "inherit",
  })?.trim();
}

function stellarKey(args) {
  return execFileSync("stellar", args, { encoding: "utf8" }).trim();
}

function sha256(content) {
  return createHash("sha256").update(content).digest("hex");
}

async function fundedAddress(alias) {
  const address = stellarKey(["keys", "public-key", alias]);
  if (!/^G[A-Z2-7]{55}$/.test(address)) throw new Error(`${alias} did not resolve to a G address`);
  const response = await fetch(`${HORIZON_URL}/accounts/${address}`);
  if (!response.ok) throw new Error(`${alias} is not funded on testnet (${response.status})`);
  const account = await response.json();
  const native = account.balances.find((balance) => balance.asset_type === "native");
  if (!native || Number(native.balance) < 2) {
    throw new Error(`${alias} must retain at least 2 testnet XLM before deployment`);
  }
  return address;
}

async function verifyWasm(contractId, expectedPath, name) {
  const fetched = join(runtimeDirectory, `${name}.fetched.wasm`);
  stellar(["contract", "fetch", "--id", contractId, "--out-file", fetched]);
  const expected = sha256(await readFile(expectedPath));
  const actual = sha256(await readFile(fetched));
  if (actual !== expected) throw new Error(`${name} deployed WASM hash mismatch`);
  return actual;
}

await mkdir(runtimeDirectory, { recursive: true });
const networkInfoBefore = JSON.parse(stellar(["network", "info", "--output", "json"], { capture: true }));
const startLedger = Number(networkInfoBefore.latestLedger);
if (!Number.isSafeInteger(startLedger) || startLedger < 1) throw new Error("testnet RPC returned no valid ledger");

const addressEntries = await Promise.all(
  Object.entries(identities).map(async ([role, alias]) => [role, await fundedAddress(alias)]),
);
const addresses = Object.fromEntries(addressEntries);
if (new Set([addresses.registryAdmin, addresses.webAuthAdmin, addresses.policyAdmin]).size !== 3) {
  throw new Error("registry and fixture administrators must be distinct accounts");
}

const currentRevision = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const releaseRevision = execFileSync("git", ["rev-parse", `${sourceRelease}^{commit}`], { encoding: "utf8" }).trim();
if (releaseRevision !== currentRevision) throw new Error("RAMPSPEC_SOURCE_RELEASE must resolve to the checked-out commit");
if (execFileSync("git", ["status", "--porcelain"], { encoding: "utf8" }).trim()) {
  throw new Error("testnet deployments require a clean source checkout");
}

if (process.platform === "win32") {
  execFileSync("powershell", ["-NoProfile", "-File", "scripts/build-reproducible.ps1"], { stdio: "inherit" });
} else {
  execFileSync("bash", ["scripts/build-reproducible.sh"], { stdio: "inherit" });
}
const wasm = {
  evidenceRegistry: join(artifactDirectory, "evidence_registry.wasm"),
  policyAccountFixture: join(artifactDirectory, "policy_account_fixture.wasm"),
  webAuthFixture: join(artifactDirectory, "web_auth_fixture.wasm"),
};
const deploy = (path, source) => stellar(
  ["contract", "deploy", "--wasm", path, "--source-account", source],
  { capture: true },
);
const contracts = {
  evidenceRegistry: deploy(wasm.evidenceRegistry, identities.deployer),
  policyAccountFixture: deploy(wasm.policyAccountFixture, identities.deployer),
  webAuthFixture: deploy(wasm.webAuthFixture, identities.deployer),
};
for (const [name, id] of Object.entries(contracts)) {
  if (!/^C[A-Z2-7]{55}$/.test(id)) throw new Error(`${name} deployment returned an invalid contract ID`);
}

const invoke = (id, source, args, capture = false) => stellar(
  ["contract", "invoke", "--id", id, "--source-account", source, "--", ...args],
  { capture },
);
invoke(contracts.evidenceRegistry, identities.registryAdmin, [
  "initialize", "--admin", addresses.registryAdmin, "--schema_version", "1",
]);
invoke(contracts.evidenceRegistry, identities.registryAdmin, [
  "register_attestor", "--attestor", addresses.attestor, "--metadata_hash", "09".repeat(32),
]);
invoke(contracts.evidenceRegistry, identities.registryAdmin, ["set_paused", "--paused", "true"]);
invoke(contracts.evidenceRegistry, identities.registryAdmin, ["set_paused", "--paused", "false"]);
if (invoke(contracts.evidenceRegistry, identities.registryAdmin, ["schema_version"], true) !== "1") {
  throw new Error("registry schema read-back failed");
}
const attestorRead = invoke(
  contracts.evidenceRegistry,
  identities.registryAdmin,
  ["get_attestor", "--attestor", addresses.attestor],
  true,
);
if (!attestorRead.includes(addresses.attestor) || !attestorRead.includes("true")) {
  throw new Error("registered attestor read-back failed");
}

invoke(contracts.webAuthFixture, identities.webAuthAdmin, [
  "initialize", "--admin", addresses.webAuthAdmin, "--mode", "Accept",
  "--valid_from_ledger", "0", "--expires_at_ledger", String(2 ** 32 - 1),
]);
const signer = JSON.stringify([{ public_key: "11".repeat(32), weight: 1 }]);
invoke(contracts.policyAccountFixture, identities.policyAdmin, [
  "initialize", "--admin", addresses.policyAdmin, "--signers", signer, "--threshold", "1",
]);
for (const [id, source] of [
  [contracts.webAuthFixture, identities.webAuthAdmin],
  [contracts.policyAccountFixture, identities.policyAdmin],
]) {
  if (invoke(id, source, ["is_test_only"], true) !== "true") {
    throw new Error(`fixture ${id} did not report its test-only label`);
  }
  invoke(id, source, ["config"], true);
}

const wasmSha256 = Object.fromEntries(await Promise.all(
  Object.entries(contracts).map(async ([name, id]) => [
    name,
    await verifyWasm(id, wasm[name], name),
  ]),
));
const eventsRaw = stellar([
  "events", "--start-ledger", String(startLedger), "--count", "100", "--output", "json",
  "--id", contracts.evidenceRegistry, contracts.webAuthFixture, contracts.policyAccountFixture,
], { capture: true });
const events = JSON.parse(eventsRaw);
if (!Array.isArray(events) || events.length < 6) throw new Error("deployment event journey is incomplete");
const transactions = [...new Set(events.map((event) => event.txHash ?? event.tx_hash).filter(Boolean))];
if (transactions.length < 6) throw new Error("deployment transaction hashes were not captured from RPC events");
const networkInfoAfter = JSON.parse(stellar(["network", "info", "--output", "json"], { capture: true }));

const manifest = sealDeploymentManifest({
  schemaVersion: 1,
  environment: "testnet",
  rpcUrl: RPC_URL,
  networkPassphrase: PASSPHRASE,
  sourceRelease,
  sourceRevision: currentRevision,
  startLedger,
  latestLedger: Number(networkInfoAfter.latestLedger),
  transactions,
  contracts,
  wasmSha256,
  administrators: {
    evidenceRegistry: addresses.registryAdmin,
    policyAccountFixture: addresses.policyAdmin,
    webAuthFixture: addresses.webAuthAdmin,
  },
  attestor: addresses.attestor,
  fixtureTestOnly: true,
  verification: { codeHashes: true, reads: true, writes: true, events: true },
  verifiedAt: new Date().toISOString(),
});
await writeFile(join(runtimeDirectory, "journey-events.json"), `${JSON.stringify(events, null, 2)}\n`, "utf8");
await writeFile(join(runtimeDirectory, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
process.stdout.write(`${join(runtimeDirectory, "manifest.json")}\n`);
