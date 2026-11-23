# Release Policy

Contract releases use semantic versions plus exact optimized WASM SHA-256 hashes. A release requires clean formatting, Clippy, unit and integration tests, property and fuzz smoke checks, resource-budget review, canonical generated specifications and bindings, reproducible WASM checksums, an SBOM, provenance, and reviewed deployment metadata.

The pinned Linux container defines the canonical release WASM hashes. Native Windows generation must produce byte-identical JSON/XDR interfaces and source metadata, but host-specific optimized WASM hashes cannot authorize a release. Release packaging fails closed when a built WASM does not match the canonical manifest.

Production or public-network use additionally requires external security review closure, explicit administrator and attestor custody decisions, and governance approval. Fixture contracts remain test-only and are not deployed to a public network without a separately documented interoperability need.

## Fuzz Regression Fixtures

When fuzzing discovers a meaningful failure condition or security-relevant boundary, the minimized input becomes a permanent regression fixture:

1. **Minimize**: Run `cargo fuzz cmin <target_name>` to reduce the input to its smallest reproducing form.
2. **Review**: Verify the failure represents a genuine security boundary, invariant violation, or edge case worth preserving.
3. **Name**: Choose a stable, descriptive filename that documents what the fixture tests (e.g., `truncated_xdr.bin`, `zero_upgrade_hash.bin`).
4. **Store**: Copy the minimized input to `fuzz/regression-fixtures/<target_name>/<descriptive-name>.bin`.
5. **Verify**: Ensure the fixture replays correctly and is covered by CI validation.
6. **Commit**: Add the fixture with a clear commit message explaining the condition it exercises.

Regression fixtures are version-controlled, bounded in size, and must never contain credentials, secrets, or sensitive data. Each fixture must remain reproducible and serve a unique testing purpose. Duplicate or unnecessary fixtures should be removed during release preparation.

