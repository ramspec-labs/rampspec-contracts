# Release Policy

Contract releases use semantic versions plus exact optimized WASM SHA-256 hashes. A release requires clean formatting, Clippy, unit and integration tests, property and fuzz smoke checks, resource-budget review, canonical generated specifications and bindings, reproducible WASM checksums, an SBOM, provenance, and reviewed deployment metadata.

Production or public-network use additionally requires external security review closure, explicit administrator and attestor custody decisions, and governance approval. Fixture contracts remain test-only and are not deployed to a public network without a separately documented interoperability need.
