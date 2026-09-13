# RampSpec Contracts

Soroban contracts for publishing compact RampSpec report commitments and for running deterministic authentication test fixtures.

This repository owns contract source, generated specifications and bindings, reproducible WASM artifacts, checksums, and deployment manifests. It does not own hosted API state, full reports, domains or URLs, customer data, credentials, KYC data, or funds. Registry records are integrity evidence, not certification or legal-compliance decisions.

## Contracts

- `evidence-registry`: production-oriented immutable report commitments.
- `web-auth-fixture`: test-only SEP-45 web-auth behavior.
- `policy-account-fixture`: test-only contract-account authorization policies.

See [implementation.md](implementation.md) for the ordered delivery plan and [SECURITY.md](SECURITY.md) for responsible disclosure.
Authentication fixture restrictions are defined in [docs/fixtures.md](docs/fixtures.md) and enforced from [fixtures/manifest.json](fixtures/manifest.json).

## License

Apache-2.0. See [LICENSE](LICENSE).
