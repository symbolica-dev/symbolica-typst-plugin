# Migrating from Symbolica 0.1.0 to 0.2.0

Version **0.2.0 is prepared for release and is not yet published** on Typst
Universe. Published `@preview/symbolica:0.1.0` has the original API; the
checkout's manifest is now `0.2.0`.

For now, [install the checkout as a local package](https://github.com/symbolica-dev/symbolica-typst-plugin#try-the-development-version)
and use `@local/symbolica:0.2.0`. After the registry submission is merged and
deployed, use `@preview/symbolica:0.2.0`. The [changelog](CHANGELOG.md) lists the
new features and fixes.

## Rename public API calls

These names change in the package exports and in the dictionary returned by
`init()`. There are no compatibility aliases.

| Published 0.1.0 | New API |
| --- | --- |
| `math` | `parse` |
| `symbol` | `literal` |
| `function` | `function-head` |
| `terms` | `summands` |
| `content` | `matrix-content` |
| `vec` | `vector` |
| `det` | `determinant` |
| `cancel` | `cancel-factors` |
| `sub` | `subtract` |
| `div` | `divide` |

Update explicit imports, calls through a module such as `s.math(...)`, and
engine fields such as `engine.symbol`. Change Symbolica references only:
Typst's native `math`, `symbol`, `content`, and math notation such as
`$vec(x, y)$` keep their meanings. Symbolica expression strings also retain
their built-in names, such as `"gamma(x)"`.

Before, with the published package:

```typst
#import "@preview/symbolica:0.1.0" as s

#let x = s.symbol("x")
#let f = s.math($x^4 - 5 x^2 + 4$)
$ #s.to-typst(s.factor(f)), quad #s.to-typst(s.derivative(f, x)) $
```

After, with the prepared 0.2.0 checkout:

```typst
#import "@local/symbolica:0.2.0" as s

#let x = s.literal("x")
#let f = s.parse($x^4 - 5 x^2 + 4$)
$ #s.to-typst(s.factor(f)), quad #s.to-typst(s.derivative(f, x)) $
```

For an engine with a custom namespace, replace bindings such as
`#let (math, symbol) = s.init(namespace: "physics")` with:

```typst
#let (parse, literal) = s.init(namespace: "physics")
#let x = literal("x")
#let f = parse($#x^2 + 1$)
#s.to-typst(f)
```

## Replace `atom` with `parse`

`atom` and `init().atom` are removed. Use `parse` for scalar expressions from
Typst math, Symbolica strings, numbers, or existing expression bytes:

```typst
#let from-math = s.parse($2 x + y$)
#let from-string = s.parse("2*x + y")
#let exact = s.parse(3)
#let existing = s.parse(from-math)
```

Strings use Symbolica syntax. Use explicit `*` to make multiplication
unambiguous. Algebra operations accept strings for individual numbers or
symbols; wrap a complete string expression in `parse`, for example
`s.factor(s.parse("x^2 - 1"))`.

A `grammar` override applies only to Typst content. Remove it from calls that
parse strings, numbers, or bytes. A `namespace` override assigns newly parsed
names; it does not rename symbols inside an existing payload. Use `matrix`
and `vector` for matrix-valued input.

## Use `literal` for labels and `wild` for patterns

`literal("x")` replaces `symbol("x")`. It can also construct one symbol with
a mathematical or text label. Choose `parse` when the content should be
interpreted algebraically:

```typst
#let label = s.literal($x+y$) // One variable with the label x+y.
#let sum = s.parse($x+y$)    // A sum of two variables.
#s.to-typst(s.pow(label, 2))
```

Use `name` with a content label when distinct variables need the same
appearance, for example `s.literal($x$, name: "position")`. Declare that label
before using its name elsewhere. Unsupported styled or contextual content
reports an error; use a string literal with a custom `notation` renderer
for such display content.

Ordinary literal names cannot end in `_`. For a pattern placeholder, use
`s.wild("a")`; for multiple trailing underscores, set `level`, for example
`s.wild("a", level: 2)`. Pass the base name without underscores. Names must be
nonempty, and `level` must be a positive integer. Replace `wild("a", level: 0)`
with `literal("a")` when you want an ordinary variable.

Subscripts, primes, and corner attachments now survive parsing and rendering
as literal notation. Label order matters: `C_(i j)` and `C_(j i)` are distinct.
A prime in a label does not differentiate it; use `derivative` for calculus.
Use `to-typst` when a rendered expression will be parsed again: it carries the
exact expression metadata. `to-typst-source` retains the visible notation
without that metadata.

`to-typst` now returns one inline equation by default. Use
`s.to-typst(f, block: true)` for a standalone display equation. If custom code
inspects the returned content as a sequence, update it to handle the outer
equation; its `body` contains the inner math content.

## Regenerate saved expression bytes

The engine changes from Symbolica 3.0.0 to 3.0.1, and its native Atom export
format changes from **5 to 6**. Regenerate saved expression and matrix payloads
from their original formulas with the new engine. Old native bytes cannot be
made compatible by changing an envelope's version field.
`parse(old-bytes)` passes bytes through unchanged; it does not convert their
native format.

Documents that construct their expressions during compilation recreate those
payloads automatically. Applications that save bytes between runs, or exchange
them with another plugin, need to rebuild both producers and consumers against
a compatible Symbolica version. The outer Atom payload envelope remains
version 2; its version is separate from the native Atom format.

The bundled engine is now an uncompressed `symbolica.wasm`, loaded directly.
If you copied the old package files into your own project, replace the runtime
and Typst interface together. The new interface does not use
`symbolica-inflate.wasm` or `symbolica.wasm.zlib`. A custom `init(source: ...)`
must receive an uncompressed Wasm module compatible with the new interface.

## Update Rust plugin dependencies

The separate `symbolica-typst-atom-payload` crate is folded into
`symbolica-typst-plugin`. Change imports such as:

```rust
use symbolica_typst_atom_payload as payload;
use symbolica_typst_atom_payload::{math_display, typst_ast};
```

to:

```rust
use symbolica_typst_plugin::{payload, math_display, typst_ast};
```

Until the crate is published, use a path dependency or a pinned Git revision.
Native consumers use the default `native` feature. Wasm consumers disable
default features and enable `wasm`:

```toml
[dependencies]
symbolica-typst-plugin = { path = "../symbolica-typst-plugin", default-features = false, features = ["wasm"] }
```

Leave `plugin` disabled when embedding the reusable library; it enables the
standalone plugin's entry points and integration engine. Symbolica 3.0.1 comes
from crates.io, so remove an old Symbolica Git patch if it was present only to
build this library. The crate currently requires Rust 1.97.

When importing payloads, inspect their attachments and register any required
symbol-data readers before importing native Atoms. Preserve annotations you
do not interpret when exporting results.

The new `inspect`/`from-tree` API and Rust `expression_tree` module expose a
[version 1 semantic tree contract](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/docs/expression-tree.md).
Its symbol and coefficient leaves retain native payloads, so tree versioning
does not remove the engine compatibility requirement above.
