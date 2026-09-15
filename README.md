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
- integrate expressions and inspect nested Rubi rule steps;
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

### Exact factorization and differentiation

After installing the local package below, this is a complete Typst document:

```typst
#import "@local/symbolica:0.1.0": *

#let x = symbol("x")
#let f = math($x^4 - 5 x^2 + 4$)

$
  f(x) &= #to-typst(f) \
       &= #to-typst(factor(f)) \
  f'(x) &= #to-typst(derivative(f, x))
$
```

$$
\begin{aligned}
f(x) &= x^4 - 5x^2 + 4 \\
     &= (x - 2)(x - 1)(x + 1)(x + 2) \\
f'(x) &= 4x^3 - 10x
\end{aligned}
$$

The factorization and derivative are computed exactly while Typst compiles the document.
Symbolica expressions are opaque values; render them with `to-typst` or inspect
them with `canonical`.

### Numerical evaluation with π

Evaluate `π² + sin(π/4)` using Symbolica's built-in value of π:

```typst
#import "@local/symbolica:0.1.0" as sym

#let expression = sym.math($pi^2 + sin(pi / 4)$)
#let value = sym.evaluate(expression)

$ pi^2 + sin(pi / 4) approx #calc.round(value.re, digits: 8) $
```

$$
\pi^2 + \sin\left(\frac{\pi}{4}\right) \approx 10.57671118
$$

`evaluate` returns a dictionary
with real and imaginary parts, `re` and `im`; here `im` is zero.

### Solve a system with parameters

Solve `x + y = a` and `x - y = b` for `x` and `y`, keeping `a` and `b` symbolic:

```typst
#import "@local/symbolica:0.1.0" as sym

#let solutions = sym.solve(
  ($x + y - a$, $x - y - b$).map(sym.math),
  ($x$, $y$).map(sym.math),
  domain: "real",
)

#let (x, y) = solutions.branches.first().values.map(sym.to-typst)
$ x = #x, quad y = #y $
```

$$
x = \frac{a + b}{2}, \qquad y = \frac{a - b}{2}
$$


### Symbolic integration

```typst
#import "@local/symbolica:0.1.0" as sym

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#sym.to-typst(sym.integrate(f, x))
```

$$
x - \log(x + 1)
$$

The first call to `integrate` or `integrate-with-steps` compiles and initializes
the integration rule sets, which may take about 10 seconds. Both functions share
the cached rules, so later calls and ordinary edits reuse them.


## Symbolic payloads

The `symbolica-typst-atom-payload` crate is the reusable boundary for extensions. It
combines Symbolica's exact native Atom export with schema-keyed portable
attachments and a generic render tree.

Several Typst packages are in development than make use of the symbolic payload, for example
the tensor algebra package [spenso](https://github.com/alphal00p/gammaloop).
## Install locally

To use this repository checkout, clone it and expose its root as a local
package. On Linux:

```sh
git clone https://github.com/symbolica-dev/symbolica-typst-plugin.git
cd symbolica-typst-plugin
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica"
ln -s "$PWD" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica/0.1.0"
```

On macOS, use `~/Library/Application Support/typst/packages` in place of the
Linux data directory. During repository development, examples instead import
`../lib.typ` directly.

## Use in the Typst web app

Before publication on Universe, upload the library files into your project
and import `"symbolica/lib.typ"`. A local installation on your computer is
not available to the web app.

If the 23.36 MiB Wasm exceeds the web app's per-file upload limit, split it
into smaller parts. For this build, the following GNU `split` command creates
three files of at most 8 MiB:

```sh
split -b 8M -d -a 1 symbolica/symbolica.wasm symbolica/symbolica.wasm.part
```

In the **uploaded copy** of `symbolica/lib.typ`, replace `_bundled_plugin`
with:

```typst
#let _bundled_plugin() = plugin(
  read("symbolica.wasm.part0", encoding: none) +
  read("symbolica.wasm.part1", encoding: none) +
  read("symbolica.wasm.part2", encoding: none)
)
```

Upload that `lib.typ`, `render.typ`, and the three parts into a `symbolica`
folder in the project. Omit the original large `symbolica.wasm`. Your document
can then use the usual `#import "symbolica/lib.typ" as sym` and `sym.integrate`
API. [Typst accepts raw bytes as a plugin source](https://typst.app/docs/reference/foundations/plugin/).
This joins the exact original engine in memory, without decompression. It
reduces individual upload sizes, but not total project storage or runtime memory.
The split loader was verified with the CLI; web upload acceptance depends on
the account's file and project limits.

Once published on Universe, use `#import "@preview/symbolica:0.1.0" as sym`
instead; the package is fetched without manually uploading its Wasm.

## Documentation and examples

- [User manual](symbolica/manual.pdf) — quickstart, concepts, recipes, limitations,
  and complete API reference
- [Rubi manual](symbolica/integration-manual.pdf) — symbolic integration and nested Rubi steps
- [Minimal example](symbolica/examples/basic.typ) — a compact first document
- [Rubi integration](symbolica/examples/integration.typ) — an antiderivative and
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
nix run .#build       # rebuild the joint Symbolica Wasm engine
nix run .#package     # build dist/symbolica-0.1.0.tar.gz
nix run .#manual      # rebuild the engine and both manuals
nix run .#check       # rebuild, compile the public examples, and verify the PDF
nix flake check       # validate the Typst distribution using tracked plugins
```

Maintainer checks also compile the non-user-facing regression fixtures under
[`symbolica/tests`](symbolica/tests) and verify the
[`@local` package import](symbolica/examples/local-package.typ).

`nix run .#check` verifies all documented `@local` installation layouts and
fails when any committed manual PDF is stale. Commit the source, bundles, and
regenerated manuals together.

See the [Wasm size investigation](docs/wasm-size.md) for measured size reductions
and the tradeoffs between compressed, direct, and preinitialized engines.

## Attribution and licensing

The official `symbolica` Typst plugin is
free to use for any use within Typst, including academic and commercial work.
No Symbolica license or license key is needed for use within Typst.

This repository's original plugin source code is available under the
[MIT License](LICENSE). The bundled WebAssembly engine is included with
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
