#!/usr/bin/env bash
set -euo pipefail
target=wasm32-unknown-unknown
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
cargo build --release --target "$target" --package symbolica-typst-plugin --no-default-features

engine_raw="symbolica/symbolica.wasm"
# Symbolica's C API exports keep unrelated code alive in the linker.
# Keep simplify_expr, which belongs to this plugin's Typst API.
wasm-opt --remove-exports --pass-arg='remove-exports@__getrandom*' \
  --remove-exports --pass-arg='remove-exports@drop' \
  --remove-exports --pass-arg='remove-exports@get_license_key' \
  --remove-exports --pass-arg='remove-exports@init' \
  --remove-exports --pass-arg='remove-exports@is_licensed' \
  --remove-exports --pass-arg='remove-exports@request_*' \
  --remove-exports --pass-arg='remove-exports@set_*' \
  --remove-exports --pass-arg='remove-exports@simplify' \
  --remove-exports --pass-arg='remove-exports@simplify_factorized' \
  --remove-exports --pass-arg='remove-exports@use_hu_*' \
  --remove-unused-module-elements -Oz --quiet \
  --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int --enable-simd --strip-debug --strip-producers \
  -o "$engine_raw" "target/$target/release/symbolica_typst_plugin.wasm"
