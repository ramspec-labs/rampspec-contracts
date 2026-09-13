import assert from "node:assert/strict";
import { Client } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["config","configure","initialize","is_nonce_consumed","is_test_only","reset_nonce","schema_version","sep_version","web_auth_verify"]);
