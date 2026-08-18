#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica

#let manifest = toml("../typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/lcnbr/tymbolica"
#let symbolica-guide = "https://symbolica.io/docs/quick_start.html"
#let symbolica-integration = "https://symbolica.io/posts/symbolic_integration/"
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

Tymbolica keeps symbolic calculation beside the mathematics it belongs to.
You can write a formula in Typst, factor or differentiate it, and place the
answer straight back into the page—without copying expressions to another
program.

Here is the whole pattern: read some mathematics with `math`, do the algebra,
and display the result with `to-typst`.

#let quickstart-source = (
  "<<<#import \"@local/tymbolica:" + package-version + "\": *\n\n"
  + "#let p = math($(x + y)^3 - (x^3 + y^3)$)\n"
  + "#to-typst(factor(expand(p)))"
)
#worked-example(raw(quickstart-source, lang: "worked", block: true))

Tymbolica finds $3 x y (x + y)$ exactly. The same three-step pattern—read,
calculate, display—runs through the rest of this manual.

== Installation

Until Tymbolica is published in Typst Universe, install it from a checkout. On
Linux, place or symlink the repository root at:

#raw(
  "~/.local/share/typst/packages/local/tymbolica/" + package-version,
  lang: "text",
  block: true,
)

Use the corresponding Typst data directory on macOS or Windows. The package
root must contain `typst.toml`; its `typst` directory contains `lib.typ` and
the bundled compressed engines and their loader.
Then import:

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

Tymbolica ships one Symbolica engine with algebra, solving, matrices, and Rubi
integration. It is stored as a compressed asset and expanded transparently by
a small loader. Most operations are available directly from the imported
top-level API. Create an engine with `init()` when you need Rubi integration, a
custom symbol namespace, or another parser grammar:

```typst
#let sym = init()
#let parse = sym.math
#let var = sym.var
#let integrate = sym.integrate
#let x = var("x")
#let result = integrate(parse($x / (x + 1)$), x)
```

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
  [`math`, `var`, and `atom`],
  [Put an answer back into the document],
  [`to-typst`],
  [Factor, expand, differentiate, or take a series],
  [`expand`, `factor`, `derivative`, `series`],
  [Integrate and inspect Rubi's rule path],
  [`init()`, then `sym.integrate-with-steps`],
  [Replace a recurring symbolic pattern],
  [`wild`, `rule`, `replace`],
  [Evaluate a formula at many points],
  [`evaluate-many`, `evaluate-grid`],
  [Solve equations exactly or numerically],
  [`solve-linear`, `solve-system`, `nsolve-system`],
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

== Variables and wildcards have different jobs

#table(
  columns: (0.8fr, 1.4fr, 2.8fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d5dbe1"),
  table.header([*Function*], [*Typical input*], [*Use*]),
  [`math`], [`$x^2 + 1$`], [Read a Typst formula.],
  [`atom`], [a number, string, formula, or expression], [Turn a general value into an expression.],
  [`var`], [`"x"`], [Create a variable that belongs to the mathematics.],
  [`wild`], [`"a"`], [Create a placeholder used only while matching a pattern.],
)

#callout(
  [The distinction that matters],
  [
    A variable is part of the formula. A wildcard is a blank in a pattern. Use
    `var("x")` for the $x$ in $x^2+1$; use `wild("a")` when a replacement rule
    should accept any expression in place of $a$.
  ],
)

Repeated occurrences of the same wildcard in one pattern must capture the same
expression. Different matches can bind it differently; the rewriting guide
below turns that rule into a concrete example.

== Built-in functions such as sine and cosine

Polynomials work with the imported top-level functions. For analytic functions
such as `sin`, `cos`, and `exp`, create a Symbolica-flavoured set of functions
so that differentiation and numerical evaluation recognize them:

```typst
#let sym = init(namespace: "symbolica")
#let parse = sym.math
#let var = sym.var
#let derivative = sym.derivative
#let render = sym.to-typst
#let x = var("x")

#render(derivative(parse($sin(x)$), x))
```

The worked pendulum example binds these functions to short local names so that
the calculation remains readable.

== Write equations as expressions equal to zero

Solver inputs are expressions understood to equal zero. For example,
`math($x + y - 3$)` represents $x+y=3$.

For several variables, each solution row follows the variable order you give
the solver. Exact solving finds all supported algebraic branches; numerical
solving looks for one branch near your initial guess.

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
  math: m, var: v, to-typst: render,
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
>>>#let (math: m, var: v, to-typst: render, derivative, series, replace, solve-linear, neg, add, sub) = sym
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

#let fit = solve-linear((
  sub(replace(model, q, q1), t1),
  sub(replace(model, q, q2), t2),
), (k, b))

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
>>>#let (math: m, var: v, derivative, series, replace, solve-linear, evaluate-many, neg, add, sub) = sym
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
>>>#let fit = solve-linear((sub(replace(model, q, q1), t1), sub(replace(model, q, q2), t2)), (k, b))
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
difference between an ordinary `var("a")` and `wild("a")`.

#callout(
  [Repeating rules],
  [
    `repeat: true` keeps applying the rule until the expression stops changing.
    Use it only for rules that settle: $a arrow.r a+1$, for example, would
    never reach a fixed point.
  ],
  kind: "warning",
)

== Follow Rubi's integration steps

For $x/(1+x)$, the useful idea is to expose a constant term before integrating.
Rubi finds that rewrite, splits the resulting integral, and records the nested
rule path. This is the same example used in
#link(symbolica-integration)[Symbolica's integration introduction].

```worked
#let sym = init()
#let parse = sym.math
#let var = sym.var
#let render = sym.to-typst
#let integrate-with-steps = sym.integrate-with-steps
#let derivative = sym.derivative
#let subtract = sym.sub
#let together = sym.together
#let x = var("x")
#let f = parse($x / (x + 1)$)
#let integration = integrate-with-steps(f, x)
#let residual = together(
  subtract(derivative(integration.result, x), f)
)
#assert(integration.complete)

$ f(x) = #render(f) $\
#for step in integration.steps [
  #h(step.depth * 1.25em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.25em)
  $#render(step.input) = #render(step.output)$
  #linebreak()
]
$ integral f(x) dif x = #render(integration.result) + C $\
verification: $ (partial I)/(partial x) - f(x) = #render(residual) $
```

The mathematics is visible in the tree: first
$x/(1+x) = 1 - 1/(1+x)$, then linearity separates the two integrals, and the
leaves give $x$ and $-log(1+x)$. The indentation comes from each step's `depth`,
so an outer rewrite is followed by the subintegrals it created. For a numbered
rule, `input` and `output` are the integral before and after that rewrite. A
step without a rule number records an auxiliary transformation, such as a
fresh-symbol substitution. `description`, `rule`, `references`, and `source`
explain where each move came from. The displayed result adds the customary
$+C$; Tymbolica itself does not.

#callout(
  [Best-effort integrals],
  [
    Rubi covers many families of integrands, but no finite rule collection solves
    every integral. When it stops early, `complete` is `false`, `result` contains
    an `unintegrable` marker for the unresolved part, and `steps` still records
    the progress.
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
#let s = var("s")
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
>>>#let max-residual(row) = row.map(
>>>  value => calc.abs(value.re),
>>>).sorted().last()
>>>
>>>exact branches:\
>>>#for (index, row) in exact.enumerate() [
>>>  #(index + 1). $x = #to-typst(row.at(0)), y = #to-typst(row.at(1))$
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
#let t = var("t")
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

Tymbolica deliberately presents a smaller surface than Symbolica itself. The
parts covered in this manual work well for exact algebra in documents, but a
few boundaries are worth knowing before you choose an approach:

- Rubi integration methods are fields of the dictionary returned by `init()`.

- Rubi integration adds symbol state, so post-integration outputs remain with
  their originating Tymbolica engine. Matrix payloads are separate from Atom
  payloads.

- Integration returns a best-effort expression when no Rubi rule finishes the
  job. Check `complete` from `integrate-with-steps` when an unevaluated
  remainder matters. No $+C$ is added automatically.

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

For operations beyond this scope—an integral Rubi cannot finish, arbitrary
precision, or deeper polynomial algorithms—use Symbolica directly.

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
things up. The generated groups below are the top-level API. The two
engine-bound integration methods are documented separately afterwards.

#let reference-groups = (
  (
    title: [Parsing, symbols, and rendering],
    names: (
      "math", "atom", "var", "wild", "array-tree", "canonical",
      "to-typst-source", "to-typst", "to-latex", "to-float",
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

== Integration methods

These methods are fields of the dictionary returned by
`init()`; they are not imported as top-level functions. Bind them before use,
as in the worked integration example.

```text
integrate(expr, var) -> bytes
integrate-with-steps(expr, var) -> dictionary
```

`integrate` returns Rubi's best antiderivative without adding $+C$. An
unfinished result contains an `unintegrable` marker. `integrate-with-steps`
returns the same result together with:

- `result` (`bytes`): the best antiderivative;
- `complete` (`bool`): whether Rubi finished every subintegral;
- `steps` (`array`): the ordered transformation tree.

Each step contains `rule` (`int` or `none`), `depth` (`int`), `description`
(`str`), `references` (`array` of strings), `source` (`str`), and the Atom
payloads `input` and `output` (`bytes`). Steps run from an outer rewrite into
the nested integrals it creates; `rule: none` marks an auxiliary
transformation such as a fresh-symbol substitution.

= Compatibility and licensing

This manual describes Tymbolica #package-version. The package is tested with
Typst 0.14 or newer.

Tymbolica's original source code is released under the
#link(repository + "/blob/main/LICENSE")[MIT License]. Symbolica is developed
by the Symbolica contributors and is distributed under its own
#link("https://symbolica.io/license/")[license terms]. The MIT License does not
relicense Symbolica or the bundled WebAssembly engines; Symbolica's terms
still apply to their use. Tymbolica's symbolic integration is supplied by the
#link("https://github.com/symbolica-dev/symbolica-integrate")[MIT-licensed
`symbolica-integrate`] crate and its port of the Rubi rules.

For source, issues, and release history, visit
#link(repository)[github.com/lcnbr/tymbolica].

= Acknowledgements

Tymbolica would not exist without #link("https://symbolica.io/")[Symbolica].
Thank you to its contributors for building and sharing the algebra engine at
the heart of this package, and for making Rubi integration available through
#link("https://github.com/symbolica-dev/symbolica-integrate")[`symbolica-integrate`].
Thanks as well to the Rubi contributors whose rule collection powers symbolic
integration.

Thanks also to #link("https://typst.app/universe/package/parsely/")[Parsely],
which makes it possible to work with mathematics written directly in Typst,
and to #link("https://typst.app/universe/package/tidy/")[Tidy], which powers
this manual's examples and reference pages.
