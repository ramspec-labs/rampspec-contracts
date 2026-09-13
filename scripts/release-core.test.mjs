import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { checksumRecords, renderChecksums, sha256, verifyReleaseBundle } from "./release-core.mjs";

async function bundle() {
  const root = await mkdtemp(join(tmpdir(), "rampspec-release-"));
  await mkdir(join(root, "wasm"));
  await writeFile(join(root, "wasm", "registry.wasm"), "wasm", "utf8");
  await writeFile(join(root, "sbom.spdx.json"), JSON.stringify({ spdxVersion: "SPDX-2.3", packages: [{ name: "x" }] }), "utf8");
  const checksums = renderChecksums(await checksumRecords(root));
  await writeFile(join(root, "checksums.sha256"), checksums, "utf8");
  await writeFile(join(root, "release-manifest.json"), JSON.stringify({
    schemaVersion: 1, status: "candidate", version: "0.1.0", sourceRevision: "a".repeat(40),
    sourceDirty: false, securityCleared: false, testnetManifest: null, signedTag: false,
    checksumFileSha256: sha256(checksums), createdAt: "2026-09-13T00:00:00Z",
  }), "utf8");
  return root;
}

test("verifies a complete candidate bundle", async () => {
  const root = await bundle();
  try { assert.equal((await verifyReleaseBundle(root)).status, "candidate"); }
  finally { await rm(root, { recursive: true, force: true }); }
});

test("rejects changed and unlisted payload files", async () => {
  const root = await bundle();
  try {
    await writeFile(join(root, "wasm", "registry.wasm"), "altered", "utf8");
    await assert.rejects(() => verifyReleaseBundle(root), /checksums differ/);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test("released status cannot bypass security and deployment gates", async () => {
  const root = await bundle();
  try {
    const manifestPath = join(root, "release-manifest.json");
    await writeFile(manifestPath, JSON.stringify({
      schemaVersion: 1, status: "released", version: "0.1.0", sourceRevision: "a".repeat(40),
      sourceDirty: false, securityCleared: false, testnetManifest: null, signedTag: false,
      checksumFileSha256: sha256(await (await import("node:fs/promises")).readFile(join(root, "checksums.sha256"))),
      createdAt: "2026-09-13T00:00:00Z",
    }), "utf8");
    await assert.rejects(() => verifyReleaseBundle(root), /mandatory gates/);
  } finally { await rm(root, { recursive: true, force: true }); }
});
