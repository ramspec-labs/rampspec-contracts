import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { Client } from "../dist/index.js";

const client = new Client({
  allowHttp: true,
  contractId: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
  networkPassphrase: "Standalone Network ; February 2017",
  rpcUrl: "http://127.0.0.1:8000",
});
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["accept_admin","cancel_admin_transfer","get_attestor","get_evidence","initialize","is_active","maintain_attestor","maintain_evidence","maintain_supersession_chain","propose_admin","publish_evidence","register_attestor","revoke_evidence","schema_version","set_attestor_enabled","set_paused","supersede_evidence","upgrade"]);

const vectors = JSON.parse(
  readFileSync(new URL("../../../../fixtures/evidence-id-vectors.json", import.meta.url), "utf8"),
);
assert.equal(vectors.schemaVersion, 1);
assert.equal(vectors.hashAlgorithm, "sha256");
assert.equal(vectors.domain, "rampspec-evidence-v1");
assert.equal(vectors.publisherAddressEncoding, "stellar-soroban-scval-xdr");
assert.equal(vectors.networkEncoding, "u32-big-endian");

for (const vector of vectors.vectors) {
  assert.equal(Buffer.from(vector.publisherAddressXdrHex, "hex").length, 44);
  const network = Buffer.alloc(4);
  network.writeUInt32BE(vector.networkDiscriminant);
  const preimage = Buffer.concat([
    Buffer.from(vectors.domain, "ascii"),
    Buffer.from(vector.publisherAddressXdrHex, "hex"),
    Buffer.from(vector.reportHashHex, "hex"),
    network,
  ]);
  assert.equal(preimage.toString("hex"), vector.preimageHex);
  assert.equal(createHash("sha256").update(preimage).digest("hex"), vector.evidenceIdHex);
}
assert.deepEqual(Object.keys(client.fromJSON).sort(), ["accept_admin","cancel_admin_transfer","get_attestor","get_evidence","initialize","is_active","maintain_attestor","maintain_evidence","propose_admin","publish_evidence","register_attestor","revoke_evidence","schema_version","set_attestor_enabled","set_paused","supersede_evidence","upgrade"]);
