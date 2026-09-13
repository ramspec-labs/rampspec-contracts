import assert from "node:assert/strict";
import test from "node:test";
import { assertIdentityAlias, recoveryDecision } from "./upgrade-rehearsal-core.mjs";

test("selects a fail-closed recovery action", () => {
  assert.equal(recoveryDecision({ integrityValid: false, oldEvidenceReadable: true, newEvidenceWritable: true }), "stop");
  assert.equal(recoveryDecision({ integrityValid: true, oldEvidenceReadable: false, newEvidenceWritable: false }), "rollback");
  assert.equal(recoveryDecision({ integrityValid: true, oldEvidenceReadable: true, newEvidenceWritable: false }), "forward-fix");
  assert.equal(recoveryDecision({ integrityValid: true, oldEvidenceReadable: true, newEvidenceWritable: true }), "accept");
});

test("identity policy rejects secret-like input", () => {
  assert.equal(assertIdentityAlias("registry-admin", "admin"), "registry-admin");
  assert.throws(() => assertIdentityAlias("SABC", "admin"), /secret material/);
  assert.throws(() => assertIdentityAlias("word word", "admin"), /secret material/);
});
