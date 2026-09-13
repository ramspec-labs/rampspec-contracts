import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { addGlobalArgs, sealDeploymentManifest } from "./deployment-manifest-core.mjs";

const runtimeDirectory = resolve(process.argv[2] ?? "deployments/local/runtime");
const containerName = process.env.RAMPSPEC_LOCAL_CONTAINER ?? "rampspec-local";
const configDirectory = join(runtimeDirectory, "stellar-config");
const artifactDirectory = resolve("artifacts/wasm");
const network = "local";
const passphrase = "Standalone Network ; February 2017";
const rpcUrl = "http://localhost:8000/soroban/rpc";

function stellar(args, options = {}) {
  const command = addGlobalArgs(args, ["--config-dir", configDirectory]);
  return execFileSync("stellar", command, {
    encoding: "utf8",
    stdio: options.capture ? ["ignore", "pipe", "inherit"] : "inherit",
  })?.trim();
}

function sha256(content) {
  return createHash("sha256").update(content).digest("hex");
}

async function verifyWasm(contractId, expectedPath, name) {
  const fetched = join(runtimeDirectory, `${name}.fetched.wasm`);
  stellar(["contract", "fetch", "--id", contractId, "--network", network, "--out-file", fetched]);
  const expected = sha256(await readFile(expectedPath));
  const actual = sha256(await readFile(fetched));
  if (actual !== expected) {
    throw new Error(`${name} deployed WASM hash mismatch`);
  }
  return actual;
}

await mkdir(runtimeDirectory, { recursive: true });
if (process.env.RAMPSPEC_USE_EXISTING_LOCAL !== "1") {
  stellar(["container", "start", "local", "--name", containerName]);
}
const networkInfoBefore = JSON.parse(
  stellar(["network", "info", "--network", network, "--output", "json"], { capture: true }),
);
const startLedger = Number(networkInfoBefore.latestLedger);
if (!Number.isSafeInteger(startLedger) || startLedger < 1) throw new Error("local RPC returned no valid ledger");

for (const identity of ["deployer", "registry-admin", "web-auth-admin", "policy-admin", "attestor"]) {
  stellar(["keys", "generate", identity, "--network", network, "--fund", "--overwrite"]);
}
const addresses = Object.fromEntries(
  ["deployer", "registry-admin", "web-auth-admin", "policy-admin", "attestor"].map((identity) => [
    identity,
    stellar(["keys", "public-key", identity], { capture: true }),
  ]),
);
if (new Set([addresses["registry-admin"], addresses["web-auth-admin"], addresses["policy-admin"]]).size !== 3) {
  throw new Error("fixture and registry administrators must be distinct");
}

if (process.platform === "win32") {
  execFileSync("powershell", ["-NoProfile", "-File", "scripts/build-reproducible.ps1"], {
    stdio: "inherit",
  });
} else {
  execFileSync("bash", ["scripts/build-reproducible.sh"], { stdio: "inherit" });
}
const wasm = {
  registry: join(artifactDirectory, "evidence_registry.wasm"),
  policy: join(artifactDirectory, "policy_account_fixture.wasm"),
  webAuth: join(artifactDirectory, "web_auth_fixture.wasm"),
};
const deploy = (path, alias) =>
  stellar(
    [
      "contract", "deploy", "--wasm", path, "--source-account", "deployer", "--network", network,
      "--alias", alias,
    ],
    { capture: true },
  );
const contracts = {
  evidenceRegistry: deploy(wasm.registry, "rampspec-evidence-registry"),
  policyAccountFixture: deploy(wasm.policy, "rampspec-policy-account-fixture"),
  webAuthFixture: deploy(wasm.webAuth, "rampspec-web-auth-fixture"),
};

const invoke = (id, source, args, capture = false) =>
  stellar(
    ["contract", "invoke", "--id", id, "--source-account", source, "--network", network, "--", ...args],
    { capture },
  );
invoke(contracts.evidenceRegistry, "registry-admin", [
  "initialize", "--admin", addresses["registry-admin"], "--schema_version", "1",
]);
invoke(contracts.evidenceRegistry, "registry-admin", [
  "register_attestor", "--attestor", addresses.attestor, "--metadata_hash", "09".repeat(32),
]);
invoke(contracts.evidenceRegistry, "registry-admin", ["set_paused", "--paused", "true"]);
invoke(contracts.evidenceRegistry, "registry-admin", ["set_paused", "--paused", "false"]);
const schemaVersion = invoke(
  contracts.evidenceRegistry,
  "registry-admin",
  ["schema_version"],
  true,
);
if (schemaVersion !== "1") throw new Error(`unexpected schema version ${schemaVersion}`);

invoke(contracts.webAuthFixture, "web-auth-admin", [
  "initialize", "--admin", addresses["web-auth-admin"], "--mode", "Accept",
  "--valid_from_ledger", "0", "--expires_at_ledger", String(2 ** 32 - 1),
]);
invoke(contracts.webAuthFixture, "web-auth-admin", ["config"], true);
const signer = JSON.stringify([{ public_key: "11".repeat(32), weight: 1 }]);
invoke(contracts.policyAccountFixture, "policy-admin", [
  "initialize", "--admin", addresses["policy-admin"], "--signers", signer, "--threshold", "1",
]);
invoke(contracts.policyAccountFixture, "policy-admin", ["config"], true);

const hashes = {
  evidenceRegistry: await verifyWasm(contracts.evidenceRegistry, wasm.registry, "evidence-registry"),
  policyAccountFixture: await verifyWasm(contracts.policyAccountFixture, wasm.policy, "policy-account-fixture"),
  webAuthFixture: await verifyWasm(contracts.webAuthFixture, wasm.webAuth, "web-auth-fixture"),
};
const networkInfo = JSON.parse(
  stellar(["network", "info", "--network", network, "--output", "json"], { capture: true }),
);
const events = JSON.parse(stellar([
  "events", "--start-ledger", String(startLedger), "--count", "100", "--output", "json",
  "--id", contracts.evidenceRegistry, contracts.webAuthFixture, contracts.policyAccountFixture,
  "--network", network,
], { capture: true }));
const transactions = [...new Set(events.map((event) => event.txHash ?? event.tx_hash).filter(Boolean))];
if (transactions.length < 6) throw new Error("local deployment transaction evidence is incomplete");
const sourceRevision = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const manifest = sealDeploymentManifest({
  schemaVersion: 1,
  environment: "local",
  rpcUrl,
  networkPassphrase: passphrase,
  sourceRelease: `local:${sourceRevision}`,
  sourceRevision,
  startLedger,
  latestLedger: Number(networkInfo.latestLedger),
  transactions,
  contracts,
  wasmSha256: hashes,
  administrators: {
    evidenceRegistry: addresses["registry-admin"],
    policyAccountFixture: addresses["policy-admin"],
    webAuthFixture: addresses["web-auth-admin"],
  },
  attestor: addresses.attestor,
  fixtureTestOnly: true,
  verification: { codeHashes: true, reads: true, writes: true, events: true },
  verifiedAt: new Date().toISOString(),
});
await writeFile(join(runtimeDirectory, "journey-events.json"), `${JSON.stringify(events, null, 2)}\n`, "utf8");
await writeFile(join(runtimeDirectory, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
process.stdout.write(`${join(runtimeDirectory, "manifest.json")}\n`);
