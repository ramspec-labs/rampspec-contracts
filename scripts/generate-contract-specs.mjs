import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { basename, join, relative, resolve } from "node:path";

const artifactDirectory = resolve(process.argv[2] ?? "artifacts/wasm");
const outputDirectory = resolve(process.argv[3] ?? "artifacts/specs");
const contracts = [
  ["evidence-registry", "evidence_registry.wasm"],
  ["policy-account-fixture", "policy_account_fixture.wasm"],
  ["web-auth-fixture", "web_auth_fixture.wasm"],
];

function sha256(content) {
  return createHash("sha256").update(content).digest("hex");
}

function asciiCompare(left, right) {
  return Buffer.from(left).compare(Buffer.from(right));
}

function entryKey(entry) {
  const kind = Object.keys(entry)[0];
  const value = entry[kind];
  return `${kind}:${value?.name ?? value?.lib ?? ""}`;
}

function canonicalize(value) {
  if (Array.isArray(value)) {
    return value.map(canonicalize);
  }
  if (!value || typeof value !== "object") {
    return value;
  }
  const entries = Object.entries(value)
    .map(([key, child]) => [key === "type_" ? "type" : key, canonicalize(child)])
    .sort(([left], [right]) => asciiCompare(left, right));
  return Object.fromEntries(entries);
}

async function walk(directory) {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walk(path)));
    } else {
      files.push(path);
    }
  }
  return files;
}

async function sourceHash(contractName) {
  const paths = [
    resolve("Cargo.lock"),
    resolve("Cargo.toml"),
    resolve("rust-toolchain.toml"),
    resolve("contracts", contractName, "Cargo.toml"),
    ...(await walk(resolve("contracts", contractName, "src"))),
  ].sort(asciiCompare);
  const hash = createHash("sha256");
  for (const path of paths) {
    hash.update(relative(resolve("."), path).replaceAll("\\", "/"));
    hash.update("\0");
    hash.update(await readFile(path));
    hash.update("\0");
  }
  return hash.digest("hex");
}

await mkdir(outputDirectory, { recursive: true });
const manifest = {
  schemaVersion: 1,
  canonicalization: "recursive ASCII object keys, legacy type alias normalized to type, top-level entries sorted",
  generator: "stellar-cli 27.1.0",
  contracts: {},
};

for (const [contractName, wasmName] of contracts) {
  const wasmPath = join(artifactDirectory, wasmName);
  const rawJson = execFileSync(
    "stellar",
    ["contract", "info", "interface", "--quiet", "--wasm", wasmPath, "--output", "json"],
    { encoding: "utf8" },
  );
  const rawEntries = JSON.parse(rawJson).sort((left, right) =>
    asciiCompare(entryKey(left), entryKey(right)),
  );
  const canonicalJson = `${JSON.stringify(rawEntries.map(canonicalize), null, 2)}\n`;
  const xdrLines = rawEntries.map((entry) =>
    execFileSync(
      "stellar",
      ["xdr", "encode", "--quiet", "--type", "ScSpecEntry", "--output", "single-base64"],
      { encoding: "utf8", input: JSON.stringify(entry) },
    ).trim(),
  );
  const canonicalXdr = `${xdrLines.join("\n")}\n`;
  const stem = basename(wasmName, ".wasm");
  await writeFile(join(outputDirectory, `${stem}.json`), canonicalJson, "utf8");
  await writeFile(join(outputDirectory, `${stem}.xdr-base64`), canonicalXdr, "utf8");
  const wasm = await readFile(wasmPath);
  manifest.contracts[contractName] = {
    json: `${stem}.json`,
    jsonSha256: sha256(canonicalJson),
    sourceSha256: await sourceHash(contractName),
    wasm: wasmName,
    wasmSha256: sha256(wasm),
    xdr: `${stem}.xdr-base64`,
    xdrSha256: sha256(canonicalXdr),
  };
}

await writeFile(
  join(outputDirectory, "manifest.json"),
  `${JSON.stringify(canonicalize(manifest), null, 2)}\n`,
  "utf8",
);
process.stdout.write(`generated canonical specs in ${outputDirectory}\n`);
