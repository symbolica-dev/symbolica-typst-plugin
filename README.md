# Tymbolica

Exact symbolic computation inside Typst, powered by
[Symbolica](https://symbolica.io/).

Tymbolica lets the formulas on a Typst page take part in the calculation. Write
an expression as ordinary Typst mathematics, work with it symbolically, and
place the result back into the same document. You can currently:

- expand, factor, collect, differentiate, and inspect expressions;
- combine, cancel, or decompose rational functions;
- calculate derivatives and series;
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

## Rubi integration and Tydenso

Symbolic integration is provided by the separate `tymbolica-rubi` package.
It exposes only `integrate` and `integrate-with-steps`; expressions move between
it and Tymbolica through the shared Atom payload:

```typst
#import "@local/tymbolica:0.1.0" as sym
#import "@local/tymbolica-rubi:0.1.0": integrate

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#sym.to-typst(integrate(f, x))
```

Rubi's rule tables are prepared with Wizer when the plugin is built. Its
compressed engine is independent of Tymbolica's smaller algebra engine.
Both Rubi arguments are portable Atom payload bytes; `math($x$)` creates the
single-symbol payload required for the integration variable.

Tensor algebra is provided by the separate `tydenso` Typst package in this
repository. Tydenso has its own manual, constructors, Typst-side Spenso
notation, compact string printer, and compressed plugin. Representations and slots are inspectable Typst
dictionaries. Symbols and tensor calls look like ordinary math and carry the
same exact Symbolica Atom export understood by both plugins:

```typst
#import "@local/tydenso:0.1.0": *

#let V = mink(4)
#let mu = slot(V, 1) // mu
#let nu = slot(V, 2) // nu
#let p = vector("p")
#let expression = math($#metric(V, mu, nu) #p(nu)$)

#to-typst(simplify-metrics(expression))
```

Built-in representations give integer slots conventional names and cycle with
subscripts; explicitly named slots remain available when a calculation needs a
different convention.

Tymbolica and Tydenso share one Parsely-to-Atom bridge and one Atom payload.
Namespaces and Symbolica attributes remain exact, while Tydenso carries the
representation declarations it needs across plugin calls instead of rebuilding
them from printed indices. Those declarations are opaque to Tymbolica and
Rubi: neither engine links Spenso or Idenso; only Tydenso interprets them.

## Install locally

To use this repository checkout, clone it and expose its root as a local
package. On Linux:

```sh
git clone https://github.com/lcnbr/tymbolica.git
cd tymbolica
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica"
ln -s "$PWD" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica/0.1.0"
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tydenso"
ln -s "$PWD/tydenso" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tydenso/0.1.0"
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica-rubi"
ln -s "$PWD/rubi" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/tymbolica-rubi/0.1.0"
```

On macOS, use `~/Library/Application Support/typst/packages` in place of the
Linux data directory. During repository development, examples instead import
`../lib.typ` directly.

## Documentation and examples

- [User manual](typst/manual.pdf) — quickstart, concepts, recipes, limitations,
  and complete API reference
- [Tydenso manual](tydenso/manual.pdf) — tensor construction, Spenso printing,
  CBOR inspection, Idenso transforms, and complete API reference
- [Rubi manual](rubi/manual.pdf) — symbolic integration and nested Rubi steps
- [Minimal example](typst/examples/basic.typ) — a compact first document
- [Rubi integration](rubi/examples/basic.typ) — an antiderivative and
  its nested rule steps
- [Tydenso tensor algebra](tydenso/examples/basic.typ) — structural tensor
  construction and metric contraction
- [Tydenso interoperability](tydenso/examples/interop.typ) — native Atom
  exchange between the two independently packaged plugins
- [Tydenso index palettes](tydenso/examples/index-palettes.typ) — automatic
  names, wraparound, dual orientations, and manual labels
- [Polynomial-system showcase](typst/examples/showcase.typ) — exact solving,
  factorization, substitution, and a Jacobian determinant in one case study
- [Batched expression grid](typst/examples/expression-grid.typ) — evaluate four
  formulas together over a two-dimensional parameter grid
- [Lotka–Volterra trajectory](typst/examples/lotka-volterra.typ) — evaluate
  both right-hand sides together inside a local Runge–Kutta loop
- [Complex phase portrait](typst/examples/phase-portrait.typ) — evaluate a
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
nix run .#build-engine # rebuild only the compressed Tymbolica core engine
nix run .#build-rubi   # rebuild the Wizer-preinitialized Rubi engine
nix run .#build-tydenso # rebuild only the compressed Tydenso engine
nix run .#manual       # rebuild all engines and manuals
nix run .#check       # rebuild, compile the public examples, and verify the PDF
nix flake check       # validate the Typst distribution using tracked plugins
```

Maintainer checks also compile
[the API surface](typst/examples/api-surface.typ) and verify the
[`@local` package import](typst/examples/local-package.typ).

`nix run .#check` verifies all documented `@local` installation layouts and
fails when any committed manual PDF is stale. Commit the source, bundles, and
regenerated manuals together.

## Attribution and licensing

Tymbolica's original source code is available under the [MIT License](LICENSE).
Tymbolica is an interface to the
[Symbolica computer algebra system](https://symbolica.io/) and follows its
upstream development. The generated WebAssembly bundles are
included here with redistribution permission. The MIT License does not
relicense Symbolica or those artifacts: Symbolica's own terms apply, so read the
[Symbolica license](https://symbolica.io/license/) before redistributing or
deploying the plugins.

Tymbolica would not exist without [Symbolica](https://symbolica.io/). Thank you
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
independently adapted to Tymbolica's API. Their pinned sources and upstream
license declaration are recorded in the [third-party notices](THIRD_PARTY.md).
