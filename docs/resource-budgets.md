# Resource Budgets

The reviewed upper bounds are stored in `tests/resource-budgets/baselines.json`. CI compares that inventory with each optimized WASM specification, so a new public method cannot ship without CPU, memory, storage-write, event, and TTL classifications.

Native Soroban tests enforce representative maximum-input and replacement-chain ceilings. Native estimates understate WASM execution, so these values are regression tripwires rather than fee predictions. Before any testnet or pubnet release, simulate the exact optimized WASM and record ledger, RPC, CPU instructions, memory bytes, read/write footprint, event bytes, and resource fee in the release evidence.

The default regression tolerance is 15 percent. Raising a ceiling requires a reviewed change to the baseline file, an explanation in the pull request, and a fresh optimized-WASM simulation. A toolchain, SDK, protocol, storage-layout, or compiler change invalidates comparisons until the baseline is remeasured.

Storage growth is linear for ordinary publication and explicitly modeled in the baseline. Supersession adds the replacement record, active lookup, and forward link. TTL maintenance requires at least one successful extension transaction per live persistent entry inside each configured TTL window. Pubnet approval remains blocked until live entry count, current network fee rates, and retention targets produce a reviewed cost forecast.
