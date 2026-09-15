#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica

#let manifest = toml("../typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/symbolica-dev/symbolica-typst-plugin"
#let symbolica-guide = "https://symbolica.io/docs/quick_start.html"
#let accent = rgb("#315c88")
#let pale-accent = rgb("#edf4fb")
#let warning = rgb("#9a5b13")
#let pale-warning = rgb("#fff6e8")
#let muted = rgb("#5f6873")

#set document(
  title: "Symbolica Manual",
  author: "Symbolica contributors",
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
        [Symbolica],
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
#let reference-style = {
  let style = tidy.utilities.get-style-functions(tidy.styles.default)
  style.show-example = (..args) => block(
    tidy.styles.default.show-example(..args),
    breakable: false,
  )
  style
}
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
    Symbolica
  ]
  #v(5mm)
  #text(size: 16pt, fill: muted)[The official Symbolica plugin for Typst]
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

    Powered by Symbolica 3.0. Free for any use within Typst.
    No license or license key needed.
  ]
  #v(34mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[Plugin source license] ·
  #link(symbolica-guide)[Symbolica guide]
]

#pagebreak()

#text(size: 22pt, weight: "bold", fill: accent)[Contents]
#v(5mm)

#outline(title: none, depth: 2, indent: auto)

#pagebreak()

= Start here

Symbolica keeps symbolic calculation beside the mathematics it belongs to.
You can write a formula in Typst, factor or differentiate it, and place the
answer straight back into the page—without copying expressions to another
program.

Here is the whole pattern: read some mathematics with `math`, do the algebra,
and display the result with `to-typst`.

#let quickstart-source = (
  "<<<#import \"@local/symbolica:" + package-version + "\": *\n\n"
  + "#let p = math($(x + y)^3 - (x^3 + y^3)$)\n"
  + "#to-typst(factor(expand(p)))"
)
#worked-example(raw(quickstart-source, lang: "worked", block: true))

Symbolica finds $3 x y (x + y)$ exactly. The same three-step pattern—read,
calculate, display—runs through the rest of this manual.

== Installation

Until Symbolica is published in Typst Universe, install it from a checkout. On
Linux, place or symlink the repository root at:

#raw(
  "~/.local/share/typst/packages/local/symbolica/" + package-version,
  lang: "text",
  block: true,
)

Use the corresponding Typst data directory on macOS or Windows. The package
root must contain `typst.toml`; its `typst` directory contains `lib.typ` and
the bundled compressed engine and its loader.
Then import:

#raw(
  "#import \"@local/symbolica:" + package-version + "\": *",
  lang: "typ",
  block: true,
)

From a source checkout, a document can instead import the library by relative
path:

```typst
#import "path/to/symbolica-typst-plugin/typst/lib.typ": *
```

No build step is needed to use the package: everything required is already
included in the checkout.

#callout(
  [Why `@local`?],
  [
    The examples use `@local` because this version is installed from the
    repository. A future Typst Universe release will use its published
    namespace instead.
  ],
)

== Create an engine

The Symbolica 3.0 engine provides algebra, solving, evaluation, and
matrices. It is stored as a compressed asset and expanded transparently by a
small loader. Most operations are available directly from the imported
top-level API. Create an engine with `init()` when you need a custom symbol
namespace or parser grammar:

```typst
#let sym = init()
#let parse = sym.math
#let symbol = sym.symbol
#let x = symbol("x")
#let result = sym.factor(parse($x^2 - 1$))
```

Symbolic integration and its rule steps live in the companion
`symbolica-integrate` package, whose manual starts from Symbolica Atom payloads.

== Where to begin

#table(
  columns: (2.25fr, 3.25fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header(
    [*If you want to…*],
    [*Start with…*],
  ),
  [Turn Typst mathematics into a symbolic expression],
  [`math`, `symbol`, and `atom`],
  [Put an answer back into the document],
  [`to-typst`],
  [Factor, expand, differentiate, or take a series],
  [`expand`, `factor`, `derivative`, `series`],
  [Integrate and inspect Rubi's rule path],
  [Use the companion `symbolica-integrate` package],
  [Replace a recurring symbolic pattern],
  [`wild`, `rule`, `replace`],
  [Evaluate a formula at many points],
  [`evaluate-many`, `evaluate-grid`],
  [Solve equations exactly or numerically],
  [`solve`, `nsolve`, `nsolve-system`],
  [Work with exact matrices],
  [`matrix`, `matrix-solve`, `row-reduce`],
)

= A few ideas before the examples

== Keep expressions symbolic until you display them

The value returned by `math` is a symbolic expression, not something Typst can
typeset on its own. Pass it through as many algebraic operations as you like,
then call `to-typst` where you want the answer to appear.

#align(center)[
  `Typst math` $arrow.r$ `math` $arrow.r$ `symbolic expression`
  $arrow.r$ `transform` $arrow.r$ `to-typst` $arrow.r$ `document`
]

```worked
#let expression = math($(x + 1)^4$)
#let transformed = expand(expression)

original: $ #to-typst(expression) $\
expanded: $ #to-typst(transformed) $
```

For ordinary documents, `to-typst` is usually all you need. `to-latex` is handy
when exporting to another system, while `canonical` shows Symbolica's plain
text form when you are diagnosing a difficult expression.

== Symbols and wildcards have different jobs

#table(
  columns: (0.8fr, 1.4fr, 2.8fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Function*], [*Typical input*], [*Use*]),
  [`math`], [`$x^2 + 1$`], [Read a Typst formula.],
  [`atom`], [a number, string, formula, or expression], [Turn a general value into an expression.],
  [`symbol`], [`"x"`], [Create an ordinary symbol that belongs to the mathematics.],
  [`function`], [`"f"`], [Create a callable function whose complete calls carry metadata.],
  [`wild`], [`"a"`], [Create a placeholder used only while matching a pattern.],
)

#callout(
  [The distinction that matters],
  [
    A variable is part of the formula. A wildcard is a blank in a pattern. Use
    `symbol("x")` for the $x$ in $x^2+1$; use `wild("a")` when a replacement rule
    should accept any expression in place of $a$.
  ],
)

Repeated occurrences of the same wildcard in one pattern must capture the same
expression. Different matches can bind it differently; the rewriting guide
below turns that rule into a concrete example.

`symbol` can also carry information that should not alter the printed formula.
Interpolate it with `#` when that information matters to the calculation:

```worked
#let mass = symbol("m", namespace: "model", tags: ("model::positive",))
#let shell = math($p^2 - #mass^2$)

$ p^2 - m^2 = #to-typst(shell) $
```

The page still shows an ordinary $m$. Behind it, a versioned envelope carries
the exact Symbolica Atom plus an inspectable description containing its
namespace and tags. The Atom is authoritative when `math` reads the formula;
packages layered on top can give the tags their own meaning. Namespaced tags
are registered on the Symbolica symbol itself, so they survive algebraic and
cross-plugin round trips. There is no separate `notation.symbol` or `var`
alias.

A content value is not callable in Typst, so a custom function head uses the
separate `function` constructor. Its metadata belongs to the whole call:

```worked
#let response = function("R", namespace: "model", tags: ("model::response",))
#let time = symbol("t", namespace: "model")
#let value = math($#response(time) + 1$)

$ R(t) + 1 = #to-typst(value) $
```

The same identities can control document-side notation. Exact `heads` style a
symbol everywhere; exact `calls` receive the complete function arguments and
can choose a different layout. This changes only presentation—the returned
content still carries the exact Atom.

```worked
#let x = symbol("x", namespace: "notation_example")
#let f = function("f", namespace: "notation_example")
#let expression = f(add(x, 1))
#let display = notation(
  heads: ("notation_example::x": $xi$),
  calls: (
    "notation_example::f": ctx => {
      let (argument,) = ctx.visual-arguments
      $cal(F)[#argument]$
    },
  ),
)

$ #to-typst(expression, notation: display) $
```

== Built-in functions such as sine and cosine

Polynomials work with the imported top-level functions. For analytic functions
such as `sin`, `cos`, and `exp`, create a Symbolica-flavoured set of functions
so that differentiation and numerical evaluation recognize them:

```typst
#let sym = init(namespace: "symbolica")
#let parse = sym.math
#let symbol = sym.symbol
#let derivative = sym.derivative
#let render = sym.to-typst
#let x = symbol("x")

#render(derivative(parse($sin(x)$), x))
```

The worked pendulum example binds these functions to short local names so that
the calculation remains readable.

== Write equations as expressions equal to zero

Solver inputs are expressions understood to equal zero. For example,
`math($x + y - 3$)` represents $x+y=3$.

For several variables, each solution row follows the variable order you give
the solver. Exact solutions are in the result's `branches` array. With symbolic
parameters, check `coverage`: a `"generic"` result applies only where every
expression in `coverage-guard` is nonzero. Numerical solving looks for one
branch near your initial guess.

= Worked mathematical guides

The quickest way to learn the package is to follow a calculation from question
to answer. Each example below ends with a check, because a plausible-looking
formula is not yet a convincing result.

== Calibrate a pendulum model

A pendulum gives us a small but complete modelling problem. Begin with the
potential $V(theta)=kappa(1-cos theta)$, derive the restoring torque, and take a
cubic small-angle approximation. Two torque readings are then enough to recover
both the unknown scale $kappa$ and a sensor offset $tau_0$.

The calculation follows the
#link("https://github.com/symbolica-dev/symbolica#pendulum-calibration")[
  pendulum-calibration example in Symbolica's repository README
]. We create the `symbolica` set of functions because `cos` must be understood
as the analytic cosine rather than as an arbitrary function name.

=== Derive the model

```worked
#let sym = init(namespace: "symbolica")
#let (
  math: m, symbol: v, to-typst: render,
  derivative, series, neg, add,
) = sym

#let potential = m($kappa (1 - cos(theta))$)
#let q = v("θ")
#let k = v("κ")
#let b = v("τ₀")

#let torque = neg(derivative(potential, q))
#let small-angle = series(torque, q, 0, 3)
#let model = add(small-angle, b)

$ V(theta) = #render(potential) $\
$ tau(theta) = -(partial V)/(partial theta) = #render(torque) $\
$ tau_"small"(theta) + tau_0 = #render(model) $
```

The derivative gives the restoring torque, while the cubic series keeps the
first nonlinear correction to the familiar small-angle law. Adding $tau_0$
leaves a model that is linear in the two unknown parameters.

=== Fit two readings exactly

```worked
>>>#let sym = init(namespace: "symbolica")
>>>#let (math: m, symbol: v, to-typst: render, derivative, series, replace, solve, neg, add, sub) = sym
>>>#let potential = m($kappa (1 - cos(theta))$)
>>>#let q = v("θ")
>>>#let k = v("κ")
>>>#let b = v("τ₀")
>>>#let torque = neg(derivative(potential, q))
>>>#let small-angle = series(torque, q, 0, 3)
>>>#let model = add(small-angle, b)
#let q1 = v("θ₁")
#let q2 = v("θ₂")
#let t1 = v("τ₁")
#let t2 = v("τ₂")

#let fit = solve((
  sub(replace(model, q, q1), t1),
  sub(replace(model, q, q2), t2),
), (k, b)).branches.first().values

$ kappa = #render(fit.at(0)) $\
$ tau_0 = #render(fit.at(1)) $
```

Nothing has been rounded: both fitted parameters are still formulas in the
angles and measured torques. Now insert two observations,
$(theta_1,tau_1)=(0.10,-0.4697)$ and
$(theta_2,tau_2)=(0.20,-0.9545)$.

=== Insert the measurements and check the fit

```worked
>>>#let sym = init(namespace: "symbolica")
>>>#let (math: m, symbol: v, derivative, series, replace, solve, evaluate-many, neg, add, sub) = sym
>>>#let potential = m($kappa (1 - cos(theta))$)
>>>#let q = v("θ")
>>>#let k = v("κ")
>>>#let b = v("τ₀")
>>>#let torque = neg(derivative(potential, q))
>>>#let model = add(series(torque, q, 0, 3), b)
>>>#let q1 = v("θ₁")
>>>#let q2 = v("θ₂")
>>>#let t1 = v("τ₁")
>>>#let t2 = v("τ₂")
>>>#let fit = solve((sub(replace(model, q, q1), t1), sub(replace(model, q, q2), t2)), (k, b)).branches.first().values
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

$ kappa approx #calc.round(fitted.at(0).re, digits: 6), quad
  tau_0 approx #calc.round(fitted.at(1).re, digits: 6) $\
predicted torques:
#predictions.map(row => str(calc.round(row.first().re, digits: 4))).join(", ")
```

The formula for $kappa$ remains exact until the measured values are inserted.
Substituting the fitted parameters back into the model reproduces both torque
readings, so the final line checks the entire chain from potential to fit.

== Rewrite a repeated identity with wildcards

This is where a wildcard earns its keep. Suppose an expression contains several
copies of $sin^2(a)+cos^2(a)$, but the argument $a$ is different each time. The
pattern should insist that the two functions share an argument without fixing
what that argument is.

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
difference between an ordinary `symbol("a")` and `wild("a")`.

#callout(
  [Repeating rules],
  [
    `repeat: true` keeps applying the rule until the expression stops changing.
    Use it only for rules that settle: $a arrow.r a+1$, for example, would
    never reach a fixed point.
  ],
  kind: "warning",
)

== Separate a rational response into modes

Suppose a response function arrives in a form with one removable factor and
two remaining poles. Cancel the shared factor, split the reduced response into
simple modes, and then recombine the modes to check that nothing was lost.
This follows the
#link(symbolica-guide)[rational-expression workflow in Symbolica's First Steps].

```worked
#let s = symbol("s")
#let response = math($((s + 3)(2 s + 5)) / (s^3 + 6 s^2 + 11 s + 6)$)
#let reduced = cancel(response)
#let modes = apart(reduced, s)
#let residual = together(sub(modes, reduced))

$
  H(s) &= #to-typst(response) \
  H_"reduced"(s) &= #to-typst(reduced) \
                   &= #to-typst(modes)
$\
verification: $ #to-typst(residual) $
```

The two terms expose poles at $s=-1$ and $s=-2$ separately, which is often the
useful form for inverse transforms or modal reasoning. `together` returns a
zero residual after recombination. The cancellation does hide the original
restriction $s != -3$; keep that restriction when the domain matters.

== Solve a nonlinear system exactly and numerically

The circle $x^2+y^2=25$ and the line $x-y=1$ meet twice. The exact solver should
find both intersections; the numerical solver should find the one nearest its
starting point.

```worked
#let x = symbol("x")
#let y = symbol("y")
#let system = (
  math($x^2 + y^2 - 25$),
  math($x - y - 1$),
)

#let exact = solve(system, (x, y), domain: "real")
#let positive = nsolve-system(
  system, (x, y), (3.0, 3.0), prec: 1e-10,
)
#let negative = nsolve-system(
  system, (x, y), (-3.0, -3.0), prec: 1e-10,
)
#let checks = evaluate-many(
  system, (x, y), (positive, negative),
)
>>>#let max-residual(row) = row.map(
>>>  value => calc.abs(value.re),
>>>).sorted().last()
>>>
>>>exact branches:\
>>>#for (index, solution) in exact.branches.enumerate() [
>>>  #(index + 1). $x = #to-typst(solution.values.at(0)), y = #to-typst(solution.values.at(1))$
>>>  #linebreak()
>>>]
>>>seed $(3,3)$ $arrow.r$
>>>(#positive.map(value => str(calc.round(value, digits: 6))).join(", "));
>>>maximum residual #repr(max-residual(checks.first()))\
>>>seed $(-3,-3)$ $arrow.r$
>>>(#negative.map(value => str(calc.round(value, digits: 6))).join(", "));
>>>maximum residual #repr(max-residual(checks.last()))
```

The exact calculation finds both branches. Starting near either intersection
selects that numerical branch, and the small residual confirms that the point
lies on both curves. A poor starting point may still converge elsewhere—or not
at all.

== Recover an interpolating polynomial with matrices

Find the quadratic through $(0,1)$, $(1,3)$, and $(2,8)$. Writing
$p(t)=a_0+a_1 t+a_2 t^2$ turns interpolation into the exact matrix equation
$A a=b$.

```worked
#let t = symbol("t")
#let A = matrix((
  (1, 0, 0),
  (1, 1, 1),
  (1, 2, 4),
))
#let b = vec((1, 3, 8))
#let coefficients = matrix-solve(A, b)
#let check = matrix-mul(A, coefficients)
#let exact = matrix-is-zero(matrix-sub(check, b))
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
verification: residual matrix is exactly zero: #exact
```

The nonzero determinant tells us the coefficients are unique. Reading them
back gives the polynomial, and multiplying $A a$ recovers the three original
measurements.

== Map a gradient across a grid

Consider the quadratic surface $f(x,y)=x^2+x y+y^2$. We first derive its two
gradient components, then sample the height and gradient together on a small
Cartesian grid.

```worked
#let x = symbol("x")
#let y = symbol("y")
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
>>>#let cells = range(grid.points.len()).map(index => {
>>>  let point = grid.points.at(index)
>>>  let values = grid.values.at(index)
>>>  (
>>>    [#point.at(0)],
>>>    [#point.at(1)],
>>>    [#values.at(0).re],
>>>    [#values.at(1).re],
>>>    [#values.at(2).re],
>>>  )
>>>}).flatten()

$ f = #to-typst(f) $\
$ partial_x f = #to-typst(fx), quad partial_y f = #to-typst(fy) $

>>>#table(
>>>  columns: 5,
>>>  inset: 4pt,
>>>  stroke: 0.35pt + rgb("#d5dbe1"),
>>>  table.header([$x$], [$y$], [$f$], [$partial_x f$], [$partial_y f$]),
>>>  ..cells,
>>>)
```

The table makes the geometry visible: the gradient vanishes at the origin and
points uphill everywhere else. `evaluate-grid` pairs each point with one value
for every requested expression; use `evaluate-many` when your sample points do
not form a Cartesian product.

= What to expect

The Typst plugin exposes a subset of the Symbolica engine's features. The
parts covered in this manual work well for exact algebra in documents, but a
few boundaries are worth knowing before you choose an approach:

- Symbolic integration is intentionally a separate concern. The companion
  `symbolica-integrate` package accepts and returns the same portable Atom payloads.
  Matrix payloads remain separate from Atom payloads.

- Exact system solving is intended for linear and polynomial equations.
  Numerical solving depends on a starting point and gives an approximate
  answer rather than a derivation of convergence.

- Decimal literals remain floating-point values. A current upstream Wasm bug
  can mis-evaluate them inside analytic functions. Write exact fractions for
  those inputs—for example, `cos(1/2)`—and apply `to-float` to the result.

- Matrix entries must be rational-polynomial expressions, and their dimensions
  must agree for the requested operation.

- Algebraic transformations do not keep a separate list of assumptions. If a
  manipulation is valid only under a condition such as
  $x != 1$, preserve that condition in the surrounding document.

- Some unusual Typst math structures may not parse. `array-tree` can help show
  what the parser received.

For symbolic integration, use `symbolica-integrate`. For arbitrary precision or
deeper polynomial algorithms, use Symbolica directly.

== When something looks wrong

#table(
  columns: (1.35fr, 2.05fr, 2.2fr),
  inset: 5pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Symptom*], [*Likely cause*], [*What to try*]),
  [`expected content, found bytes`],
  [A symbolic result was inserted directly into `$...$`.],
  [Display it with `to-typst`.],
  [A symbolic result fails in another API],
  [The bytes hold a matrix or came from an incompatible package version.],
  [Use the matching package version and keep matrix values in matrix APIs.],
  [A derivative or series leaves a function unchanged],
  [`sin`, `cos`, or another analytic function was read as an ordinary name.],
  [Use `init(namespace: "symbolica")` for Symbolica built-ins.],
  [An analytic function of a decimal gives an unexpected value],
  [Its floating-point argument encountered the current upstream Wasm bug.],
  [Use an exact fraction such as `cos(1/2)`, then call `to-float`.],
  [A replacement does not match],
  [Repeated wildcards would have to capture different expressions.],
  [Check that every occurrence of the wildcard should match the same value.],
  [An exact solver errors],
  [An equation was not rearranged to zero, or the system is outside the supported polynomial scope.],
  [Move every term to the left; try `nsolve-system` for a numerical branch.],
  [A numerical solver errors or finds an unwanted root],
  [The starting point leads to a different root or no root.],
  [Try another physically meaningful initial guess and check residuals.],
  [A matrix operation errors],
  [Shapes differ, a matrix is singular, or an entry is unsupported.],
  [Inspect `matrix-shape`, `det`, and the coefficient expressions.],
)

= API reference

The worked chapters are meant for reading; this section is meant for looking
things up. The generated groups below cover the complete top-level API.

#let reference-groups = (
  (
    title: [Parsing, symbols, and rendering],
    names: (
      "math", "atom", "symbol", "function", "wild", "array-tree", "canonical",
      "notation", "merge-notation", "to-typst-source", "to-typst", "to-latex", "to-float",
    ),
  ),
  (
    title: [Algebra and calculus],
    names: (
      "simplify", "expand", "factor", "together", "cancel", "apart",
      "collect", "coefficient", "coefficient-list", "terms",
      "indeterminates", "contains", "is-constant",
      "derivative", "series",
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
      "solve", "nsolve", "nsolve-system",
    ),
  ),
  (
    title: [Matrices],
    names: (
      "matrix", "vec", "identity", "eye", "matrix-add", "matrix-sub",
      "matrix-mul", "matrix-div-scalar", "transpose", "det", "inv",
      "matrix-solve", "matrix-solve-any", "row-reduce", "augment",
      "split-col", "primitive-part", "content", "matrix-at",
      "matrix-shape", "matrix-is-zero", "matrix-is-diagonal",
      "matrix-derivative",
    ),
  ),
  (
    title: [Scalar constructors],
    names: ("add", "mul", "neg", "sub", "div", "pow"),
  ),
  (
    title: [Advanced configuration],
    names: ("init",),
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
      style: reference-style,
      show-module-name: false,
      show-outline: false,
      sort-functions: none,
      omit-private-definitions: true,
      first-heading-level: 2,
      break-param-descriptions: true,
    )
  }
]

= Compatibility and licensing

This manual describes the official Symbolica Typst plugin #package-version,
powered by Symbolica 3.0. The package is tested with Typst 0.14 or newer.

The official `symbolica` plugin and its `symbolica-integrate` companion are
free to use for any use within Typst, including academic and commercial work.
No Symbolica license or license key is needed for use within Typst.

The original plugin source code is released under the
#link(repository + "/blob/main/LICENSE")[MIT License]. The bundled WebAssembly
engines are included with redistribution permission. The underlying Symbolica
computer algebra system is governed by its own
#link("https://symbolica.io/license/")[license terms] outside this Typst usage
permission.

For source, issues, and release history, visit
#link(repository)[github.com/symbolica-dev/symbolica-typst-plugin].

= Acknowledgements

This plugin is powered by #link("https://symbolica.io/")[Symbolica].
Thank you to its contributors for building and sharing the algebra engine at
the heart of this package.

Thanks also to #link("https://typst.app/universe/package/parsely/")[Parsely],
which makes it possible to work with mathematics written directly in Typst,
and to #link("https://typst.app/universe/package/tidy/")[Tidy], which powers
this manual's examples and reference pages.
