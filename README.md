# Symbolica

The official [Symbolica](https://symbolica.io/) plugin for Typst, powered by
Symbolica 3.0. It is free to use for any use within Typst, including academic
and commercial work.

Symbolica lets you do symbolic computations and numerical evaluations directly in your Typst document. This avoids error-prone copy-pasting and keeps the displayed results in sync when you
change an equation or parameter.

You can currently:

- expand, factor, collect, differentiate, and inspect expressions;
- combine, cancel, or decompose rational functions;
- calculate derivatives and series;
- integrate expressions and see the integration steps;
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

Import the published package to use Symbolica in a Typst document:

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

The public names work with wildcard imports while leaving Typst's native
`math`, `symbol`, `function`, `content`, and math layout functions available.

### Parse Typst formulas or strings

`parse` accepts both Typst math content and strings in Symbolica syntax:

```typst
#import "@preview/symbolica:0.1.0": *

#let from-math = parse($x^2 / (1 + x)$)
#let from-string = parse("x^2/(1+x)")
#assert.eq(canonical(from-math), canonical(from-string))

#let expression = parse("2*x + y", namespace: "model")
#let x = literal("x", namespace: "model")
$ #to-typst(derivative(expression, x)) $
```

Strings use Symbolica's expression syntax, including explicit `*` for
multiplication. Typst formulas use Parsely and retain their decorated notation.
`parse` also accepts numbers and existing expressions: integers remain exact,
floats retain their value, and existing Atom bytes pass through unchanged.
Use `literal("x")` to construct a symbol explicitly. In algebra function
arguments, strings still represent single leaves; parse a string first when
it contains a whole expression.

### Use a formula as one literal symbol

`literal` accepts a symbol name or supported Typst content. Content supplies the
label for one opaque symbol: `literal($x+1$)` does not become an algebraic sum.
Composite labels are grouped when rendered by either `to-typst` or
`to-typst-source`.

```typst
#import "@preview/symbolica:0.1.0": *

#let label = literal($x + 1$)
$ #to-typst(pow(label, 2)) $

#let first = literal($a_0$, name: "first_coefficient", namespace: "model")
#let second = literal($a_0$, name: "second_coefficient", namespace: "model")
#assert.ne(canonical(first, namespaces: true), canonical(second, namespaces: true))
```

Without `name`, repeated labels have stable identities within their namespace.
Simple `literal($x$)` agrees with `literal("x")`, and `literal($a_0$)` agrees
with `parse($a_0$)`. The optional `name` applies only to content and lets two
symbols share a label while retaining distinct identities. `namespace` and
`tags` work with both input forms. Names must not end with an underscore.

Labels support portable math structures and text. Unsupported styling,
context-dependent content, and layout elements are rejected; use `notation`
for custom presentation around an explicitly named symbol. Plain source output
preserves the label's appearance, while `to-typst` also carries its exact identity.

Use `wild("a")` for a pattern placeholder. Its base name must be nonempty and
must not end with an underscore; `level` is a positive integer. `wild` returns
an Atom payload for matching, and `level: 0` is rejected.

### Numerical evaluation with π

Evaluate `π² + sin(π/4)` using Symbolica's built-in value of π:

```typst
#import "@preview/symbolica:0.1.0": *

#let expression = parse($pi^2 + sin(pi / 4)$)
#let value = evaluate(expression)

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
#import "@preview/symbolica:0.1.0": *

#let solutions = solve(
  ($x + y - a$, $x - y - b$).map(parse),
  ($x$, $y$).map(parse),
  domain: "real",
)

#let (x, y) = solutions.branches.first().values.map(to-typst)
$ x = #x, quad y = #y $
```

$$
x = \frac{a + b}{2}, \qquad y = \frac{a - b}{2}
$$


### Symbolic integration

```typst
#import "@preview/symbolica:0.1.0": *

#let x = parse($x$)
#let f = parse($x / (x + 1)$)
#to-typst(integrate(f, x))
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

`parse` preserves subscripts and corner attachments as decorated symbols.
For example, `$a_0-a_1$` keeps two distinct variables, `$C_(i j)$` retains the
order of its labels, and `$h'(c)$` is a call with a decorated function head.
All six Typst attachment positions (`t`, `b`, `tl`, `tr`, `bl`, `br`) are
supported, including nested labels and primes. In ordinary expressions, `t`
keeps its algebraic meaning: `$a_i^2$` is the square of `$a_i$`.

Decorations are literal notation: primes do not request differentiation, and
substitution does not traverse labels stored in symbol data. Use `derivative`
for differentiation and function arguments for indices that participate in
algebra. Rendered content retains exact Atom metadata; plain Typst source
preserves notation but does not carry namespaces or other hidden metadata.

The shared `math_display::MathDisplay` type stores the ordered display tree in
versioned Symbolica symbol data and exports a `symbolica.math-display`
attachment for other renderers. Its deterministic symbol names distinguish
labels without depending on a runtime's symbol IDs.

Several Typst packages are in development that make use of the symbolic payload, for example
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
Linux data directory. For that local installation, replace `@preview` with
`@local` in your document import. The repository checks make the checkout
available under both namespaces, so public examples run unchanged.

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
can then use the usual `#import "symbolica/lib.typ": *` and `integrate`
API. [Typst accepts raw bytes as a plugin source](https://typst.app/docs/reference/foundations/plugin/).
This joins the exact original engine in memory, without decompression. It
reduces individual upload sizes, but not total project storage or runtime memory.
The split loader was verified with the CLI; web upload acceptance depends on
the account's file and project limits.

Once published on Universe, use `#import "@preview/symbolica:0.1.0": *`
instead; the package is fetched without manually uploading its Wasm.

## Development shell

The repository's Nix flake provides Typst and its pinned package dependencies,
plus the Rust and Wasm build tools:

```sh
nix develop
typst compile --root . symbolica/tests/parsely-metadata.typ /tmp/parsely-metadata.pdf
```

Use `nix run .#typst -- <arguments>` to run Typst without entering the shell.
Run `nix flake check` to validate the bundled plugin, examples, and manual.

## Logo

Use `logo()` to place the Symbolica logo inline. It follows the text size by
default; use `size` to set its width and height.

```typst
#import "@preview/symbolica:0.1.0": logo

Calculated with #logo() Symbolica.

#logo(size: 2em)
```

## Documentation and examples

- [User manual](symbolica/manual.pdf) — quickstart, concepts, recipes, symbolic
  integration with nested Rubi steps, limitations, and complete API reference
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
