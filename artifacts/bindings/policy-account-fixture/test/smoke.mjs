import assert from "node:assert/strict";
import { Client, PolicyError } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});

// Test that client methods are properly generated
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["config","config_version","initialize","is_test_only","passkey_supported","schema_version","set_controls","set_passkeys","set_signers"]);

// Test PolicyError enumeration structure
assert.strictEqual(typeof PolicyError, "object", "PolicyError should be an object");
assert.ok(PolicyError[1], "PolicyError should have code 1");
assert.ok(PolicyError[2], "PolicyError should have code 2");
assert.ok(PolicyError[3], "PolicyError should have code 3");
assert.ok(PolicyError[4], "PolicyError should have code 4");
assert.ok(PolicyError[5], "PolicyError should have code 5");
assert.ok(PolicyError[6], "PolicyError should have code 6");
assert.ok(PolicyError[7], "PolicyError should have code 7");
assert.ok(PolicyError[8], "PolicyError should have code 8");
assert.ok(PolicyError[9], "PolicyError should have code 9");
assert.ok(PolicyError[10], "PolicyError should have code 10");
assert.ok(PolicyError[11], "PolicyError should have code 11");
assert.ok(PolicyError[12], "PolicyError should have code 12");
assert.ok(PolicyError[13], "PolicyError should have code 13");
assert.ok(PolicyError[14], "PolicyError should have code 14");
assert.ok(PolicyError[15], "PolicyError should have code 15");
assert.ok(PolicyError[16], "PolicyError should have code 16");
assert.ok(PolicyError[17], "PolicyError should have code 17");

// Test that all documented error codes have message fields
assert.strictEqual(PolicyError[1].message, "NotInitialized");
assert.strictEqual(PolicyError[2].message, "AlreadyInitialized");
assert.strictEqual(PolicyError[3].message, "InvalidPolicy");
assert.strictEqual(PolicyError[4].message, "MissingSignature");
assert.strictEqual(PolicyError[5].message, "DuplicateSigner");
assert.strictEqual(PolicyError[6].message, "UnauthorizedSigner");
assert.strictEqual(PolicyError[7].message, "InsufficientWeight");
assert.strictEqual(PolicyError[8].message, "InvalidTimeWindow");
assert.strictEqual(PolicyError[9].message, "NotYetValid");
assert.strictEqual(PolicyError[10].message, "Expired");
assert.strictEqual(PolicyError[11].message, "Rejected");
assert.strictEqual(PolicyError[12].message, "Replay");
assert.strictEqual(PolicyError[13].message, "RequiredSignerMissing");
assert.strictEqual(PolicyError[14].message, "InvalidContext");
assert.strictEqual(PolicyError[15].message, "WrongSignatureMode");
assert.strictEqual(PolicyError[16].message, "InvalidPasskey");
assert.strictEqual(PolicyError[17].message, "TooManySignatures");

// Test error code distinction
const errorCodes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
const errorMessages = errorCodes.map(code => PolicyError[code].message);
const uniqueMessages = new Set(errorMessages);
assert.strictEqual(uniqueMessages.size, errorCodes.length, "All error messages should be unique");

// Test that unknown error codes are handled safely
assert.strictEqual(PolicyError[0], undefined, "Unknown error code 0 should be undefined");
assert.strictEqual(PolicyError[18], undefined, "Unknown error code 18 should be undefined");
assert.strictEqual(PolicyError[999], undefined, "Unknown error code 999 should be undefined");

// Test error code to name mapping consistency
const errorNameMap = {
  1: "NotInitialized",
  2: "AlreadyInitialized",
  3: "InvalidPolicy",
  4: "MissingSignature",
  5: "DuplicateSigner",
  6: "UnauthorizedSigner",
  7: "InsufficientWeight",
  8: "InvalidTimeWindow",
  9: "NotYetValid",
  10: "Expired",
  11: "Rejected",
  12: "Replay",
  13: "RequiredSignerMissing",
  14: "InvalidContext",
  15: "WrongSignatureMode",
  16: "InvalidPasskey",
  17: "TooManySignatures",
};

for (const [code, expectedName] of Object.entries(errorNameMap)) {
  const numCode = Number(code);
  assert.strictEqual(
    PolicyError[numCode].message,
    expectedName,
    `Error code ${numCode} should map to ${expectedName}`
  );
}

console.log("✓ All PolicyError decoding tests passed");
console.log("✓ Client method generation verified");

