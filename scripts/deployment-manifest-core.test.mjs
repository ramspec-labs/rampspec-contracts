import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  addGlobalArgs, sealDeploymentManifest, validateDeploymentManifest, verifyKnownWasm,
} from "./deployment-manifest-core.mjs";

const specs = JSON.parse(await readFile("artifacts/specs/manifest.json", "utf8"));
const hashes = {
  evidenceRegistry: specs.contracts["evidence-registry"].wasmSha256,
  policyAccountFixture: specs.contracts["policy-account-fixture"].wasmSha256,
  webAuthFixture: specs.contracts["web-auth-fixture"].wasmSha256,
};
const names = ["evidenceRegistry", "policyAccountFixture", "webAuthFixture"];

test("places global options before implicit contract arguments", () => {
  assert.deepEqual(
    addGlobalArgs(["contract", "invoke", "--id", "C123", "--", "schema_version"], ["--network", "testnet"]),
    ["contract", "invoke", "--id", "C123", "--network", "testnet", "--", "schema_version"],
  );
});

function validManifest() {
  return sealDeploymentManifest({
    schemaVersion: 1,
    environment: "testnet",
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
    sourceRelease: "v0.1.0",
    sourceRevision: "a".repeat(40),
    startLedger: 100,
    latestLedger: 110,
    transactions: ["1".repeat(64)],
    contracts: Object.fromEntries(names.map((name, index) => [name, `C${String.fromCharCode(65 + index).repeat(55)}`])),
    wasmSha256: hashes,
    administrators: Object.fromEntries(names.map((name, index) => [name, `G${String.fromCharCode(65 + index).repeat(55)}`])),
    attestor: `G${"D".repeat(55)}`,
    fixtureTestOnly: true,
    verification: { codeHashes: true, reads: true, writes: true, events: true },
    verifiedAt: "2026-09-13T00:00:00.000Z",
  });
}

test("accepts a sealed manifest with released artifacts", () => {
  const manifest = validManifest();
  assert.equal(validateDeploymentManifest(manifest, { now: Date.parse("2026-09-13T00:01:00Z") }), manifest);
  assert.doesNotThrow(() => verifyKnownWasm(manifest, specs));
});

test("rejects wrong networks and altered manifests", () => {
  const wrongNetwork = sealDeploymentManifest({ ...validManifest(), networkPassphrase: "wrong" });
  assert.throws(() => validateDeploymentManifest(wrongNetwork), /passphrase mismatch/);
  const altered = validManifest();
  altered.latestLedger += 1;
  assert.throws(() => validateDeploymentManifest(altered), /integrity check failed/);
});

test("rejects unknown artifacts and stale schema claims", () => {
  const unknown = validManifest();
  unknown.wasmSha256.evidenceRegistry = "f".repeat(64);
  const resealed = sealDeploymentManifest(unknown);
  assert.throws(() => verifyKnownWasm(resealed, specs), /unknown WASM hash/);
  const stale = sealDeploymentManifest({ ...validManifest(), schemaVersion: 2 });
  assert.throws(() => validateDeploymentManifest(stale), /unsupported.*schema/);
});

test("rejects incomplete verification and malformed transaction evidence", () => {
  const incomplete = validManifest();
  incomplete.verification.events = false;
  assert.throws(() => validateDeploymentManifest(sealDeploymentManifest(incomplete)), /incomplete verification/);
  const malformed = validManifest();
  malformed.transactions = ["not-a-hash"];
  assert.throws(() => validateDeploymentManifest(sealDeploymentManifest(malformed)), /transaction hashes/);
});

test("live verifier fails closed when RPC is unreachable", async () => {
  const directory = await mkdtemp(join(tmpdir(), "rampspec-unreachable-"));
  const path = join(directory, "manifest.json");
  const manifest = sealDeploymentManifest({ ...validManifest(), rpcUrl: "http://127.0.0.1:1" });
  await writeFile(path, JSON.stringify(manifest), "utf8");
  try {
    const result = spawnSync(process.execPath, ["scripts/verify-deployment-manifest.mjs", path, "--live"], {
      cwd: process.cwd(), encoding: "utf8", timeout: 10_000,
    });
    assert.notEqual(result.status, 0);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("pubnet format prohibits fixture deployment", () => {
  const manifest = validManifest();
  manifest.environment = "pubnet";
  manifest.networkPassphrase = "Public Global Stellar Network ; September 2015";
  for (const field of ["contracts", "wasmSha256", "administrators"]) {
    manifest[field].policyAccountFixture = null;
    manifest[field].webAuthFixture = null;
  }
  assert.doesNotThrow(() => validateDeploymentManifest(sealDeploymentManifest(manifest)));
  manifest.contracts.webAuthFixture = `C${"E".repeat(55)}`;
  assert.throws(() => validateDeploymentManifest(sealDeploymentManifest(manifest)), /must not include test fixture/);
});
