import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { validateFixtureManifest } from "./fixture-manifest-core.mjs";

const artifactDirectory = resolve(process.argv[2] ?? "artifacts/wasm");
const manifest = validateFixtureManifest(
  JSON.parse(await readFile("fixtures/manifest.json", "utf8")),
);
const forbidden = new Set([
  "allowance",
  "approve",
  "balance",
  "burn",
  "deposit",
  "mint",
  "publish_evidence",
  "transfer",
  "upgrade",
  "withdraw",
]);
const required = new Map([
  ["web-auth-fixture", "web_auth_verify"],
  ["policy-account-fixture", "__check_auth"],
]);

for (const fixture of manifest.fixtures) {
  const artifact = join(artifactDirectory, fixture.wasm);
  const spec = execFileSync(
    "stellar",
    ["contract", "info", "interface", "--wasm", artifact],
    { encoding: "utf8" },
  );
  const functions = new Set(
    [...spec.matchAll(/\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g)].map(
      (match) => match[1],
    ),
  );
  if (!functions.has("is_test_only") || !functions.has(required.get(fixture.name))) {
    throw new Error(`${fixture.name} is missing required fixture entrypoints`);
  }
  for (const name of forbidden) {
    if (functions.has(name)) {
      throw new Error(`${fixture.name} exposes forbidden function ${name}`);
    }
  }
}

process.stdout.write("fixture WASM interfaces are isolated\n");
