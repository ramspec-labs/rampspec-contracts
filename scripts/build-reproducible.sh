#!/usr/bin/env bash
set -euo pipefail

export CARGO_INCREMENTAL=0
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-0}"
export TZ=UTC
export LC_ALL=C.UTF-8

output_dir="${1:-artifacts/wasm}"
rm -rf "${output_dir}"
mkdir -p "${output_dir}"

stellar contract build --locked --out-dir "${output_dir}"

find "${output_dir}" -maxdepth 1 -type f -name '*.wasm' -print0 \
  | sort -z \
  | xargs -0 sha256sum \
  | sed "s#${output_dir}/##" > "${output_dir}/checksums.sha256"

rust_version="$(rustc --version)"
stellar_version="$(stellar --version | head -n 1 | tr -d '\r')"
source_revision="$(git rev-parse HEAD 2>/dev/null || printf 'uncommitted')"

cat > "${output_dir}/provenance.json" <<EOF
{
  "container": "rust:1.93.1-bookworm@sha256:7c4ae649a84014c467d79319bbf17ce2632ae8b8be123ac2fb2ea5be46823f31",
  "rust": "${rust_version}",
  "source_revision": "${source_revision}",
  "stellar_cli": "${stellar_version}",
  "target": "wasm32v1-none"
}
EOF
