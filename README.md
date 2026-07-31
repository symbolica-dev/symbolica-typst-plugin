# Tymbolica

Exact symbolic computation inside Typst, powered by
[Symbolica](https://symbolica.io/).

Tymbolica lets the formulas on a Typst page take part in the calculation. Write
an expression as ordinary Typst mathematics, work with it symbolically, and
place the result back into the same document. You can currently:

- expand, factor, collect, differentiate, and inspect expressions;
- combine, cancel, or decompose rational functions;
- integrate polynomials and calculate series;
- replace recurring patterns with wildcards;
- solve systems exactly or numerically;
- evaluate formulas over points or grids; and
- solve exact matrix problems.

Start with the [user manual](typst/manual.pdf), the
[minimal example](typst/examples/basic.typ), or the
[polynomial-system showcase](typst/examples/showcase.typ). The manual contains
the conceptual guide, task-oriented examples, limitations, and complete API
reference.

## Quick start

After installing the local package below, this is a complete Typst document:

```typst
#import "@local/tymbolica:0.1.0": *

#let x = var("x")
#let f = math($x^4 - 5 x^2 + 4$)

$
  f(x) &= #to-typst(f) \
       &= #to-typst(factor(f)) quad "factored" \
  f'(x) &= #to-typst(derivative(f, x))
$
```

The factorization and derivative are computed exactly while Typst compiles the
document. Symbolica expressions are opaque values; render them with `to-typst`
or inspect them with `canonical`.

## Install locally

To use this repository checkout, clone it and expose its root as a local
package. On Linux:

```sh
git clone https://github.com/lcnbr/tymbolica.git
cd tymbolica
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica"
ln -s "$PWD" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica/0.1.0"
```

On macOS, use `~/Library/Application Support/typst/packages` in place of the
Linux data directory. During repository development, examples instead import
`../lib.typ` directly.

## Documentation and examples

- [User manual](typst/manual.pdf) — quickstart, concepts, recipes, limitations,
  and complete API reference
- [Minimal example](typst/examples/basic.typ) — a compact first document
- [Polynomial-system showcase](typst/examples/showcase.typ) — exact solving,
  factorization, substitution, and a Jacobian determinant in one case study
- [Changelog](CHANGELOG.md) — user-visible changes and compatibility notes

## Development

The pinned Nix flake supplies Rust, Binaryen, and Typst:

```sh
nix develop
```

Use the repository apps for the normal release workflow:

```sh
nix run .#build    # rebuild typst/tymbolica.wasm
nix run .#manual   # rebuild typst/tymbolica.wasm and typst/manual.pdf
nix run .#check    # rebuild, compile the public examples, and verify the PDF
nix flake check    # validate the Typst distribution using the tracked bundle
```

Maintainer checks also compile
[the API surface](typst/examples/api-surface.typ) and verify the
[`@local` package import](typst/examples/local-package.typ).

`nix run .#check` verifies the documented `@local` installation layout and
fails when `typst/manual.pdf` does not match `manual.typ` and the current
WebAssembly bundle. Commit the source, bundle, and regenerated manual together.

## Attribution and licensing

Tymbolica's original source code is available under the [MIT License](LICENSE).
Tymbolica is an interface to the
[Symbolica computer algebra system](https://symbolica.io/) and follows its
upstream `dev` branch. The generated `typst/tymbolica.wasm` is bundled here with
redistribution permission. The MIT License does not relicense Symbolica or that
artifact: Symbolica's own terms apply, so read the
[Symbolica license](https://symbolica.io/license/) before redistributing or
deploying the bundle.

Tymbolica would not exist without [Symbolica](https://symbolica.io/). Thank you
to its contributors for the algebra engine at the heart of this package.
Thanks also to [Parsely](https://typst.app/universe/package/parsely/) for making
native Typst-math parsing possible, and to
[Tidy](https://typst.app/universe/package/tidy/) for the documentation tools.
