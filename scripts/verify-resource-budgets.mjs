import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const artifactDirectory = resolve(process.argv[2] ?? "artifacts/wasm");
const baseline = JSON.parse(
  await readFile("tests/resource-budgets/baselines.json", "utf8"),
);
if (baseline.schemaVersion !== 1 || baseline.sdk !== "27.0.0") {
  throw new Error("resource budget schema or SDK pin is invalid");
}

const ttlClasses = new Set(["instance", "none", "persistent", "temporary"]);
for (const [wasm, contract] of Object.entries(baseline.contracts)) {
  const spec = execFileSync(
    "stellar",
    ["contract", "info", "interface", "--wasm", join(artifactDirectory, wasm)],
    { encoding: "utf8" },
  );
  const exported = new Set(
    [...spec.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)].map(
      (match) => match[1],
    ),
  );
  const budgeted = new Set(Object.keys(contract));
  if (exported.size !== budgeted.size || [...exported].some((name) => !budgeted.has(name))) {
    throw new Error(`${wasm} exported methods do not match budget baselines`);
  }
  for (const [name, limits] of Object.entries(contract)) {
    for (const field of ["cpuMax", "memoryMax", "storageWritesMax", "eventsMax"]) {
      if (!Number.isInteger(limits[field]) || limits[field] < 0) {
        throw new Error(`${wasm}:${name} has invalid ${field}`);
      }
    }
    if (limits.cpuMax === 0 || limits.memoryMax === 0 || !ttlClasses.has(limits.ttl)) {
      throw new Error(`${wasm}:${name} has incomplete limits`);
    }
  }
}

process.stdout.write("all exported methods have reviewed resource budgets\n");
