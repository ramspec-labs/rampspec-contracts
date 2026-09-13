import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { validateFixtureManifest } from "./fixture-manifest-core.mjs";

const valid = JSON.parse(await readFile("fixtures/manifest.json", "utf8"));

test("accepts the released test-only fixture inventory", () => {
  assert.equal(validateFixtureManifest(structuredClone(valid)).schemaVersion, 1);
});

test("rejects missing test-only metadata", () => {
  const manifest = structuredClone(valid);
  delete manifest.fixtures[0].testOnly;
  assert.throws(() => validateFixtureManifest(manifest), /explicitly test-only/);
});

test("rejects public-network fixture deployment", () => {
  const manifest = structuredClone(valid);
  manifest.fixtures[0].allowedNetworks.push("pubnet");
  assert.throws(() => validateFixtureManifest(manifest), /forbidden network pubnet/);
});

test("rejects shared fixture administration", () => {
  const manifest = structuredClone(valid);
  manifest.fixtures[1].adminRole = manifest.fixtures[0].adminRole;
  assert.throws(() => validateFixtureManifest(manifest), /admin roles must be distinct/);
});
