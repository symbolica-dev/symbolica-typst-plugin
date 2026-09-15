# Upstream source

This directory is vendored from `symbolica-dev/symbolica-integrate` commit
`fbb7dffa62cc4a560175b06e208aa8217e0f292e`.

The Rust source has one compatibility adjustment for Symbolica's replacement
API: the top-level-only matcher uses `min_level(0).max_level(0)` instead of
`level_range((0, Some(0)))`. Its matching behavior is unchanged.

In `Cargo.toml`, the Symbolica dependency uses version `3.0.0` from crates.io
with default features disabled so its Wasm consumer can enable only
`symbolica/wasm`. Package profiles are defined at the workspace root.
