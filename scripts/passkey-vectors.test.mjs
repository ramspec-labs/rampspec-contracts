// Reproduces fixtures/passkey-assertion-vectors.json with Node's own P-256 implementation, so the
// published outcomes do not depend on the Rust toolchain that wrote them. The order of checks
// mirrors the policy-account fixture's __check_auth (see the file's "verification" field).
import assert from "node:assert/strict";
import { createPublicKey, verify } from "node:crypto";
import { readFile } from "node:fs/promises";
import test from "node:test";

const fixture = JSON.parse(await readFile("fixtures/passkey-assertion-vectors.json", "utf8"));

const P256_ORDER = 0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551n;
const MIN_AUTHENTICATOR_DATA = 37;
const MAX_AUTHENTICATOR_DATA = 512;

const bytes = (hex) => Buffer.from(hex, "hex");
const scalar = (buffer) => BigInt(`0x${buffer.toString("hex")}`);

function publicKey(uncompressed) {
  assert.equal(uncompressed.length, 65);
  assert.equal(uncompressed[0], 0x04);
  return createPublicKey({
    format: "jwk",
    key: {
      kty: "EC",
      crv: "P-256",
      x: uncompressed.subarray(1, 33).toString("base64url"),
      y: uncompressed.subarray(33).toString("base64url"),
    },
  });
}

function outcome(vector) {
  const assertion = vector.assertion;
  const configured = new Set(fixture.policy.passkeySigners.map((signer) => signer.publicKey));
  const authenticatorData = bytes(assertion.authenticatorData);
  const signature = bytes(assertion.signature);
  const r = scalar(signature.subarray(0, 32));
  const s = scalar(signature.subarray(32));

  if (assertion.challenge !== vector.payload) {
    return { result: "contractError", error: "InvalidPasskey", code: 16 };
  }
  if (
    authenticatorData.length < MIN_AUTHENTICATOR_DATA ||
    authenticatorData.length > MAX_AUTHENTICATOR_DATA
  ) {
    return { result: "contractError", error: "InvalidPasskey", code: 16 };
  }
  if (r === 0n || s === 0n || r >= P256_ORDER || s > P256_ORDER / 2n) {
    return { result: "contractError", error: "InvalidPasskey", code: 16 };
  }
  if (!configured.has(assertion.publicKey)) {
    return { result: "contractError", error: "UnauthorizedSigner", code: 6 };
  }
  const message = Buffer.concat([authenticatorData, bytes(assertion.clientDataHash)]);
  const valid = verify(
    "sha256",
    message,
    { key: publicKey(bytes(assertion.publicKey)), dsaEncoding: "ieee-p1363" },
    signature,
  );
  return valid ? { result: "accepted" } : { result: "hostTrap" };
}

test("every published outcome reproduces with Node's P-256 implementation", () => {
  for (const vector of fixture.vectors) {
    assert.deepEqual(outcome(vector), vector.expected, vector.name);
  }
});

test("covers positive vectors, every contract refusal and host traps", () => {
  const results = new Set(fixture.vectors.map((v) => v.expected.error ?? v.expected.result));
  for (const kind of ["accepted", "InvalidPasskey", "UnauthorizedSigner", "hostTrap"]) {
    assert.ok(results.has(kind), `no ${kind} vector`);
  }
  const payloads = fixture.vectors.map((v) => v.payload);
  assert.equal(new Set(payloads).size, payloads.length, "payloads must be unique");
});

test("contains only public material", () => {
  const text = JSON.stringify(fixture).toLowerCase();
  for (const forbidden of ["privatekey", "private_key", "secret", "seedbytes", "mnemonic", "http"]) {
    assert.ok(!text.includes(forbidden), `fixture mentions ${forbidden}`);
  }
  for (const vector of fixture.vectors) {
    assert.deepEqual(Object.keys(vector.assertion).sort(), [
      "authenticatorData",
      "challenge",
      "clientDataHash",
      "publicKey",
      "signature",
    ]);
  }
});
