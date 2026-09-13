import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  addGlobalArgs, CONTRACT_NAMES, validateDeploymentManifest, verifyKnownWasm,
} from "./deployment-manifest-core.mjs";

const manifestPath = resolve(process.argv[2] ?? "");
if (!process.argv[2]) throw new Error("usage: node scripts/verify-deployment-manifest.mjs <manifest> [--live] [--environment name]");
const live = process.argv.includes("--live");
const environmentIndex = process.argv.indexOf("--environment");
const expectedEnvironment = environmentIndex === -1 ? undefined : process.argv[environmentIndex + 1];
if (environmentIndex !== -1 && !expectedEnvironment) throw new Error("--environment requires a value");

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const specs = JSON.parse(await readFile(resolve("artifacts/specs/manifest.json"), "utf8"));
validateDeploymentManifest(manifest, { expectedEnvironment });
verifyKnownWasm(manifest, specs);

function stellar(args, capture = true) {
  const network = ["--rpc-url", manifest.rpcUrl, "--network-passphrase", manifest.networkPassphrase];
  const command = addGlobalArgs(args, network);
  return execFileSync("stellar", command, {
    encoding: "utf8", stdio: capture ? ["ignore", "pipe", "inherit"] : "inherit",
  })?.trim();
}

if (live) {
  const info = JSON.parse(stellar(["network", "info", "--output", "json"]));
  if (!Number.isSafeInteger(Number(info.latestLedger)) || Number(info.latestLedger) < manifest.latestLedger) {
    throw new Error("RPC is stale or belongs to the wrong network");
  }
  const directory = await mkdtemp(join(tmpdir(), "rampspec-manifest-"));
  try {
    for (const name of CONTRACT_NAMES) {
      if (manifest.contracts[name] === null) continue;
      const fetched = join(directory, `${name}.wasm`);
      stellar(["contract", "fetch", "--id", manifest.contracts[name], "--out-file", fetched], false);
      const hash = createHash("sha256").update(await readFile(fetched)).digest("hex");
      if (hash !== manifest.wasmSha256[name]) throw new Error(`${name} RPC code hash does not match`);
      const source = manifest.administrators[name];
      const view = (fn) => stellar([
        "contract", "invoke", "--send", "no", "--id", manifest.contracts[name],
        "--source-account", source, "--", fn,
      ]);
      if (view("schema_version") !== "1") throw new Error(`${name} schema is stale`);
      if (name !== "evidenceRegistry" && view("is_test_only") !== "true") {
        throw new Error(`${name} lost its test-only label`);
      }
    }
    const attestor = stellar([
      "contract", "invoke", "--send", "no", "--id", manifest.contracts.evidenceRegistry,
      "--source-account", manifest.administrators.evidenceRegistry, "--", "get_attestor",
      "--attestor", manifest.attestor,
    ]);
    if (!attestor.includes(manifest.attestor) || !attestor.includes("true")) throw new Error("attestor policy read-back failed");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

process.stdout.write(`verified ${manifest.environment} deployment manifest${live ? " against RPC" : " offline"}\n`);
