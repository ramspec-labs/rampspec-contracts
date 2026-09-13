#!/usr/bin/env bash
set -euo pipefail

temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT

bash scripts/build-reproducible.sh "${temp_dir}/wasm"
node scripts/generate-contract-specs.mjs "${temp_dir}/wasm" "${temp_dir}/specs"
diff -ru artifacts/specs "${temp_dir}/specs"
node scripts/generate-typescript-bindings.mjs "${temp_dir}/wasm" "${temp_dir}/bindings"
diff -ru --exclude=node_modules --exclude=dist artifacts/bindings "${temp_dir}/bindings"
