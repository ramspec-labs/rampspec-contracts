import assert from "node:assert/strict";
import { Client } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["accept_admin","get_attestor","get_evidence","initialize","is_active","maintain_attestor","maintain_evidence","propose_admin","publish_evidence","register_attestor","revoke_evidence","schema_version","set_attestor_enabled","set_paused","supersede_evidence","upgrade"]);
