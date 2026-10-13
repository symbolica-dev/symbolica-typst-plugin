<!-- readme-artwork: banner -->
![Symbolica — Computer algebra in your document, with algebraic curves and optimization contours](symbolica/readme/banner.svg)

The official [Symbolica](https://symbolica.io/) plugin for Typst, powered by
Symbolica 3.0. Calculate directly from your Typst formulas and typeset the
results in the same document. Change an equation or parameter and the results
update when you recompile.

**Free for all use within Typst, including academic and commercial work.
No license key required.**

[User manual](symbolica/manual.pdf) · [Minimal example](symbolica/examples/basic.typ) · [More examples](#examples)

## Quick start

Write a polynomial in native Typst math, then factor and differentiate it:

<!-- readme-example: factorization -->
```typst
#import "@preview/symbolica:0.1.0": *

#let x = literal("x")
#let f = parse($x^4 - 5 x^2 + 4$)

$
  f(x) &= #to-typst(f) \
       &= #to-typst(factor(f)) \
  f'(x) &= #to-typst(derivative(f, x))
$
```

![Exact factorization of x⁴ − 5x² + 4 and its derivative](symbolica/readme/factorization.svg)

The results are computed exactly as Typst compiles your document. `parse`
reads the formula, `factor` and `derivative` perform the calculations, and
`to-typst` turns each result back into typeset mathematics.

The same import works in the Typst web app and with the command-line compiler;
Typst fetches the package automatically.

## See what you can build

<!-- readme-artwork: integration-steps -->
<!-- readme-artwork: exact-solving -->
<!-- readme-artwork: optimization -->

| Integration with steps | Exact system solving | Optimization valley |
| --- | --- | --- |
| [![A computed antiderivative of x/(x+1), with the complete rule trace in the linked example](symbolica/readme/integration-steps.svg)](symbolica/examples/integration.typ) | [![Four exact intersections of a circle and a hyperbola](symbolica/readme/exact-solving.svg)](symbolica/examples/showcase.typ) | [![Optimization steps through Rosenbrock contours toward the minimum at (1,1)](symbolica/readme/optimization.svg)](symbolica/examples/optimization.typ) |
| Compute an antiderivative and inspect its steps. | Solve, substitute, and check the Jacobian. | Derive gradients and trace optimization steps. |

Click a preview to open the complete Typst example. Symbolica computes the
values; Typst handles the layout and graphics.

The first integration call initializes its rule sets, which may take about
10 seconds. Later calls and ordinary edits reuse the cached rules.

## What can it do?

| Task | Operations |
| --- | --- |
| Exact algebra | Expand, factor, collect, and simplify rational functions |
| Calculus | Differentiate, calculate series, and integrate with steps |
| Equations and matrices | Solve systems exactly or numerically; compute exact matrix results |
| Numerical evaluation | Evaluate real and complex formulas over points or grids |
| Patterns and notation | Replace expressions with wildcards; preserve decorated Typst symbols |

Start with native Typst formulas or strings in Symbolica syntax, such as
`parse("x^2/(1+x)")`. The [manual](symbolica/manual.pdf) explains parsing,
literal labels, namespaces, custom notation, and the complete API.

## Examples

- [Minimal document](symbolica/examples/basic.typ) — your first symbolic calculation
- [Integration with Rubi steps](symbolica/examples/integration.typ) — an antiderivative and its nested rule trace
- [Circle–hyperbola intersections](symbolica/examples/showcase.typ) — exact solving, factorization, substitution, and a Jacobian determinant
- [Batched expression grid](symbolica/examples/expression-grid.typ) — four formulas over a two-dimensional parameter grid
- [Optimization valley](symbolica/examples/optimization.typ) — symbolic derivatives, an exact minimum, and a damped Newton loop written in Typst
- [Lotka–Volterra trajectory](symbolica/examples/lotka-volterra.typ) — batched evaluation inside a Runge–Kutta loop written in Typst
- [Complex phase portrait](symbolica/examples/phase-portrait.typ) — thousands of complex evaluations in one batch

See the [user manual](symbolica/manual.pdf) for concepts, recipes, limitations,
and the full reference, or the [changelog](CHANGELOG.md) for compatibility notes.
The [repository guide](https://github.com/symbolica-dev/symbolica-typst-plugin#reuse-the-rust-library)
covers reuse of the Rust library and symbolic payloads in other plugins.

## Attribution and licensing

The official `symbolica` Typst plugin is
free to use for any use within Typst, including academic and commercial work.
No Symbolica license or license key is needed for use within Typst.

The [Symbolica Typst permission](LICENSE-SYMBOLICA-TYPST.md) explicitly grants
free runtime use for all purposes within Typst, including commercial, server,
and hosted use. No payment, registration, activation, license key, or separate
runtime agreement is required. It also permits redistribution of the plugin
and rebuilding unmodified Symbolica as part of it. It grants no additional
rights to modify Symbolica itself or distribute modified Symbolica source.

The original Typst interface and Rust adapter code are under [MIT](LICENSE).
The `license = "MIT"` field in `typst.toml` describes that original plugin code.

**The bundled `symbolica/symbolica.wasm` is built from components under multiple
licenses and is not covered solely by MIT.** Symbolica's components are covered
by its [source-available license](LICENSE-SYMBOLICA.md), with the
[Symbolica Typst permission](LICENSE-SYMBOLICA-TYPST.md) taking precedence for
the uses it grants. Use outside Typst retains the otherwise applicable
Symbolica terms. Other dependencies retain their respective licenses,
including LGPL and MPL terms where applicable.

See the [third-party notices](THIRD_PARTY.md),
[complete dependency license texts](THIRD_PARTY_LICENSES.txt), and
[source locations and rebuilding instructions](REBUILDING.md). The MIT
declaration does not relicense any bundled dependency.

Thanks also to [Parsely](https://typst.app/universe/package/parsely/) for making
native Typst-math parsing possible, and to
[Tidy](https://typst.app/universe/package/tidy/) for the documentation tools.
The batched-evaluation, predator–prey, and phase-portrait examples were inspired
by TimeTravelPenguin's
[`symbolic-eval`](https://github.com/TimeTravelPenguin/symbolic-eval) package and
independently adapted to Symbolica's API. Their pinned sources and upstream
license declaration are recorded in the [third-party notices](THIRD_PARTY.md).
