//! Shared Symbolica feature adapter for the Tymbolica WebAssembly plugins.
//!
//! Some upstream crates request Symbolica features intended for native builds.
//! Re-exporting one Wasm-configured upstream revision keeps a single set of
//! Symbolica types while disabling native-only backends.

pub use symbolica_upstream::*;
