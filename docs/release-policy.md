# Release Policy

Contract releases use semantic versions plus exact optimized WASM SHA-256 hashes. A release requires clean formatting, Clippy, unit and integration tests, property and fuzz smoke checks, resource-budget review, canonical generated specifications and bindings, reproducible WASM checksums, an SBOM, provenance, and reviewed deployment metadata.

The pinned Linux container defines the canonical release WASM hashes. Native Windows generation must produce byte-identical JSON/XDR interfaces and source metadata, but host-specific optimized WASM hashes cannot authorize a release. Release packaging fails closed when a built WASM does not match the canonical manifest.

Production or public-network use additionally requires external security review closure, explicit administrator and attestor custody decisions, and governance approval. Fixture contracts remain test-only and are not deployed to a public network without a separately documented interoperability need.
