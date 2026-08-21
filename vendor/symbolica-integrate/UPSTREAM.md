# Upstream source

This directory is vendored from `symbolica-dev/symbolica-integrate` commit
`fbb7dffa62cc4a560175b06e208aa8217e0f292e`.

The Rust source is unchanged. In `Cargo.toml`, the Symbolica dependency uses the
`2.2` package requirement so Tymbolica's workspace patch can route it through
the WebAssembly adapter, and package profiles are defined at the workspace root.
The upstream commit follows Symbolica's `dev` branch directly and enables native
allocator features that do not target `wasm32-unknown-unknown`.
