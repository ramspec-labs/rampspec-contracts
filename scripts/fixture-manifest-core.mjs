const ALLOWED_NETWORKS = new Set(["local", "testnet"]);
const EXPECTED_FIXTURES = new Set([
  "policy-account-fixture",
  "web-auth-fixture",
]);

function fail(message) {
  throw new Error(`fixture manifest: ${message}`);
}

export function validateFixtureManifest(manifest) {
  if (!manifest || manifest.schemaVersion !== 1) {
    fail("schemaVersion must equal 1");
  }
  if (!Array.isArray(manifest.fixtures) || manifest.fixtures.length !== 2) {
    fail("exactly two fixtures are required");
  }

  const names = new Set();
  const adminRoles = new Set();
  for (const fixture of manifest.fixtures) {
    if (!fixture || typeof fixture !== "object") {
      fail("each fixture must be an object");
    }
    if (!EXPECTED_FIXTURES.has(fixture.name) || names.has(fixture.name)) {
      fail(`unexpected or duplicate fixture ${String(fixture.name)}`);
    }
    names.add(fixture.name);
    if (fixture.testOnly !== true || fixture.productionUse !== false) {
      fail(`${fixture.name} must be explicitly test-only`);
    }
    if (fixture.holdsFunds !== false) {
      fail(`${fixture.name} must explicitly reject fund custody`);
    }
    if (!Array.isArray(fixture.allowedNetworks) || fixture.allowedNetworks.length === 0) {
      fail(`${fixture.name} must declare allowed networks`);
    }
    for (const network of fixture.allowedNetworks) {
      if (!ALLOWED_NETWORKS.has(network)) {
        fail(`${fixture.name} includes forbidden network ${String(network)}`);
      }
    }
    if (typeof fixture.wasm !== "string" || !fixture.wasm.endsWith(".wasm")) {
      fail(`${fixture.name} must identify its WASM artifact`);
    }
    if (typeof fixture.adminRole !== "string" || fixture.adminRole.length === 0) {
      fail(`${fixture.name} must identify a dedicated admin role`);
    }
    if (adminRoles.has(fixture.adminRole)) {
      fail("fixture admin roles must be distinct");
    }
    adminRoles.add(fixture.adminRole);
  }

  for (const expected of EXPECTED_FIXTURES) {
    if (!names.has(expected)) {
      fail(`missing ${expected}`);
    }
  }
  return manifest;
}
