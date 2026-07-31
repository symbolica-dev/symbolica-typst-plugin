#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica

#let manifest = toml("../typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/lcnbr/tymbolica"
#let symbolica-guide = "https://symbolica.io/docs/quick_start.html"
#let accent = rgb("#315c88")
#let pale-accent = rgb("#edf4fb")
#let warning = rgb("#9a5b13")
#let pale-warning = rgb("#fff6e8")
#let muted = rgb("#5f6873")

#set document(
  title: "Tymbolica Manual",
  author: "Tymbolica contributors",
)
#set page(
  paper: "a4",
  margin: (x: 20mm, top: 19mm, bottom: 18mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(size: 8pt, fill: muted)
      grid(
        columns: (1fr, auto),
        align: (left, right),
        [Tymbolica],
        [Version #package-version],
      )
      line(length: 100%, stroke: 0.35pt + rgb("#ccd3da"))
    }
  },
  footer: context {
    if counter(page).get().first() > 1 {
      align(center, text(size: 8pt, fill: muted, counter(page).display("1")))
    }
  },
)
#set text(font: "Libertinus Serif", size: 10.5pt)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.1.1")
#show heading: set text(font: "Libertinus Serif", fill: accent)
#show heading.where(level: 4): set heading(numbering: none)
#show heading.where(level: 5): set heading(numbering: none)
#show link: set text(fill: accent)
#show raw: set text(font: "DejaVu Sans Mono")

#let callout(title, body, kind: "note") = {
  let color = if kind == "warning" { warning } else { accent }
  let fill = if kind == "warning" { pale-warning } else { pale-accent }
  block(
    width: 100%,
    breakable: true,
    inset: 9pt,
    radius: 3pt,
    fill: fill,
    stroke: (left: 2.2pt + color),
  )[
    #text(font: "Libertinus Serif", weight: "bold", fill: color)[#title]
    #body
  ]
}

#let example-preamble = "#import symbolica: *\n"
#let docs = tidy.parse-module(
  read("lib.typ"),
  name: "symbolica",
  scope: (symbolica: symbolica),
  preamble: example-preamble,
)
#let worked-example(code) = tidy.styles.default.show-example(
  raw(code.text, lang: "typ", block: true),
  scope: (symbolica: symbolica),
  preamble: example-preamble,
  mode: "markup",
  dir: ttb,
  scale-preview: 100%,
)
#show raw.where(lang: "worked"): worked-example

#align(center)[
  #v(25mm)
  #text(font: "Libertinus Serif", size: 35pt, weight: "bold", fill: accent)[
    Tymbolica
  ]
  #v(5mm)
  #text(size: 16pt, fill: muted)[Exact symbolic computation inside Typst]
  #v(13mm)
  #text(size: 11pt)[User manual · version #package-version]
  #v(23mm)
  #block(
    width: 76%,
    inset: 14pt,
    radius: 5pt,
    fill: pale-accent,
  )[
    Parse Typst mathematics, transform it with Symbolica's exact algebra
    engine, and place the result directly back into a document.
  ]
  #v(34mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[MIT license] ·
  #link(symbolica-guide)[Symbolica guide] ·
  #link("https://symbolica.io/license/")[Symbolica license]
]

#pagebreak()

#text(size: 22pt, weight: "bold", fill: accent)[Contents]
#v(5mm)

#outline(title: none, depth: 2, indent: auto)

#pagebreak()

= Start here

Tymbolica is a focused Typst interface to a subset of
#link("https://symbolica.io/")[Symbolica]. It supports exact expression
construction, expansion and factorization, differentiation, polynomial
integration, series, structural rewriting, exact and numerical system solving,
batched evaluation, and rational-polynomial matrices. The algebra runs in a
bundled WebAssembly plugin; the surrounding API and all rendering stay in
Typst.

The shortest useful workflow is: parse an expression, transform it, then render
the opaque result with `to-typst`.

#let quickstart-source = (
  "<<<#import \"@local/tymbolica:" + package-version + "\": *\n\n"
  + "#let p = math($(x + y)^3 - (x^3 + y^3)$)\n"
  + "#to-typst(factor(expand(p)))"
)
#worked-example(raw(quickstart-source, lang: "worked", block: true))

The result is exact: the expanded polynomial is collected and factored as
$3 x y (x + y)$ without converting its coefficients to floating point.

== Installation

The repository currently provides a local package. On Linux, place or symlink
the repository root at:

#raw(
  "~/.local/share/typst/packages/local/tymbolica/" + package-version,
  lang: "text",
  block: true,
)

Use the corresponding Typst data directory on macOS or Windows. The package
root must contain `typst.toml`; its `typst` directory contains `lib.typ` and
`tymbolica.wasm`. Then import:

#raw(
  "#import \"@local/tymbolica:" + package-version + "\": *",
  lang: "typ",
  block: true,
)

From a source checkout, a document can instead import the library by relative
path:

```typst
#import "path/to/tymbolica/typst/lib.typ": *
```

The checked-in WebAssembly bundle is ready to use. Rebuild it after changing the
Rust bridge with `nix run .#build`. Build this manual with
`nix run .#manual`, and compile every checked-in example with
`nix run .#check`.

#callout(
  [Package status],
  [
    The `@local` namespace is intentional for version #package-version.
    Replace it with the published namespace if Tymbolica is released through
    Typst Universe.
  ],
)

== Choosing an operation

#table(
  columns: (1.25fr, 1.8fr, 2.25fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header(
    [*Goal*],
    [*Primary functions*],
    [*Result*],
  ),
  [Read Typst mathematics],
  [`math`, `var`, `atom`],
  [Opaque expression payload],
  [Display or export],
  [`to-typst`, `canonical`, `to-latex`],
  [Typst content or source text],
  [Exact algebra and calculus],
  [`expand`, `factor`, `derivative`, `integrate`, `series`],
  [Opaque expression payload],
  [Structural rewriting],
  [`wild`, `rule`, `replace`],
  [Transformed expression payload],
  [Evaluate many points],
  [`evaluate-many`, `evaluate-grid`],
  [Real/complex records and arrays],
  [Solve equations],
  [`solve-linear`, `solve-system`, `nsolve-system`],
  [Exact solution rows or one numerical row],
  [Exact linear algebra],
  [`matrix`, `matrix-solve`, `row-reduce`],
  [Matrix payloads or structured records],
)

= Core concepts

== Expressions are opaque payloads

`math`, `var`, arithmetic constructors, transformations, solvers, and matrix
operations return `bytes`. Those bytes are an internal Symbolica payload, not
Typst math content. Keep them opaque and pass them to other Tymbolica functions.
Render only at the document boundary:

#align(center)[
  `Typst math` $arrow.r$ `math` $arrow.r$ `bytes`
  $arrow.r$ `transform` $arrow.r$ `bytes`
  $arrow.r$ `to-typst` $arrow.r$ `content`
]

```worked
#let expression = math($(x + 1)^4$)
#let transformed = expand(expression)

payload type: #type(transformed)\
document output: #to-typst(transformed)\
Symbolica text: #raw(canonical(transformed))
```

Use `to-typst` for normal document output. `to-typst-source` returns Typst math
source, `to-latex` returns LaTeX source, and `canonical` returns Symbolica's
canonical text representation. `canonical(namespaces: true)` is useful for
debugging symbol identity; it is not a documented import format.

== `math`, `atom`, `var`, and `wild`

#table(
  columns: (0.8fr, 1.4fr, 2.8fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Function*], [*Typical input*], [*Use*]),
  [`math`], [`$x^2 + 1$`], [Parse Typst math through the bundled Parsely grammar.],
  [`atom`], [number, string, math, or bytes], [Convert a general value to an expression payload.],
  [`var`], [`"x"`], [Create an ordinary symbolic variable.],
  [`wild`], [`"a"`], [Create a pattern placeholder such as `a_`.],
)

#callout(
  [`var` versus `wild`],
  [
    A variable is part of the mathematics. A wildcard exists only to capture a
    subexpression while matching a replacement pattern. If the expression is
    $x^2 + 1$, construct $x$ with `var("x")`. Use `wild("a")` when a rule needs
    to match an arbitrary expression as `a_`.
  ],
  kind: "warning",
)

Repeated occurrences of the same wildcard in one pattern must capture the same
expression. Different matches can bind it differently; the rewriting guide
below turns that rule into a concrete example.

== Default functions and custom engines

For ordinary work, import and call the top-level functions. They share one
default engine whose namespace is `"typst"`.

Use `init` only when you need a different default namespace, a custom parsing
grammar, or a different plugin source. Built-in analytic functions such as
`sin`, `cos`, and `exp` need Symbolica's namespace for calculus and numerical
evaluation:

```typst
#let sym = init(namespace: "symbolica")
#let parse = sym.math
#let x = sym.var("x")
#let derivative = sym.derivative

#sym.to-typst(derivative(parse($sin(x)$), x))
```

Dictionary function fields must be bound first or called with parentheses:
`(sym.derivative)(...)`. The examples bind short local names to keep the
mathematics readable.

Symbols from different namespaces are distinct even when they print alike:

```worked
#let physics = init(namespace: "physics")
#let mixed = add(var("x"), (physics.var)("x"))

#raw(canonical(mixed, namespaces: true))
```

== Systems and return ordering

Solver inputs are expressions understood to equal zero. For example,
`math($x + y - 3$)` represents $x+y=3$.

Exact system solvers return arrays of expression payloads. Columns follow the
variable order supplied by the caller; the order of solution rows is not a
stable contract. Numerical solvers return one floating-point solution near the
initial guess.

= Worked mathematical guides

Each guide asks a mathematical question, shows complete copyable code, renders
the symbolic result, and performs a check. The examples are evaluated by Tidy
while this manual compiles.

== Calibrate a pendulum model

This workflow is adapted to Typst from Symbolica's
#link("https://github.com/symbolica-dev/symbolica#pendulum-calibration")[
  pendulum calibration example
]. Start from the potential
$V(theta)=kappa(1-cos theta)$, derive the restoring torque, approximate it for
small angles, and infer the unknown scale $kappa$ and sensor offset $tau_0$ from
two measurements.

```worked
#let sym = init(namespace: "symbolica")
#let m = sym.math
#let v = sym.var
#let render = sym.to-typst
#let derivative = sym.derivative
#let series = sym.series
#let replace = sym.replace
#let solve-linear = sym.solve-linear
#let evaluate-many = sym.evaluate-many
#let neg = sym.neg
#let add = sym.add
#let sub = sym.sub

// Parse before binding q and k, so Typst's math names stay content.
#let potential = m($kappa (1 - cos(theta))$)
#let q = v("θ")
#let k = v("κ")
#let b = v("τ₀")
#let q1 = v("θ₁")
#let q2 = v("θ₂")
#let t1 = v("τ₁")
#let t2 = v("τ₂")

#let torque = neg(derivative(potential, q))
#let small-angle = series(torque, q, 0, 3)
#let model = add(small-angle, b)

#let fit = solve-linear((
  sub(replace(model, q, q1), t1),
  sub(replace(model, q, q2), t2),
), (k, b))

#let fitted = evaluate-many(
  fit,
  (q1, q2, t1, t2),
  ((0.10, 0.20, -0.4697, -0.9545),),
).first()
#let predictions = evaluate-many(
  model,
  (q, k, b),
  (
    (0.10, fitted.at(0).re, fitted.at(1).re),
    (0.20, fitted.at(0).re, fitted.at(1).re),
  ),
)

$ V(theta) = #render(potential) $\
$ tau(theta) = -(partial V)/(partial theta) = #render(torque) $\
$ tau_"small"(theta) = #render(small-angle) $\
$ kappa = #render(fit.at(0)) $\
$ kappa approx #calc.round(fitted.at(0).re, digits: 6), quad
  tau_0 approx #calc.round(fitted.at(1).re, digits: 6) $\
predicted torques:
#predictions.map(row => str(calc.round(row.first().re, digits: 4))).join(", ")
```

The symbolic result for $kappa$ is retained exactly until the final batched
evaluation. The two predicted torques reproduce the measurements, which checks
the substitution and solve rather than merely displaying the fitted formula.

#callout(
  [Why the namespace matters],
  [
    In the default `"typst"` namespace, `cos` is an ordinary user function.
    The `"symbolica"` engine identifies it as Symbolica's analytic cosine, so
    differentiation, series expansion, and evaluation know its semantics.
  ],
)

== Rewrite a repeated identity with wildcards

Suppose a long expression contains several copies of
$sin^2(a)+cos^2(a)$. One wildcard must capture the same argument in both
functions. Repeating the rule then handles each independently matched argument.

```worked
#let source = math(
  $3 (sin(x)^2 + cos(x)^2)
    + sin(x + y)^2 + cos(x + y)^2$
)
#let identity = math(
  $sin("a_")^2 + cos("a_")^2$
)

#let one-pass = replace(source, identity, 1)
#let reduced = replace(source, identity, 1, repeat: true)

source: #to-typst(source)\
one pass: #to-typst(one-pass)\
repeat to a fixed point: #to-typst(reduced)
```

Within one match, both occurrences of `a_` capture the same expression. Across
matches it first captures one argument and then another. This is the practical
difference between an ordinary `var("a")` and `wild("a")`.

#callout(
  [Repeating rules],
  [
    `repeat: true` stops at a fixed point. Do not repeat mutually reversing or
    indefinitely growing rules: the current bridge has no public iteration
    limit for replacement loops.
  ],
  kind: "warning",
)

== Integrate a polynomial and verify it

Tymbolica currently integrates polynomials exactly. For a factored degree-five
input, request the additive contributions, assemble the antiderivative, and
differentiate the result to prove it.

```worked
#let x = var("x")
#let p = math($(x + 1)^3 (2 x^2 - 3 x + 5)$)
#let integration = integrate-with-steps(p, x)
#let residual = expand(
  sub(derivative(integration.result, x), p)
)

$ p(x) = #to-typst(p) $\
term contributions:\
#for (index, term) in integration.steps.enumerate() [
  #(index + 1). #to-typst(term) #linebreak()
]
$ integral p(x) dif x = #to-typst(integration.result) + C $\
verification: $ (partial I)/(partial x) - p(x) = #to-typst(residual) $
```

`integrate-with-steps` returns
`(result: bytes, steps: array)`. Here `steps` are the additive antiderivative
contributions produced after polynomial expansion; they are not Symbolica's
full pedagogical integration tree. The payload represents one antiderivative,
so the manual adds the conventional arbitrary constant $C$ when displaying an
indefinite integral.

#callout(
  [Integration boundary],
  [
    Denominators depending on the integration variable are currently rejected.
    In particular, Symbolica's richer examples such as
    $integral 1/(1+x^2) dif x$ are outside Tymbolica's present bridge.
  ],
  kind: "warning",
)

== Solve a nonlinear system exactly and numerically

Intersect the circle $x^2+y^2=25$ with the line $x-y=1$. Exact solving should
return every algebraic branch. Numerical solving should select a branch near
the supplied seed.

```worked
#let x = var("x")
#let y = var("y")
#let system = (
  math($x^2 + y^2 - 25$),
  math($x - y - 1$),
)

#let exact = solve-system(system, (x, y))
#let positive = nsolve-system(
  system, (x, y), (3.0, 3.0), prec: 1e-10,
)
#let negative = nsolve-system(
  system, (x, y), (-3.0, -3.0), prec: 1e-10,
)
#let checks = evaluate-many(
  system, (x, y), (positive, negative),
)
#let max-residual(row) = row.map(
  value => calc.abs(value.re),
).sorted().last()

exact branches:\
#for (index, row) in exact.enumerate() [
  #(index + 1). $x = #to-typst(row.at(0)), y = #to-typst(row.at(1))$
  #linebreak()
]
seed $(3,3)$ $arrow.r$
(#positive.map(value => str(calc.round(value, digits: 6))).join(", "));
maximum residual #repr(max-residual(checks.first()))\
seed $(-3,-3)$ $arrow.r$
(#negative.map(value => str(calc.round(value, digits: 6))).join(", "));
maximum residual #repr(max-residual(checks.last()))
```

Every input expression means “equal to zero.” Each exact row is ordered as
`(x, y)` because that is the requested variable order. Numerical results use
`f64`; small nonzero residuals near machine precision are expected. A different
seed may converge to another root or fail to converge.

== Recover an interpolating polynomial with matrices

Find the quadratic through $(0,1)$, $(1,3)$, and $(2,8)$. Writing
$p(t)=a_0+a_1 t+a_2 t^2$ turns interpolation into the exact matrix equation
$A a=b$.

```worked
#let t = var("t")
#let A = matrix((
  (1, 0, 0),
  (1, 1, 1),
  (1, 2, 4),
))
#let b = vec((1, 3, 8))
#let coefficients = matrix-solve(A, b)
#let check = matrix-mul(A, coefficients)
#let reduced = row-reduce(
  augment(A, b),
  max-col: 3,
)
#let polynomial = add(
  matrix-at(coefficients, 0, 0),
  mul(matrix-at(coefficients, 1, 0), t),
  mul(matrix-at(coefficients, 2, 0), pow(t, 2)),
)

$ A = #to-typst(A), quad b = #to-typst(b) $\
$ op("det")(A) = #to-typst(det(A)) $\
$ a = #to-typst(coefficients) $\
$ p(t) = #to-typst(polynomial) $\
$ A a = #to-typst(check) $\
rank: #reduced.rank; augmented RREF: #to-typst(reduced.matrix)
```

`vec` constructs a column matrix, and matrix indices are zero-based.
`max-col: 3` tells `row-reduce` to determine pivots and rank from the three
coefficient columns rather than from the augmented right-hand side. The
multiply-back check recovers the original measurement vector.

== Differentiate once, evaluate a grid in one call

For $f(x,y)=x^2+x y+y^2$, derive both gradient components symbolically and then
evaluate all three expressions over a Cartesian grid. This is both clearer and
cheaper than issuing one WebAssembly call per cell and per expression.

```worked
#let x = var("x")
#let y = var("y")
#let f = math($x^2 + x y + y^2$)
#let fx = derivative(f, x)
#let fy = derivative(f, y)
#let grid = evaluate-grid(
  (f, fx, fy),
  (x, y),
  (
    domain(-1, 1, samples: 3),
    domain(-1, 1, samples: 3),
  ),
)
#let cells = range(grid.points.len()).map(index => {
  let point = grid.points.at(index)
  let values = grid.values.at(index)
  (
    [#point.at(0)],
    [#point.at(1)],
    [#values.at(0).re],
    [#values.at(1).re],
    [#values.at(2).re],
  )
}).flatten()

$ f = #to-typst(f) $\
$ partial_x f = #to-typst(fx), quad partial_y f = #to-typst(fy) $

#table(
  columns: 5,
  inset: 4pt,
  stroke: 0.35pt + rgb("#d5dbe1"),
  table.header([$x$], [$y$], [$f$], [$partial_x f$], [$partial_y f$]),
  ..cells,
)
```

The returned dictionary has three fields:

#table(
  columns: (1fr, 1.2fr, 3fr),
  inset: 5pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Field*], [*Type*], [*Meaning*]),
  [`shape`], [`array`], [Sample count along each variable axis.],
  [`points`], [`array`], [Flattened real coordinate rows.],
  [`values`], [`array`], [Rows of `(re, im)` results, one per expression.],
)

The last domain varies fastest in the flattened arrays. Use
`evaluate-many` instead when the points are irregular rather than a Cartesian
product.

= Result contracts

Most symbolic functions return opaque expression or matrix bytes. Functions
that need metadata return Typst dictionaries or arrays:

#table(
  columns: (1.35fr, 1.5fr, 2.7fr),
  inset: 5pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Function*], [*Shape*], [*Contract*]),
  [`evaluate`], [`dictionary`], [`(re: float, im: float)`],
  [`integrate-with-steps`], [`dictionary`], [`result: bytes`; `steps: array` of additive contributions],
  [`evaluate-many`], [`array`], [One result row per point; one complex value per expression],
  [`evaluate-grid`], [`dictionary`], [`shape`, flattened `points`, flattened `values`],
  [`solve-linear`], [`array`], [One exact expression per requested variable],
  [`solve-system`], [`array`], [One row per exact solution; columns follow variable order],
  [`nsolve-system`], [`array`], [One floating-point value per requested variable],
  [`row-reduce`], [`dictionary`], [`matrix: bytes`; `rank: int`],
  [`split-col`], [`array`], [Left and right matrix payloads],
)

Real evaluation results still use the complex record with `im: 0.0`. That
uniform shape lets the same code handle expressions that later become complex.

= Limitations and troubleshooting

== Capability boundaries

- Tymbolica exposes a selected Symbolica surface, not Symbolica's complete
  Python or Rust API. Polynomial objects, Gröbner bases, `collect`, `together`,
  `apart`, `cancel`, persistent/JIT evaluators, arbitrary precision, and the
  full integration package are not currently exposed.

- Exact integration currently accepts polynomial expressions only and returns
  no integration constant in its payload. Its additive contributions follow
  Symbolica's canonical expanded-term order rather than the source-text order.

- `solve-system` targets linear or polynomial nonlinear systems.
  `nsolve` and `nsolve-system` use `f64`, depend on an initial guess, and return
  no iteration or convergence metadata beyond success or failure.

- Matrix entries must be compatible with Symbolica rational-polynomial
  coefficients. Matrices must be nonempty and dimensionally compatible.

- Structural transformations return expressions, not assumption or domain
  records. If a manipulation is valid only under a condition such as
  $x != 1$, preserve that condition in the surrounding document.

- The parser covers the Typst math structures represented by the bundled
  Parsely grammar. Use `array-tree` to inspect unfamiliar input.

== Common failures

#table(
  columns: (1.35fr, 2.05fr, 2.2fr),
  inset: 5pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Symptom*], [*Likely cause*], [*What to try*]),
  [`expected content, found bytes`],
  [An opaque Tymbolica payload was inserted directly into `$...$`.],
  [Render it with `to-typst`, or keep it outside Typst math construction.],
  [A derivative or series leaves a function unchanged],
  [The function lives in the default user namespace.],
  [Use `init(namespace: "symbolica")` for Symbolica built-ins.],
  [A replacement does not match],
  [Pattern and source use different namespaces, or a repeated wildcard would capture unequal expressions.],
  [Inspect `canonical(..., namespaces: true)` and simplify the pattern.],
  [An exact solver errors],
  [The system is inconsistent or outside the supported polynomial surface.],
  [Confirm that every item means `= 0`; try `nsolve-system` for a numerical branch.],
  [A numerical solver errors or finds an unwanted root],
  [The seed is poor or selects a different basin.],
  [Try another physically meaningful initial guess and check residuals.],
  [A matrix operation errors],
  [Shapes differ, a matrix is singular, or an entry is unsupported.],
  [Inspect `matrix-shape`, `det`, and the coefficient expressions.],
)

= API at a glance

#table(
  columns: (1.35fr, 3.8fr),
  inset: 5pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Area*], [*Functions*]),
  [Engine],
  [`init`],
  [Parsing and rendering],
  [`math`, `atom`, `var`, `wild`, `array-tree`, `canonical`, `to-typst-source`, `to-typst`, `to-latex`],
  [Algebra and calculus],
  [`simplify`, `expand`, `factor`, `derivative`, `integrate`, `integrate-with-steps`, `series`],
  [Rewriting],
  [`rule`, `replace`, `replace-multiple`, `replace-wildcards`],
  [Evaluation and solving],
  [`evaluate`, `domain`, `evaluate-many`, `evaluate-grid`, `solve-linear`, `solve-system`, `nsolve`, `nsolve-system`],
  [Matrices],
  [`matrix`, `vec`, `identity`, `eye`, `matrix-add`, `matrix-sub`, `matrix-mul`, `matrix-div-scalar`, `transpose`, `det`, `inv`, `matrix-solve`, `matrix-solve-any`, `row-reduce`, `augment`, `split-col`, `primitive-part`, `content`, `matrix-at`, `matrix-shape`],
  [Scalar constructors],
  [`add`, `mul`, `neg`, `sub`, `div`, `pow`],
)

= API reference

The reference is generated from the doc comments in `lib.typ` with Tidy. Its
examples import the real top-level exports, so compiling this manual also
exercises those wrappers.

#let reference-groups = (
  (
    title: [Engine],
    names: ("init",),
  ),
  (
    title: [Parsing, symbols, and rendering],
    names: (
      "math", "atom", "var", "wild", "array-tree", "canonical",
      "to-typst-source", "to-typst", "to-latex",
    ),
  ),
  (
    title: [Algebra and calculus],
    names: (
      "simplify", "expand", "factor", "derivative", "integrate",
      "integrate-with-steps", "series",
    ),
  ),
  (
    title: [Rewriting],
    names: ("rule", "replace", "replace-multiple", "replace-wildcards"),
  ),
  (
    title: [Evaluation and solving],
    names: (
      "evaluate", "domain", "evaluate-many", "evaluate-grid",
      "solve-linear", "solve-system", "nsolve", "nsolve-system",
    ),
  ),
  (
    title: [Matrices],
    names: (
      "matrix", "vec", "identity", "eye", "matrix-add", "matrix-sub",
      "matrix-mul", "matrix-div-scalar", "transpose", "det", "inv",
      "matrix-solve", "matrix-solve-any", "row-reduce", "augment",
      "split-col", "primitive-part", "content", "matrix-at",
      "matrix-shape",
    ),
  ),
  (
    title: [Scalar constructors],
    names: ("add", "mul", "neg", "sub", "div", "pow"),
  ),
)

#let public-names = docs.functions.filter(
  doc => doc.name.slice(0, 1) != "_",
).map(doc => doc.name)
#let grouped-names = reference-groups.map(group => group.names).flatten()
#assert.eq(
  grouped-names.sorted(),
  public-names.sorted(),
  message: "Every public API function must appear in exactly one reference group.",
)

#for group in reference-groups [
  == #group.title

  #{
    let subset = docs
    subset.functions = group.names.map(name =>
      docs.functions.find(doc => doc.name == name)
    )
    subset.variables = ()
    tidy.show-module(
      subset,
      style: tidy.styles.default,
      show-module-name: false,
      show-outline: false,
      sort-functions: none,
      omit-private-definitions: true,
      first-heading-level: 2,
      break-param-descriptions: true,
    )
  }
]

= Compatibility, attribution, and licensing

This manual describes Tymbolica #package-version and is compiled against the
bundled `tymbolica.wasm`. The package is tested with Typst 0.14 or newer.

Tymbolica's original source code is released under the
#link(repository + "/blob/main/LICENSE")[MIT License]. Symbolica is developed
by the Symbolica contributors and is distributed under its own
#link("https://symbolica.io/license/")[license terms]. The MIT License does not
relicense Symbolica or the bundled WebAssembly artifact; Symbolica's terms
still apply to their use.

For source, issues, and release history, visit
#link(repository)[github.com/lcnbr/tymbolica].
