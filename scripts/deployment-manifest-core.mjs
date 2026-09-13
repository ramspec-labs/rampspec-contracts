import { createHash } from "node:crypto";

const CONTRACT_NAMES = ["evidenceRegistry", "policyAccountFixture", "webAuthFixture"];
const HASH = /^[0-9a-f]{64}$/;
const REVISION = /^[0-9a-f]{40}$/;
const CONTRACT = /^C[A-Z2-7]{55}$/;
const ACCOUNT = /^G[A-Z2-7]{55}$/;
const PASSPHRASES = {
  local: "Standalone Network ; February 2017",
  testnet: "Test SDF Network ; September 2015",
  pubnet: "Public Global Stellar Network ; September 2015",
};

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
  }
  return value;
}

export function addGlobalArgs(args, globalArgs) {
  const separator = args.indexOf("--");
  return separator === -1
    ? [...args, ...globalArgs]
    : [...args.slice(0, separator), ...globalArgs, ...args.slice(separator)];
}

export function manifestDigest(manifest) {
  const { integritySha256: _ignored, ...unsigned } = manifest;
  return createHash("sha256").update(JSON.stringify(canonical(unsigned))).digest("hex");
}

export function sealDeploymentManifest(manifest) {
  const sealed = { ...manifest };
  sealed.integritySha256 = manifestDigest(sealed);
  return sealed;
}

function exactKeys(value, keys, path) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${path} fields do not match schema`);
}

export function validateDeploymentManifest(manifest, options = {}) {
  exactKeys(manifest, [
    "schemaVersion", "environment", "rpcUrl", "networkPassphrase", "sourceRelease",
    "sourceRevision", "startLedger", "latestLedger", "transactions", "contracts",
    "wasmSha256", "administrators", "attestor", "fixtureTestOnly", "verification",
    "verifiedAt", "integritySha256",
  ], "manifest");
  if (manifest.schemaVersion !== 1) throw new Error("unsupported deployment manifest schema");
  if (!(manifest.environment in PASSPHRASES)) throw new Error("unsupported deployment environment");
  if (options.expectedEnvironment && manifest.environment !== options.expectedEnvironment) {
    throw new Error(`expected ${options.expectedEnvironment}, received ${manifest.environment}`);
  }
  if (manifest.networkPassphrase !== PASSPHRASES[manifest.environment]) throw new Error("network passphrase mismatch");
  const url = new URL(manifest.rpcUrl);
  if (!['http:', 'https:'].includes(url.protocol)) throw new Error("RPC URL must use HTTP or HTTPS");
  if (typeof manifest.sourceRelease !== "string" || !manifest.sourceRelease) throw new Error("source release is required");
  if (!REVISION.test(manifest.sourceRevision)) throw new Error("source revision is invalid");
  for (const field of ["startLedger", "latestLedger"]) {
    if (!Number.isSafeInteger(manifest[field]) || manifest[field] < 1) throw new Error(`${field} is invalid`);
  }
  if (manifest.latestLedger < manifest.startLedger) throw new Error("ledger range is reversed");
  if (!Array.isArray(manifest.transactions) || manifest.transactions.length < 1) throw new Error("transaction evidence is required");
  if (new Set(manifest.transactions).size !== manifest.transactions.length || manifest.transactions.some((hash) => !HASH.test(hash))) {
    throw new Error("transaction hashes are invalid or duplicated");
  }
  exactKeys(manifest.contracts, CONTRACT_NAMES, "contracts");
  exactKeys(manifest.wasmSha256, CONTRACT_NAMES, "wasmSha256");
  exactKeys(manifest.administrators, CONTRACT_NAMES, "administrators");
  for (const name of CONTRACT_NAMES) {
    if (manifest.environment === "pubnet" && name !== "evidenceRegistry") {
      if (manifest.contracts[name] !== null || manifest.wasmSha256[name] !== null || manifest.administrators[name] !== null) {
        throw new Error("pubnet manifests must not include test fixture deployments");
      }
      continue;
    }
    if (!CONTRACT.test(manifest.contracts[name])) throw new Error(`${name} contract ID is invalid`);
    if (!HASH.test(manifest.wasmSha256[name])) throw new Error(`${name} WASM hash is invalid`);
    if (!ACCOUNT.test(manifest.administrators[name])) throw new Error(`${name} administrator is invalid`);
  }
  const contractIds = Object.values(manifest.contracts).filter(Boolean);
  const administrators = Object.values(manifest.administrators).filter(Boolean);
  if (new Set(contractIds).size !== contractIds.length) throw new Error("contract IDs must be distinct");
  if (new Set(administrators).size !== administrators.length) throw new Error("administrators must be distinct");
  if (!ACCOUNT.test(manifest.attestor)) throw new Error("attestor is invalid");
  if (manifest.fixtureTestOnly !== true) throw new Error("fixture contracts must be labeled test-only");
  exactKeys(manifest.verification, ["codeHashes", "reads", "writes", "events"], "verification");
  if (Object.values(manifest.verification).some((value) => value !== true)) throw new Error("manifest contains an incomplete verification result");
  const verifiedAt = Date.parse(manifest.verifiedAt);
  if (!Number.isFinite(verifiedAt)) throw new Error("verification timestamp is invalid");
  if (verifiedAt > (options.now ?? Date.now()) + 5 * 60_000) throw new Error("verification timestamp is in the future");
  if (!HASH.test(manifest.integritySha256) || manifest.integritySha256 !== manifestDigest(manifest)) {
    throw new Error("deployment manifest integrity check failed");
  }
  return manifest;
}

export function expectedWasmHashes(specManifest) {
  const contracts = specManifest?.contracts;
  return {
    evidenceRegistry: contracts?.["evidence-registry"]?.wasmSha256,
    policyAccountFixture: contracts?.["policy-account-fixture"]?.wasmSha256,
    webAuthFixture: contracts?.["web-auth-fixture"]?.wasmSha256,
  };
}

export function verifyKnownWasm(manifest, specManifest) {
  const expected = expectedWasmHashes(specManifest);
  for (const name of CONTRACT_NAMES) {
    if (manifest.wasmSha256[name] === null) continue;
    if (!HASH.test(expected[name] ?? "") || manifest.wasmSha256[name] !== expected[name]) {
      throw new Error(`${name} uses an unknown WASM hash`);
    }
  }
}

export { CONTRACT_NAMES, PASSPHRASES };
