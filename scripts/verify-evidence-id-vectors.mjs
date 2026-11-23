import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const EVIDENCE_ID_DOMAIN = Buffer.from("rampspec-evidence-v1", "utf8");

const NETWORK_DISCRIMINANTS = {
  Testnet: 0,
  Futurenet: 1,
  Pubnet: 2,
  Standalone: 3,
};

// Base32 decoding for Stellar addresses (simplified, matches StrKey.decodeEd25519PublicKey)
const BASE32_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

function decodeBase32(input) {
  const bits = [];
  for (const char of input) {
    const value = BASE32_ALPHABET.indexOf(char.toUpperCase());
    if (value === -1) throw new Error(`Invalid base32 character: ${char}`);
    bits.push(...value.toString(2).padStart(5, "0").split(""));
  }
  
  const bytes = [];
  for (let i = 0; i < bits.length; i += 8) {
    if (i + 8 <= bits.length) {
      bytes.push(parseInt(bits.slice(i, i + 8).join(""), 2));
    }
  }
  return Buffer.from(bytes);
}

function decodeEd25519PublicKey(address) {
  // Stellar addresses format: G (1 byte version) + 32 bytes public key + 2 bytes checksum
  // All encoded in base32
  if (!address.startsWith("G")) {
    throw new Error("Invalid Ed25519 public key address - must start with 'G'");
  }
  
  const decoded = decodeBase32(address);
  if (decoded.length < 35) {
    throw new Error("Invalid Ed25519 public key address - too short");
  }
  
  // Extract the 32-byte public key (skip 1-byte version, ignore 2-byte checksum)
  return decoded.slice(1, 33);
}

/**
 * Derive evidence ID using the exact algorithm from evidence_id.rs:
 * SHA256(domain_separator || publisher_xdr || report_hash || network_discriminant)
 */
function deriveEvidenceId(publisher, reportHashHex, network) {
  // Validate inputs
  if (!publisher.startsWith("G") || publisher.length !== 56) {
    throw new Error(`Invalid publisher address: ${publisher}`);
  }
  if (!/^[0-9a-fA-F]{64}$/.test(reportHashHex)) {
    throw new Error(`Invalid report hash: ${reportHashHex}`);
  }
  if (!(network in NETWORK_DISCRIMINANTS)) {
    throw new Error(`Unsupported network: ${network}`);
  }

  // Encode publisher to Soroban XDR (ScAddress::Account)
  const publicKey = decodeEd25519PublicKey(publisher);
  const publisherXdr = Buffer.concat([
    Buffer.from([0, 0, 0, 0]), // ScAddress::Account discriminant (0 as i32)
    publicKey,
  ]);

  // Parse report hash
  const reportHash = Buffer.from(reportHashHex, "hex");

  // Encode network discriminant as big-endian u32
  const networkDiscriminant = NETWORK_DISCRIMINANTS[network];
  const networkBytes = Buffer.alloc(4);
  networkBytes.writeUInt32BE(networkDiscriminant, 0);

  // Build preimage: domain || publisher_xdr || report_hash || network
  const preimage = Buffer.concat([
    EVIDENCE_ID_DOMAIN,
    publisherXdr,
    reportHash,
    networkBytes,
  ]);

  // SHA256 hash
  return createHash("sha256").update(preimage).digest("hex");
}

/**
 * Verify all vectors in the fixture
 */
async function verifyVectors() {
  const fixturePath = resolve("fixtures/evidence-id-vectors.json");

  let fixture;
  try {
    const content = await readFile(fixturePath, "utf8");
    fixture = JSON.parse(content);
  } catch (error) {
    console.error(`✗ Failed to load fixture: ${error.message}`);
    process.exit(1);
  }

  // Validate fixture structure
  if (!fixture.schemaVersion) {
    console.error("✗ Missing schemaVersion in fixture");
    process.exit(1);
  }
  if (!Array.isArray(fixture.vectors)) {
    console.error("✗ Missing or invalid vectors array");
    process.exit(1);
  }
  if (fixture.vectors.length === 0) {
    console.error("✗ Empty vectors array");
    process.exit(1);
  }

  console.log(
    `Verifying ${fixture.vectors.length} evidence ID vector(s)...\n`
  );

  let allPassed = true;

  for (const [index, vector] of fixture.vectors.entries()) {
    const { description, publisher, reportHash, network, expectedId } = vector;

    // Validate required fields
    if (!publisher || !reportHash || !network || !expectedId) {
      console.error(
        `✗ Vector ${index + 1}: Missing required fields (publisher, reportHash, network, expectedId)`
      );
      allPassed = false;
      continue;
    }

    try {
      const derivedId = deriveEvidenceId(publisher, reportHash, network);

      if (derivedId === expectedId.toLowerCase()) {
        console.log(
          `✓ Vector ${index + 1}: ${description || "Unnamed vector"}`
        );
        console.log(`  Publisher: ${publisher}`);
        console.log(`  Report Hash: ${reportHash}`);
        console.log(`  Network: ${network}`);
        console.log(`  Expected ID: ${expectedId}`);
        console.log(`  Derived ID:  ${derivedId}`);
        console.log();
      } else {
        console.error(
          `✗ Vector ${index + 1}: ${description || "Unnamed vector"}`
        );
        console.error(`  Publisher: ${publisher}`);
        console.error(`  Report Hash: ${reportHash}`);
        console.error(`  Network: ${network}`);
        console.error(`  Expected ID: ${expectedId}`);
        console.error(`  Derived ID:  ${derivedId}`);
        console.error(`  ❌ ID mismatch\n`);
        allPassed = false;
      }
    } catch (error) {
      console.error(
        `✗ Vector ${index + 1}: ${description || "Unnamed vector"}`
      );
      console.error(`  Error: ${error.message}\n`);
      allPassed = false;
    }
  }

  if (allPassed) {
    console.log(
      `✓ All ${fixture.vectors.length} evidence ID vector(s) passed verification`
    );
    process.exit(0);
  } else {
    console.error(`✗ Some vectors failed verification`);
    process.exit(1);
  }
}

verifyVectors();
