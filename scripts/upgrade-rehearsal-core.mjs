export function recoveryDecision({
  integrityValid,
  oldEvidenceReadable,
  newEvidenceWritable,
}) {
  if (!integrityValid) return "stop";
  if (!oldEvidenceReadable) return "rollback";
  if (!newEvidenceWritable) return "forward-fix";
  return "accept";
}

export function assertIdentityAlias(value, name) {
  if (!value || value.startsWith("S") || /\s/.test(value)) {
    throw new Error(
      `${name} must be a configured identity alias without secret material`,
    );
  }
  return value;
}

export function evidenceRegistryStorageShape(
  schemaVersion = 1,
  storageLayoutVersion = 1,
) {
  if (
    !Number.isSafeInteger(schemaVersion) ||
    schemaVersion < 1 ||
    !Number.isSafeInteger(storageLayoutVersion) ||
    storageLayoutVersion < 1
  ) {
    throw new Error(
      "invalid storage schema version or layout version: expected positive integers",
    );
  }
  return {
    schemaVersion,
    storageLayoutVersion,
    instanceKeys: {
      Admin: "Address",
      PendingAdmin: "Address|null",
      Paused: "bool",
      SchemaVersion: "u32",
      CurrentWasmHash: "BytesN<32>|null",
    },
    persistentKeys: {
      Attestor: "Address -> AttestorRecord",
      Evidence: "BytesN<32> -> EvidenceRecord",
      ActiveReport: "(Address,BytesN<32>,NetworkKind) -> BytesN<32>",
      SupersededBy: "BytesN<32> -> BytesN<32>",
    },
  };
}

export function compareStorageSnapshots(
  before,
  after,
  migrationEvidence = null,
) {
  if (!before || !after)
    throw new Error("both before and after storage snapshots are required");

  const beforeInstance = before.instanceKeys ?? {};
  const afterInstance = after.instanceKeys ?? {};
  const beforePersistent = before.persistentKeys ?? {};
  const afterPersistent = after.persistentKeys ?? {};

  const keyChanges = [];
  for (const [namespace, beforeKeys, afterKeys] of [
    ["instance", beforeInstance, afterInstance],
    ["persistent", beforePersistent, afterPersistent],
  ]) {
    for (const [key, beforeShape] of Object.entries(beforeKeys)) {
      if (!(key in afterKeys)) {
        keyChanges.push({
          namespace,
          key,
          kind: "removed",
          before: beforeShape,
          after: null,
        });
      } else if (afterKeys[key] !== beforeShape) {
        keyChanges.push({
          namespace,
          key,
          kind: "retyped",
          before: beforeShape,
          after: afterKeys[key],
        });
      }
    }
  }

  if (
    before.storageLayoutVersion !== after.storageLayoutVersion ||
    before.schemaVersion !== after.schemaVersion
  ) {
    keyChanges.push({
      namespace: "schema",
      key: "storageVersion",
      kind: "retyped",
      before: {
        schemaVersion: before.schemaVersion,
        storageLayoutVersion: before.storageLayoutVersion,
      },
      after: {
        schemaVersion: after.schemaVersion,
        storageLayoutVersion: after.storageLayoutVersion,
      },
    });
  }

  if (keyChanges.length === 0) {
    return {
      compatible: true,
      changes: [],
      migrationEvidence: migrationEvidence ? "reviewed" : null,
    };
  }
  if (
    !migrationEvidence ||
    typeof migrationEvidence.reviewId !== "string" ||
    !migrationEvidence.reviewId ||
    typeof migrationEvidence.migrationPlan !== "string" ||
    !migrationEvidence.migrationPlan ||
    typeof migrationEvidence.sourceRevision !== "string" ||
    typeof migrationEvidence.targetRevision !== "string"
  ) {
    throw new Error(
      "removed, retyped, or undecodable storage shapes require migration evidence",
    );
  }
  return {
    compatible: true,
    changes: keyChanges,
    migrationEvidence: "reviewed",
  };
}
