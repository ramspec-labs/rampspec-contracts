#!/usr/bin/env bash
set -euo pipefail

temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT

bash scripts/build-reproducible.sh "${temp_dir}/wasm"
node scripts/generate-contract-specs.mjs "${temp_dir}/wasm" "${temp_dir}/specs"
diff -ru artifacts/specs "${temp_dir}/specs"
