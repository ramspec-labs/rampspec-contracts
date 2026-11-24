import assert from "node:assert/strict";
import test from "node:test";
import {
  assertIdentityAlias,
  compareStorageSnapshots,
  evidenceRegistryStorageShape,
  recoveryDecision,
} from "./upgrade-rehearsal-core.mjs";

test("selects a fail-closed recovery action", () => {
  assert.equal(
    recoveryDecision({
      integrityValid: false,
      oldEvidenceReadable: true,
      newEvidenceWritable: true,
    }),
    "stop",
  );
  assert.equal(
    recoveryDecision({
      integrityValid: true,
      oldEvidenceReadable: false,
      newEvidenceWritable: false,
    }),
    "rollback",
  );
  assert.equal(
    recoveryDecision({
      integrityValid: true,
      oldEvidenceReadable: true,
      newEvidenceWritable: false,
    }),
    "forward-fix",
  );
  assert.equal(
    recoveryDecision({
      integrityValid: true,
      oldEvidenceReadable: true,
      newEvidenceWritable: true,
    }),
    "accept",
  );
});

test("identity policy rejects secret-like input", () => {
  assert.equal(
    assertIdentityAlias("registry-admin", "admin"),
    "registry-admin",
  );
  assert.throws(() => assertIdentityAlias("SABC", "admin"), /secret material/);
  assert.throws(
    () => assertIdentityAlias("word word", "admin"),
    /secret material/,
  );
});

test("same versioned storage snapshots are compatible", () => {
  const shape = evidenceRegistryStorageShape();
  assert.deepEqual(
    compareStorageSnapshots(shape, evidenceRegistryStorageShape()),
    {
      compatible: true,
      changes: [],
      migrationEvidence: null,
    },
  );
});

test("removed or retyped storage shapes fail closed without migration evidence", () => {
  const before = evidenceRegistryStorageShape();
  const removed = structuredClone(before);
  delete removed.persistentKeys.Evidence;
  assert.throws(
    () => compareStorageSnapshots(before, removed),
    /migration evidence/,
  );

  const retyped = structuredClone(before);
  retyped.instanceKeys.Admin = "BytesN<32>";
  assert.throws(
    () => compareStorageSnapshots(before, retyped),
    /migration evidence/,
  );
});

test("reviewed migration evidence permits an explicitly versioned change", () => {
  const before = evidenceRegistryStorageShape();
  const after = structuredClone(before);
  after.storageLayoutVersion = 2;
  after.persistentKeys.LegacyEvidence = after.persistentKeys.Evidence;
  delete after.persistentKeys.Evidence;
  const evidence = {
    reviewId: "review-2026-11-24",
    migrationPlan: "release/migration-notes.md#evidence-v2",
    sourceRevision: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    targetRevision: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  };
  const result = compareStorageSnapshots(before, after, evidence);
  assert.equal(result.compatible, true);
  assert.equal(result.migrationEvidence, "reviewed");
  assert.ok(result.changes.length >= 1);
});

test("incomplete migration evidence and boundary versions are rejected", () => {
  const before = evidenceRegistryStorageShape();
  const after = evidenceRegistryStorageShape(2, 2);
  assert.throws(
    () => compareStorageSnapshots(before, after, {}),
    /migration evidence/,
  );
  assert.throws(() => evidenceRegistryStorageShape(0, 1), /schema version/);
});
