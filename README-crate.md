# Symbolica for Typst

Rust library and WebAssembly plugin for using [Symbolica](https://symbolica.io)
in Typst. The library lets other plugins exchange exact expressions with
portable notation and annotations.

For the Typst package, see [Typst Universe](https://typst.app/universe/package/symbolica/).
For development and the current Typst API, see the
[repository](https://github.com/symbolica-dev/symbolica-typst-plugin).

## Rust library

```toml
[dependencies]
symbolica-typst-plugin = "0.1.0"
```

The public modules provide:

- `payload`: native Symbolica Atom exports with versioned, schema-keyed attachments.
- `math_display`: portable notation for decorated symbols and literal labels.
- `typst_ast`: conversion from Parsely trees to annotated Symbolica expressions.
- `expression_tree`: typed semantic trees, CBOR encoding, and conversion to and
  from annotated expressions, independent of Parsely and print layout.

The [expression-tree contract](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/docs/expression-tree.md)
describes the CBOR fields and shows how to inspect attachments before importing
the tree's native leaves.

Inspect and validate attachments before importing an Atom so your plugin can
register any required symbol callbacks. Preserve attachments you do not
interpret when exporting the result. Exchanging native Atoms requires compatible
Symbolica export formats; this release is tested with Symbolica 3.0.1 (format 6).

## Features

The default `native` feature uses GMP/MPFR. To embed the library in another Wasm
plugin, disable defaults and select `wasm`:

```toml
[dependencies]
symbolica-typst-plugin = { version = "0.1.0", default-features = false, features = ["wasm"] }
```

The optional `plugin` feature adds the standalone plugin's entry points,
integration engine, and runtime setup. Leave it disabled when embedding the
library. The consuming plugin supplies its own Wasm runtime imports and
getrandom backend. Normal builds produce an `rlib`; the repository's build
script explicitly selects `cdylib` for the standalone Typst plugin.

See the [API documentation](https://docs.rs/symbolica-typst-plugin) and
[build instructions](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/REBUILDING.md).

## Licensing

This crate's original Rust adapter code is MIT licensed. Symbolica retains its
[source-available license](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/LICENSE-SYMBOLICA.md).
The [Typst permission](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/LICENSE-SYMBOLICA-TYPST.md)
allows free use within Typst, including commercial use. Embedding the Rust
library outside Typst does not extend that permission to those uses.
