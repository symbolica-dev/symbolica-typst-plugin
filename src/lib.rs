//! Symbolica expressions and portable annotations for Typst plugins.
//!
//! The public modules provide payload interchange, notation, and Parsely AST
//! conversion. They can be used without the Symbolica Typst plugin's exports or
//! integration rules. Enable `wasm` without default features when embedding this
//! library in another Wasm plugin; enable `plugin` to build our plugin itself.

pub mod payload;
pub use payload::{expression_tree, math_display, typst_ast};

#[cfg(feature = "plugin")]
mod plugin;
