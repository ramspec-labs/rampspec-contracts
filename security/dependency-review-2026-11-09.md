# Dependency Review - 2026-11-09

## Result

The weekly dependency and toolchain review completed with one time-bounded
exception and no vulnerability in the production contract WASM graph.

`RUSTSEC-2026-0387` affects `crypto-bigint 0.5.5`. The dependency is present in
native Soroban host tooling and policy-account fixture test signing. It is not
present in normal or build dependencies for `wasm32-unknown-unknown`.

## Evidence

```text
cargo audit --deny warnings
cargo tree --target wasm32-unknown-unknown -e normal,build -i crypto-bigint
cargo test --workspace --all-features
```

The target-specific dependency query returns no production-WASM path. Contract
P-256 verification remains delegated to the Soroban host.

## Decision

The exception is recorded as `vulnerable-non-production`, expires on
2026-12-09, and cannot authorize native production cryptography. Upgrade the
Soroban SDK and fixture signing stack when their supported dependency graph
moves to `crypto-bigint >=0.7.6`, then remove the exception.
