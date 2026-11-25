import assert from "node:assert/strict";
import { Client } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["config","configure","initialize","is_nonce_consumed","is_test_only","reset_nonce","schema_version","sep_version","web_auth_verify"]);


// Verify misspelled / unsupported keys fail deterministically with MalformedArgument
await assert.rejects(
  async () => {
    await client.web_auth_verify({
      args: new Map([
        ['account', accountId],
        ['home_domain', 'example.org'],
        ['web_auth_domain', 'auth.example.org'],
        ['web_auth_domain_account', serverAccountId],
        ['nonce', 'nonce-123456'],
        ['client_domian', 'wallet.example.org'], // intentional typo
      ]),
    });
  },
  (err) => String(err).includes('MalformedArgument') || String(err).includes('Error(Contract,'),
  'Expected deterministic MalformedArgument rejection for unknown key',
);