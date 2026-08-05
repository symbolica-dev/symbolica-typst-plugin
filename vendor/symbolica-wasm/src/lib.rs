//! Feature adapter for `symbolica-integrate` in WebAssembly builds.
//!
//! The integration crate depends on Symbolica with its native default features.
//! Re-exporting the project's existing Wasm-configured dependency keeps a single
//! set of Symbolica types while disabling native-only backends.

pub use symbolica_upstream::*;
