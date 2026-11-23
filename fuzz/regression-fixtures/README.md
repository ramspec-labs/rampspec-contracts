# Fuzz Regression Fixtures

This directory contains minimized, reviewed fuzz inputs that have been accepted as permanent regression tests. Each subdirectory corresponds to a fuzz target.

## Structure

```
regression-fixtures/
├── decode_evidence/
│   └── [descriptive-name].bin
├── evidence_id_encoding/
│   └── [descriptive-name].bin
├── upgrade_inputs/
│   └── [descriptive-name].bin
└── registry_transitions/
    └── [descriptive-name].bin
```

## Adding a Regression Fixture

When fuzzing discovers a failure:

1. **Minimize the input**:
   ```bash
   cargo fuzz cmin <target_name>
   ```

2. **Review the failure**: Verify it represents a meaningful security boundary or invariant violation.

3. **Name it descriptively**: Choose a stable, meaningful name that describes the condition being tested.
   - Good: `truncated_address_xdr.bin`, `zero_hash_upgrade.bin`
   - Bad: `crash-123.bin`, `input.bin`

4. **Copy to regression-fixtures**:
   ```bash
   cp fuzz/corpus/<target>/<hash> fuzz/regression-fixtures/<target>/<descriptive-name>.bin
   ```

5. **Verify replay**: Ensure the fixture can be loaded and replayed:
   ```bash
   cargo test --package rampspec-contract-fuzz
   ```

6. **Commit the fixture**: Add and commit the fixture with a clear commit message explaining what it tests.

## Maintenance

- Keep fixtures small and minimal
- Remove duplicates that test the same condition
- Ensure every fixture has a clear purpose
- Never commit credentials, keys, or sensitive data
- Review fixtures during security audits

## Automated Testing

Regression fixtures are automatically tested by:
- Fuzz target compilation checks
- Integration test harnesses (if implemented)
- CI validation workflows

Each fixture must remain valid and reproducible across all supported platforms.
