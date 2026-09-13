import { execFileSync, spawnSync } from "node:child_process";
import { cp, mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { validateFindingsRegister } from "./security-readiness-core.mjs";
import { checksumRecords, renderChecksums, sha256, verifyReleaseBundle } from "./release-core.mjs";

function option(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}
const version = option("--version");
if (!version || !/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error("--version must be a semantic version");
const output = resolve(option("--output") ?? `artifacts/releases/v${version}`);
const candidate = process.argv.includes("--allow-uncleared-candidate");
const allowDirty = candidate && process.argv.includes("--allow-dirty-candidate");
const signTag = process.argv.includes("--sign-tag");
const testnetManifest = option("--testnet-manifest");
try { await stat(output); throw new Error(`release output already exists: ${output}`); } catch (error) {
  if (error.code !== "ENOENT") throw error;
}
const dirty = Boolean(execFileSync("git", ["status", "--porcelain"], { encoding: "utf8" }).trim());
if (dirty && !allowDirty) throw new Error("release packaging requires a clean checkout");
const sourceRevision = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const review = JSON.parse(await readFile("security/review-status.json", "utf8"));
const findings = JSON.parse(await readFile("security/findings.json", "utf8"));
const security = validateFindingsRegister(findings, review);
if (!security.cleared && !candidate) throw new Error(`security gate is not cleared: ${security.reason}`);
if ((signTag || !candidate) && !testnetManifest) throw new Error("released bundles require --testnet-manifest");
if (testnetManifest) {
  execFileSync("node", ["scripts/verify-deployment-manifest.mjs", resolve(testnetManifest), "--live", "--environment", "testnet"], { stdio: "inherit" });
}
if (signTag && candidate) throw new Error("candidate bundles cannot create signed tags");
if (!candidate && !signTag) throw new Error("released bundles require --sign-tag");

await mkdir(output, { recursive: true });
const wasmDirectory = join(output, "wasm");
if (process.platform === "win32") {
  execFileSync("powershell", ["-NoProfile", "-File", "scripts/build-reproducible.ps1", "-OutputDirectory", wasmDirectory], { stdio: "inherit" });
  execFileSync("powershell", ["-NoProfile", "-File", "scripts/verify-generated.ps1"], { stdio: "inherit" });
} else {
  execFileSync("bash", ["scripts/build-reproducible.sh", wasmDirectory], { stdio: "inherit" });
  execFileSync("bash", ["scripts/verify-generated.sh"], { stdio: "inherit" });
}
const specManifest = JSON.parse(await readFile("artifacts/specs/manifest.json", "utf8"));
for (const [name, entry] of Object.entries(specManifest.contracts)) {
  const actualHash = sha256(await readFile(join(wasmDirectory, entry.wasm)));
  if (actualHash !== entry.wasmSha256) {
    throw new Error(`${name} build does not match the canonical release WASM hash; use the pinned Linux container`);
  }
}
async function copyTracked(prefix, destination) {
  const files = execFileSync("git", ["ls-files", prefix], { encoding: "utf8" }).trim().split(/\r?\n/).filter(Boolean);
  if (files.length === 0) throw new Error(`no tracked release files found under ${prefix}`);
  for (const file of files) {
    const target = join(destination, file.slice(prefix.length).replace(/^[/\\]/, ""));
    await mkdir(resolve(target, ".."), { recursive: true });
    await cp(file, target);
  }
}
await copyTracked("artifacts/specs", join(output, "specs"));
await mkdir(join(output, "clients"));
function npm(directory, args) {
  if (process.platform === "win32") {
    execFileSync(process.env.ComSpec ?? "cmd.exe", ["/d", "/s", "/c", "npm", ...args], { cwd: directory, stdio: "inherit" });
  } else {
    execFileSync("npm", args, { cwd: directory, stdio: "inherit" });
  }
}
function npmAudit(directory) {
  const command = process.platform === "win32" ? (process.env.ComSpec ?? "cmd.exe") : "npm";
  const args = process.platform === "win32"
    ? ["/d", "/s", "/c", "npm", "audit", "--json"]
    : ["audit", "--json"];
  const result = spawnSync(command, args, { cwd: directory, encoding: "utf8", maxBuffer: 10 * 1024 * 1024 });
  if (result.error) throw result.error;
  try { return JSON.parse(result.stdout); }
  catch { throw new Error(`npm audit did not return JSON for ${directory}`); }
}
const npmAudits = {};
for (const name of ["evidence-registry", "policy-account-fixture", "web-auth-fixture"]) {
  const directory = resolve("artifacts/bindings", name);
  npm(directory, ["ci"]);
  npmAudits[name] = npmAudit(directory);
  npm(directory, ["run", "build"]);
  npm(directory, ["pack", "--pack-destination", join(output, "clients")]);
}
await writeFile(join(output, "npm-audit.json"), `${JSON.stringify(npmAudits, null, 2)}\n`, "utf8");
if (!candidate) {
  for (const [name, audit] of Object.entries(npmAudits)) {
    const vulnerabilities = audit.metadata?.vulnerabilities ?? {};
    if (['moderate', 'high', 'critical'].some((severity) => Number(vulnerabilities[severity] ?? 0) > 0)) {
      throw new Error(`${name} has unresolved moderate-or-higher npm advisories`);
    }
  }
}
await mkdir(join(output, "evidence"));
for (const directory of ["deployments", "docs", "fixtures", "security", "tests/resource-budgets", "upgrade"]) {
  await copyTracked(directory, join(output, "evidence", directory.replaceAll("/", "-")));
}
await cp("release/migration-notes.md", join(output, "migration-notes.md"));
await cp("release/release-manifest.schema.json", join(output, "evidence", "release-manifest.schema.json"));
if (testnetManifest) await cp(resolve(testnetManifest), join(output, "testnet-manifest.json"));
execFileSync("git", ["archive", "--format=tar", `--output=${join(output, "source.tar")}`, "HEAD"]);

const cargo = JSON.parse(execFileSync("cargo", ["metadata", "--locked", "--format-version", "1"], {
  encoding: "utf8", maxBuffer: 50 * 1024 * 1024,
}));
const npmLocks = await Promise.all(["evidence-registry", "policy-account-fixture", "web-auth-fixture"].map(async (name) => ({
  name,
  lock: JSON.parse(await readFile(join("artifacts/bindings", name, "package-lock.json"), "utf8")),
})));
const packages = cargo.packages.map((pkg, index) => ({
  SPDXID: `SPDXRef-Cargo-${index}`, name: pkg.name, versionInfo: pkg.version,
  downloadLocation: pkg.source ?? "NOASSERTION", filesAnalyzed: false,
  licenseConcluded: "NOASSERTION", licenseDeclared: pkg.license ?? "NOASSERTION",
}));
for (const item of npmLocks) {
  for (const [path, pkg] of Object.entries(item.lock.packages ?? {})) {
    if (!pkg.version) continue;
    packages.push({
      SPDXID: `SPDXRef-Npm-${packages.length}`, name: pkg.name ?? `${item.name}:${path || "root"}`,
      versionInfo: pkg.version, downloadLocation: pkg.resolved ?? "NOASSERTION", filesAnalyzed: false,
      licenseConcluded: "NOASSERTION", licenseDeclared: pkg.license ?? "NOASSERTION",
    });
  }
}
const createdAt = new Date().toISOString();
await writeFile(join(output, "sbom.spdx.json"), `${JSON.stringify({
  spdxVersion: "SPDX-2.3", dataLicense: "CC0-1.0", SPDXID: "SPDXRef-DOCUMENT",
  name: `rampspec-contracts-${version}`, documentNamespace: `https://rampspec.org/spdx/${sourceRevision}/${version}`,
  creationInfo: { created: createdAt, creators: ["Tool: scripts/build-release.mjs"] }, packages,
}, null, 2)}\n`, "utf8");
const buildMetadata = JSON.parse(await readFile(join(wasmDirectory, "provenance.json"), "utf8"));
await writeFile(join(output, "provenance.json"), `${JSON.stringify({
  schemaVersion: 1, sourceRevision, sourceDirty: dirty, version, createdAt,
  toolchain: {
    rust: execFileSync("rustc", ["--version"], { encoding: "utf8" }).trim(),
    cargo: execFileSync("cargo", ["--version"], { encoding: "utf8" }).trim(),
    stellar: execFileSync("stellar", ["--version"], { encoding: "utf8" }).trim(),
    node: process.version,
    container: buildMetadata.container,
  },
  wasmSha256: Object.fromEntries(Object.entries(specManifest.contracts).map(([name, entry]) => [name, entry.wasmSha256])),
  generatedSpecs: specManifest,
}, null, 2)}\n`, "utf8");
const records = await checksumRecords(output, new Set(["checksums.sha256", "release-manifest.json"]));
const checksumContent = renderChecksums(records);
await writeFile(join(output, "checksums.sha256"), checksumContent, "utf8");
const releaseManifest = {
  schemaVersion: 1,
  status: "candidate",
  version,
  sourceRevision,
  sourceDirty: dirty,
  securityCleared: security.cleared,
  testnetManifest: testnetManifest ? "testnet-manifest.json" : null,
  signedTag: false,
  checksumFileSha256: sha256(checksumContent),
  createdAt,
};
await writeFile(join(output, "release-manifest.json"), `${JSON.stringify(releaseManifest, null, 2)}\n`, "utf8");
await verifyReleaseBundle(output);
if (signTag) {
  execFileSync("git", ["tag", "-s", `v${version}`, "-m", `RampSpec contracts v${version}`], { stdio: "inherit" });
  const signedTag = execFileSync("git", ["cat-file", "tag", `v${version}`]);
  await writeFile(join(output, "signed-tag.txt"), signedTag);
  const signedRecords = await checksumRecords(output, new Set(["checksums.sha256", "release-manifest.json"]));
  const signedChecksums = renderChecksums(signedRecords);
  await writeFile(join(output, "checksums.sha256"), signedChecksums, "utf8");
  releaseManifest.status = "released";
  releaseManifest.signedTag = true;
  releaseManifest.checksumFileSha256 = sha256(signedChecksums);
  await writeFile(join(output, "release-manifest.json"), `${JSON.stringify(releaseManifest, null, 2)}\n`, "utf8");
  await verifyReleaseBundle(output);
}
process.stdout.write(`${output}\n`);
