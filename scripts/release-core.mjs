import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { join, relative, resolve } from "node:path";

export const sha256 = (content) => createHash("sha256").update(content).digest("hex");

export async function filesUnder(root) {
  const files = [];
  async function visit(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    entries.sort((a, b) => a.name < b.name ? -1 : a.name > b.name ? 1 : 0);
    for (const entry of entries) {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) await visit(path);
      else if (entry.isFile()) files.push(path);
    }
  }
  await visit(resolve(root));
  return files;
}

export async function checksumRecords(root, excluded = new Set()) {
  const base = resolve(root);
  const records = [];
  for (const path of await filesUnder(base)) {
    const name = relative(base, path).replaceAll("\\", "/");
    if (!excluded.has(name)) records.push({ path: name, sha256: sha256(await readFile(path)) });
  }
  return records;
}

export function renderChecksums(records) {
  return `${records.map((record) => `${record.sha256}  ${record.path}`).join("\n")}\n`;
}

export function parseChecksums(content) {
  return content.trim().split(/\r?\n/).filter(Boolean).map((line) => {
    const match = /^([0-9a-f]{64})  (.+)$/.exec(line);
    if (!match) throw new Error("checksum file contains an invalid line");
    return { path: match[2], sha256: match[1] };
  });
}

export async function verifyReleaseBundle(root) {
  const base = resolve(root);
  const manifest = JSON.parse(await readFile(join(base, "release-manifest.json"), "utf8"));
  const checksumContent = await readFile(join(base, "checksums.sha256"), "utf8");
  const fields = [
    "schemaVersion", "status", "version", "sourceRevision", "sourceDirty", "securityCleared",
    "testnetManifest", "signedTag", "checksumFileSha256", "createdAt",
  ];
  if (JSON.stringify(Object.keys(manifest).sort()) !== JSON.stringify(fields.sort())) throw new Error("release manifest fields do not match schema");
  if (manifest.schemaVersion !== 1 || !['candidate', 'released'].includes(manifest.status)) throw new Error("release manifest status is invalid");
  if (!/^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/.test(manifest.version)) throw new Error("release version is invalid");
  if (!/^[0-9a-f]{40}$/.test(manifest.sourceRevision)) throw new Error("release source revision is invalid");
  if (sha256(checksumContent) !== manifest.checksumFileSha256) throw new Error("checksum inventory digest mismatch");
  const recorded = parseChecksums(checksumContent);
  const actual = await checksumRecords(base, new Set(["checksums.sha256", "release-manifest.json"]));
  if (JSON.stringify(recorded) !== JSON.stringify(actual)) throw new Error("release payload inventory or checksums differ");
  const sbom = JSON.parse(await readFile(join(base, "sbom.spdx.json"), "utf8"));
  if (sbom.spdxVersion !== "SPDX-2.3" || !Array.isArray(sbom.packages) || sbom.packages.length === 0) throw new Error("release SBOM is invalid");
  if (!Number.isFinite(Date.parse(manifest.createdAt)) || typeof manifest.sourceDirty !== "boolean") throw new Error("release metadata is invalid");
  if (manifest.status === "released" && (manifest.sourceDirty || !manifest.securityCleared || !manifest.testnetManifest || !manifest.signedTag)) {
    throw new Error("released bundle lacks mandatory gates");
  }
  return manifest;
}
