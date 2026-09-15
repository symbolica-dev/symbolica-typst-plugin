# Symbolica

The official [Symbolica](https://symbolica.io/) plugin for Typst, powered by
Symbolica 3.0. It is free to use for any use within Typst, including academic
and commercial work. No license or license key is needed.

Symbolica lets the formulas on a Typst page take part in the calculation. Write
an expression as ordinary Typst mathematics, work with it symbolically, and
place the result back into the same document. You can currently:

- expand, factor, collect, differentiate, and inspect expressions;
- combine, cancel, or decompose rational functions;
- calculate derivatives and series;
- replace recurring patterns with wildcards;
- solve systems exactly or numerically;
- evaluate formulas over points or grids; and
- solve exact matrix problems.

Start with the [user manual](symbolica/manual.pdf), the
[minimal example](symbolica/examples/basic.typ), or the
[polynomial-system showcase](symbolica/examples/showcase.typ). The manual contains
the conceptual guide, task-oriented examples, limitations, and complete API
reference.

## Quick start

After installing the local package below, this is a complete Typst document:

```typst
#import "@local/symbolica:0.1.0": *

#let x = symbol("x")
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

## Rubi integration

Symbolic integration is provided by the separate `symbolica-integrate` package.
It exposes only `integrate` and `integrate-with-steps`; expressions move between
it and Symbolica through the shared Atom payload:

```typst
#import "@local/symbolica:0.1.0" as sym
#import "@local/symbolica-integrate:0.1.0": integrate

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#sym.to-typst(integrate(f, x))
```

Rubi's rule tables are prepared on first use through Typst's cached plugin
transition. Subsequent calls and ordinary edits reuse the initialized module;
restarting the compiler or losing its cache requires initialization again.
Its compressed engine is independent of Symbolica's smaller algebra engine.
Both Rubi arguments are portable Atom payload bytes; `math($x$)` creates the
single-symbol payload required for the integration variable.

The `symbolica-typst-atom-payload` crate is the reusable boundary for extensions. It
combines Symbolica's exact native Atom export with schema-keyed portable
attachments and a generic render tree. Symbolica and Rubi preserve unknown
attachments without interpreting them and do not depend on Spenso, Idenso, or
GammaLoop.

Tensor algebra and the Tydenso Typst package are maintained with
[GammaLoop](https://github.com/alphal00p/gammaloop). GammaLoop consumes the
shared payload crate as a pinned Git dependency, so tensor-specific metadata
and rendering can evolve beside Spenso and Spynso without coupling those
projects back into Symbolica.

## Install locally

To use this repository checkout, clone it and expose its root as a local
package. On Linux:

```sh
git clone https://github.com/symbolica-dev/symbolica-typst-plugin.git
cd symbolica-typst-plugin
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica"
ln -s "$PWD" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica/0.1.0"
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica-integrate"
ln -s "$PWD/symbolica-integrate" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica-integrate/0.1.0"
```

On macOS, use `~/Library/Application Support/typst/packages` in place of the
Linux data directory. During repository development, examples instead import
`../lib.typ` directly.

## Documentation and examples

- [User manual](symbolica/manual.pdf) — quickstart, concepts, recipes, limitations,
  and complete API reference
- [Rubi manual](symbolica-integrate/manual.pdf) — symbolic integration and nested Rubi steps
- [Minimal example](symbolica/examples/basic.typ) — a compact first document
- [Rubi integration](symbolica-integrate/examples/basic.typ) — an antiderivative and
  its nested rule steps
- [Polynomial-system showcase](symbolica/examples/showcase.typ) — exact solving,
  factorization, substitution, and a Jacobian determinant in one case study
- [Batched expression grid](symbolica/examples/expression-grid.typ) — evaluate four
  formulas together over a two-dimensional parameter grid
- [Lotka–Volterra trajectory](symbolica/examples/lotka-volterra.typ) — evaluate
  both right-hand sides together inside a local Runge–Kutta loop
- [Complex phase portrait](symbolica/examples/phase-portrait.typ) — evaluate a
  rational function over thousands of complex points in one batch
- [Changelog](CHANGELOG.md) — user-visible changes and compatibility notes

## Development

The pinned Nix flake supplies Rust, Binaryen, and Typst:

```sh
nix develop
```

Use the repository apps for the normal release workflow:

```sh
nix run .#build        # rebuild all compressed engines and their loaders
nix run .#build-engine # rebuild only the compressed Symbolica core engine
nix run .#build-rubi   # rebuild the Rubi engine with cached runtime initialization
nix run .#manual       # rebuild all engines and manuals
nix run .#check       # rebuild, compile the public examples, and verify the PDF
nix flake check       # validate the Typst distribution using tracked plugins
```

Maintainer checks also compile the non-user-facing regression fixtures under
[`symbolica/tests`](symbolica/tests) and [`symbolica-integrate/tests`](symbolica-integrate/tests), and verify the
[`@local` package import](symbolica/examples/local-package.typ).

`nix run .#check` verifies all documented `@local` installation layouts and
fails when any committed manual PDF is stale. Commit the source, bundles, and
regenerated manuals together.

See the [Wasm size investigation](docs/wasm-size.md) for measured size reductions
and the tradeoffs between compressed, direct, and preinitialized engines.

## Attribution and licensing

The official `symbolica` plugin and its `symbolica-integrate` companion are
free to use for any use within Typst, including academic and commercial work.
No Symbolica license or license key is needed for use within Typst.

This repository's original plugin source code is available under the
[MIT License](LICENSE). The bundled WebAssembly engines are included with
redistribution permission. The MIT License covers the plugin source, while
the [Symbolica license](https://symbolica.io/license/) governs the underlying
computer algebra system outside this Typst usage permission.

This plugin is powered by [Symbolica](https://symbolica.io/). Thank you
to its contributors for the algebra engine at the heart of this package.
Integration is provided by the MIT-licensed
[`symbolica-integrate`](https://github.com/symbolica-dev/symbolica-integrate)
port of the Rubi rule collection; thanks to both projects and their
contributors.
Thanks also to [Parsely](https://typst.app/universe/package/parsely/) for making
native Typst-math parsing possible, and to
[Tidy](https://typst.app/universe/package/tidy/) for the documentation tools.
The batched-evaluation, predator–prey, and phase-portrait examples were inspired
by TimeTravelPenguin's
[`symbolic-eval`](https://github.com/TimeTravelPenguin/symbolic-eval) package and
independently adapted to Symbolica's API. Their pinned sources and upstream
license declaration are recorded in the [third-party notices](THIRD_PARTY.md).
