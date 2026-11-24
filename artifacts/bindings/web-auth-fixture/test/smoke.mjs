import assert from "node:assert/strict";
import { Client, decodeWebAuthFailure, interpretWebAuthResult, FIXTURE_CONFIG_GENERATION_METHOD } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["config","configure","initialize","is_nonce_consumed","is_test_only","reset_nonce","schema_version","sep_version","web_auth_verify"]);
assert.equal(FIXTURE_CONFIG_GENERATION_METHOD, "config_generation");
assert.deepEqual(decodeWebAuthFailure(9), { ok: false, code: 9, name: "Rejected" });
assert.deepEqual(decodeWebAuthFailure(99), { ok: false, code: 99, name: "Unknown" });
assert.deepEqual(decodeWebAuthFailure("nope"), { ok: false, code: null, name: "Unknown" });
assert.deepEqual(interpretWebAuthResult({ ok: true }), { ok: true });
assert.equal(interpretWebAuthResult({ ok: true, error: 1 }).ok, false);
assert.equal(interpretWebAuthResult({ code: 404 }).ok, false);
assert.equal(interpretWebAuthResult(null).ok, false);
