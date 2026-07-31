# Tymbolica

Exact symbolic computation inside Typst, powered by
[Symbolica](https://symbolica.io/).

Tymbolica parses native Typst mathematics, sends exact expressions through a
bundled WebAssembly plugin, and returns results that can be typeset directly in
the same document. It currently supports:

- expression construction, expansion, factorization, and differentiation;
- polynomial integration and univariate series;
- wildcard-based structural rewriting;
- exact and numerical equation-system solving;
- batched real or complex evaluation; and
- exact matrix construction, reduction, inversion, and solving.

Start with the [user manual](typst/manual.pdf), the
[minimal example](typst/examples/basic.typ), or the
[polynomial-system showcase](typst/examples/showcase.typ). The manual contains
the conceptual guide, task-oriented examples, limitations, and generated API
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
- [API surface check](typst/examples/api-surface.typ) — broad executable API
  coverage for maintainers
- [Local-package smoke test](typst/examples/local-package.typ) — verifies the
  root manifest and `@local` import path
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
