# Upstream source

This directory is vendored from `symbolica-dev/symbolica-integrate` commit
`fbb7dffa62cc4a560175b06e208aa8217e0f292e`.

The Rust source is unchanged. In `Cargo.toml`, the Symbolica dependency uses the
`2.2` package requirement with default features disabled so Tymbolica's
workspace patch can select the pinned upstream `dev` revision and its Wasm
consumer can enable only `symbolica/wasm`. Package profiles are defined at the
workspace root.
