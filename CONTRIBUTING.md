# Contributing

Keep each change narrow and add tests for every authorization, state-transition, serialization, and error path it affects. Run formatting, Clippy with warnings denied, tests, deterministic generation, and checksum checks before requesting review.

Generated specifications, bindings, and WASM must come from repository scripts and must not be edited by hand. Never add credentials, customer data, report bodies, raw domains, URLs with tokens, or fund-custody behavior.

All changes require review. Security-sensitive changes to authorization, storage, TTL, upgrades, cryptography, or deployment need a maintainer explicitly responsible for contract security.
