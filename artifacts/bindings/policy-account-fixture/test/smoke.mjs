import assert from "node:assert/strict";
import { Client } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["config","config_version","initialize","is_test_only","passkey_supported","policy_commitment","schema_version","set_controls","set_passkeys","set_signers"]);
