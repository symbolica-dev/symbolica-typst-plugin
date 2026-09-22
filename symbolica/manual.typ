#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica

#let manifest = toml("../typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/symbolica-dev/symbolica-typst-plugin"
#let symbolica-guide = "https://symbolica.io/docs/quick_start.html"
// Symbolica website's light palette; see ../docs/manual-assets/README.md.
#let accent = rgb("#087f64")
#let pale-accent = rgb("#e9f5ef")
#let ink = rgb("#18222f")
#let link-color = rgb("#0c7fc0")
#let border = rgb("#d8e3ef")
#let heading-font = "DejaVu Sans"
#let warning = rgb("#9a5b13")
#let pale-warning = rgb("#fff6e8")
#let muted = rgb("#445365")
#let body-text-size = 10.5pt
#let code-text-size = 8pt

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
      line(length: 100%, stroke: 0.35pt + border)
    }
  },
  footer: context {
    if counter(page).get().first() > 1 {
      align(center, text(size: 8pt, fill: muted, counter(page).display("1")))
    }
  },
)
#set text(font: "Libertinus Serif", size: body-text-size, fill: ink)
#set par(justify: true, leading: 0.65em)
#set heading(numbering: "1.1.1")
#show heading: set text(font: heading-font, fill: accent)
#show heading.where(level: 4): set heading(numbering: none)
#show heading.where(level: 5): set heading(numbering: none)
#show link: set text(fill: link-color)
#show raw: set text(font: "DejaVu Sans Mono")
#show raw.where(block: true): set text(size: code-text-size)

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
    #block(above: 0pt, below: 5pt)[
      #text(font: heading-font, size: 9.5pt, weight: "bold", fill: color)[#title]
    ]
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
#let example-layout(code, preview, ..options) = layout(size => context {
  set text(size: body-text-size)
  let gap = 8pt
  let code = {
    // Override both Raw's default size and Tidy's additional reduction.
    show raw: set text(size: code-text-size)
    code
  }
  let source-width = measure(code).width + 10pt
  let result-width = measure(preview).width + 20pt
  let available = size.width - gap
  let source-column = calc.max(available * 0.55, source-width)
  let beside = (
    source-column <= available * 0.75 and
    result-width > 20pt and result-width <= available - source-column
  )

  tidy.show-example.default-layout-example(
    code, preview,
    dir: if beside { ltr } else { ttb },
    ratio: if beside {
      (source-column + gap / 2) / (size.width - source-column - gap / 2)
    } else { 1 },
    scale-preview: 100%,
    code-block: block.with(radius: 3pt, stroke: 0.5pt + border),
    preview-block: block.with(radius: 3pt, fill: pale-accent),
    col-spacing: gap,
  )
})
#let reference-style = {
  let style = tidy.utilities.get-style-functions(tidy.styles.default)
  style.show-example = (..args) => block(
    tidy.show-example.show-example(..args, layout: example-layout),
    breakable: false,
  )
  style
}
#let worked-example(code) = block(
  tidy.show-example.show-example(
    raw(code.text, lang: "typ", block: true),
    scope: (symbolica: symbolica),
    preamble: example-preamble,
    mode: "markup",
    layout: example-layout,
  ),
  breakable: false,
)
#show raw.where(lang: "worked"): worked-example

#align(center)[
  #v(25mm)
  #symbolica.logo(size: 25mm)
  #v(6mm)
  #text(font: heading-font, size: 38pt, weight: "bold", fill: ink)[Symbolica]
  #v(4mm)
  #text(font: heading-font, size: 16pt, fill: muted)[Computer algebra for Typst]
  #v(12mm)
  #text(size: 11pt)[User manual · version #package-version]
  #v(20mm)
  #block(
    width: 76%,
    inset: 14pt,
    radius: 5pt,
    fill: pale-accent,
  )[
    Symbolic algebra, calculus, and numerical evaluation in Typst.

    Free for all use within Typst, including commercial work.
    No Symbolica license key is required.
  ]
  #v(26mm)
  #link("https://symbolica.io/")[symbolica.io] ·
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE-SYMBOLICA-TYPST.md")[Typst permission]
]

#pagebreak()

#text(font: heading-font, size: 22pt, weight: "bold", fill: accent)[Contents]
#v(5mm)

#outline(title: none, depth: 2, indent: auto)

#pagebreak()

= Introduction

#link("https://symbolica.io/")[Symbolica] is a computer algebra system used in
research, available as a Rust crate and through Python bindings. This plugin
runs it as WebAssembly inside Typst, so you can factor polynomials, take
derivatives, solve equations, and evaluate the results directly in your document.

When writing a derivation, you often expand an expression, substitute a
definition, or isolate a variable. You can write these operations in Typst and
let Symbolica calculate each step. Change the starting expression, and the
displayed results update with it.

`parse` reads a formula and `to-typst` displays a result. For example, expand
and factor a polynomial:

#let quickstart-source = (
  "<<<#import \"@preview/symbolica:" + package-version + "\": *\n\n"
  + "#let p = parse($(x + y)^3 - (x^3 + y^3)$)\n"
  + "$ #to-typst(factor(expand(p))) $"
)
#worked-example(raw(quickstart-source, lang: "worked", block: true))

#callout(
  [Free within Typst],
  [
    The plugin is free for all use within Typst, including commercial work.
    No license key is needed.

    Symbolica itself is source-available. Outside Typst, its
    #link("https://symbolica.io/license/")[standard terms] allow one core and
    one instance per device for free non-commercial use; commercial work
    requires a professional license. The plugin's
    #link(repository + "/blob/main/LICENSE-SYMBOLICA-TYPST.md")[Typst permission]
    covers its use here.
  ],
)

== Installation

Import Symbolica from Typst Universe. Typst downloads and caches the package
automatically:

#raw(
  "#import \"@preview/symbolica:" + package-version + "\": *",
  lang: "typ",
  block: true,
)

Use the wildcard import for ordinary calculations. The API names leave Typst's
native `math`, `symbol`, `function`, `content`, and math layout functions
available. Import as a module if you prefer qualified names.

= Expressions and symbols

== Create an expression

`parse` turns a formula into a symbolic expression. Store it in a variable,
apply an operation, and use `to-typst` to show the result:

```worked
#let f = parse($(x + 1)^3$)
#let expanded = expand(f)

$
  (x + 1)^3 &= #to-typst(expanded)
$
>>>#assert.eq(canonical(expand(subtract(expanded, parse($x^3 + 3 x^2 + 3 x + 1$)))), "0")
```

The expression `f` keeps its original form; `expand(f)` returns a new value.
Use `factor` to look for a product and `collect` to group terms by powers of a
chosen variable.

Some arithmetic happens automatically. Symbolica combines like terms and
reduces fractions when you create an expression:

```worked
#let f = parse($2 x + x/2 + 1/3 + 1/6$)
$ #to-typst(f) $
>>>#assert.eq(canonical(expand(subtract(f, parse($5/2 x + 1/2$)))), "0")
```

Integers and fractions remain exact. Decimal input such as `0.1` is stored
as a floating-point number. Expansion and factorization require explicit
calls; automatic normalization does not choose a single form for every
algebraically equivalent expression.

For plain text, use `canonical`; for LaTeX source, use `to-latex`.
Symbolica's #link("https://symbolica.io/docs/expressions.html")[expression guide]
explains its normal form in more detail.

== Parse formulas and expression strings

Use Typst math when writing a formula in your document. Use a string when
reading an expression from data or another program:

```worked
#let from-math = parse($x^2 / (1 + x)$)
#let from-string = parse("x^2/(1+x)")
>>>#assert.eq(canonical(from-math), canonical(from-string))
$ #to-typst(from-string) $
```

Strings use Symbolica's syntax, including explicit multiplication:
`parse("2*x + y")`. Typst math allows juxtaposition: `parse($2 x + y$)`.
The string parser does not evaluate Typst code.

`parse` also accepts integers, floats, and existing expressions. Integers stay
exact; floats keep their value. Existing Atom bytes pass through unchanged,
including their symbol identities, regardless of `namespace`.

Algebra functions accept numbers and single-leaf strings directly. For a whole
expression in a string, call `parse` first: `expand(parse("(x+1)^2"))`.

To create one variable, use `literal("x")`. To give symbols separate names
or change the interpretation of Typst math, see @engine-guide.

== Substitute a value or evaluate a formula

`replace` substitutes an expression for a variable and keeps the answer
symbolic. `evaluate` calculates a numerical value once you supply values for
the remaining variables:

```worked
#let x = literal("x")
#let f = parse($x^2 + 2 x$)
#let shifted = replace(f, x, parse($t + 1$))
#let value = evaluate(f, values: ((x, 2.0),))

$ f(t + 1) = #to-typst(shifted) $\
$ f(2) = #value.re $
>>>#assert.eq(value.re, 8.0)
>>>#assert.eq(value.im, 0.0)
```

`values` is an array of `(variable, value)` pairs. The result has fields `re`
and `im` for its real and imaginary parts. Here the imaginary part is zero.
For several input points, use `evaluate-many`; for a Cartesian grid, use
`evaluate-grid`. The sampling example in @sampling-guide shows both the
variable order and the shape of the returned data.

== Variables, functions, and wildcards

`literal("x")` creates a variable that can be differentiated or substituted.
`function-head("g")` creates a callable name for an unspecified function:

```worked
#let x = literal("x")
#let g = function-head("g")
#let f = parse($#g(x) + x$)
$ #to-typst(replace(f, x, parse($y + 1$))) $
>>>#assert.eq(canonical(expand(subtract(replace(f, x, 2), parse($g(2) + 2$)))), "0")
```

Substitution reaches the arguments of `g` as well as the separate $x$.
Declaring a function gives it a symbolic identity; numerical evaluation
also needs to know what the function computes.

A wildcard stands for an expression to be matched. In this rule, `a` takes
the argument of each call to `g`:

```worked
#let g = function-head("g")
#let a = wild("a")
#let squared = rule(g(a), pow(a, 2))
#let f = parse($#g(2) + #g(5)$)

$ #to-typst(replace-multiple(f, (squared,))) $
>>>#assert.eq(canonical(replace-multiple(f, (squared,))), "29")
```

`wild("a")` creates the pattern symbol `a_`; give the base name without the
underscore. Repeated occurrences of the same wildcard in one pattern must
match the same expression. The trigonometric example in @rewrite-guide uses
this to require equal arguments for sine and cosine.

=== Literal display labels

Sometimes a whole label should act as one variable. Pass math or text to
`literal` to keep that label intact. For example, `literal($x+1$)` is one
symbol, whereas `parse($x+1$)` adds two expressions. Replacing $x$ has no
effect inside a literal label:

```worked
#let label = literal($x + 1$)
$ #to-typst(pow(label, 2)) $
```

Both renderers group composite labels so their boundaries stay clear in
products and powers. `to-typst` retains exact Atom metadata. `to-typst-source`
preserves appearance, but its plain text does not preserve the opaque identity.

A namespace groups symbol names: two groups can each have their own $x$.
See @namespaces-guide for examples. Without an explicit name, the same display
in the same namespace creates the same symbol. Simple `literal($x$)` and `literal("x")` agree, as do
`literal($a_0$)` and `parse($a_0$)`. A `name` can be supplied only for content;
it selects a semantic name independently of the label:

```worked
#let first = literal(
  $a_0$, name: "first_coefficient", namespace: "model",
)
#let second = literal(
  $a_0$, name: "second_coefficient", namespace: "model",
)
>>>#assert.ne(canonical(first, namespaces: true), canonical(second, namespaces: true))
$ #to-typst(subtract(first, second)) $
```

The difference does not cancel: the labels look alike, but the names are
different. Declare a named label before using that name in other expressions.
Literal labels support math and text, including attachments. For custom
styling or layout, keep a simple symbol name and use `notation`.

=== Preserve symbol identity in a formula

Use `#` to insert a declared symbol into Typst math. Its namespace, tags,
and display label travel with it:

```worked
#let mass = literal("m", namespace: "model", tags: ("model::mass",))
#let shell = parse($p^2 - #mass^2$)

$ p^2 - m^2 = #to-typst(shell) $
```

`parse` reads the inserted symbol's identity, including its `model`
namespace. The tag `model::mass` is a label for use by notation rules or other
packages; it does not by itself impose a mathematical assumption.
Declare symbols with tags before parsing expressions that use them.

`function-head` also accepts a namespace and tags. Inserting a call preserves
both its head and its arguments:

```worked
#let response = function-head(
  "R", namespace: "model", tags: ("model::response",),
)
#let time = literal("t", namespace: "model")
#let value = parse($#response(time) + 1$)

$ R(t) + 1 = #to-typst(value) $
```

=== Choose how an expression is displayed

Use `notation` to change a symbol's label or the layout of a function call.
Here `x` displays as $xi$, and `f(x+1)` displays with a calligraphic letter
and square brackets:

```worked
#let x = literal("x", namespace: "notation_example")
#let f = function-head("f", namespace: "notation_example")
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

`heads` maps full symbol names to labels. A `calls` renderer receives the
call's arguments through `ctx.visual-arguments`. The displayed result retains
its mathematical identity, so `parse` can read it back. `to-typst-source`
returns source text with the same default layout, but plain text cannot carry
this hidden identity.

== Differentiate built-in functions

Symbolica knows derivatives of functions such as sine, cosine, and the
exponential. Select its built-in namespace when parsing them from Typst math:

```worked
#let x = literal("x")
#let f = parse($exp(#x) sin(#x)$, namespace: "symbolica")
$ f'(x) = #to-typst(derivative(f, x)) $
>>>#assert.eq(canonical(expand(subtract(derivative(f, x), parse("exp(x)*(sin(x)+cos(x))")))), "0")
```

The inserted `#x` retains the identity of the declared variable. The namespace
selects Symbolica's function definitions; @namespaces-guide explains how to
set it for a whole calculation.

== Use special functions

For functions that Typst does not name directly, use a constructor such as
`polylog` or `bessel-j`. These always select Symbolica's built-in function:

```worked
#let x = literal("x")
#let f = polylog(2, x)
#let value = evaluate(polylog(2, divide(1, 2)))

$ f(x) = #to-typst(f) $\
$ f'(x) = #to-typst(derivative(f, x)) $\
$ f(1/2) approx #calc.round(value.re, digits: 8) $
```

The constructor's result can go directly into algebra and evaluation calls,
or into a larger formula: `parse($#polylog(2, x) + 1$)`.

The direct constructors are:

#table(
  columns: (1fr, 1.5fr, 1.5fr),
  inset: 6pt,
  stroke: 0.35pt + rgb("#c9d3dd"),
  table.header([*Function*], [*Constructor*], [*Meaning*]),
  [Gamma], [`gamma-function(z)`], [$Gamma(z)$],
  [Polygamma], [`polygamma(n, z)`], [Order $n$, with nonnegative integer $n$; $n=0$ is digamma.],
  [Polylogarithm], [`polylog(s, z)`], [Order $s$, argument $z$.],
  [Riemann zeta], [`zeta-function(s)`], [$zeta(s)$],
  [Bessel], [`bessel-j(nu, z)`, `bessel-y(nu, z)`], [$J_nu(z)$ and $Y_nu(z)$.],
  [Modified Bessel], [`bessel-i(nu, z)`, `bessel-k(nu, z)`], [$I_nu(z)$ and $K_nu(z)$.],
)

Use `evaluate` for a numerical value, then choose how many digits to display:

```worked
$ J_0(1) approx #calc.round(evaluate(bessel-j(0, 1)).re, digits: 8) $\
$ zeta(3) approx #calc.round(evaluate(zeta-function(3)).re, digits: 8) $
```

For other built-ins, use `function-head(name, namespace: "symbolica")` with
Symbolica's spelling, such as `"bessel_j"`. An unknown name creates an
unspecified symbolic function. It will not acquire a numerical definition
from its name alone. Supply numerical values for every free variable before
calling `evaluate`. Some derivatives and series of special functions are
not implemented; exact fractional arguments also avoid the decimal-input
issue described under @limitations.

== Write equations as expressions equal to zero

Solver inputs are expressions understood to equal zero. For example,
`parse($x + y - 3$)` represents $x+y=3$.

Pass the unknowns in the order you want them returned. `solve` returns one
entry in `branches` for each solution; its `values` array follows that order.
The worked example in @solving-guide compares these exact branches with
numerical solutions from two starting points.

With symbolic parameters, inspect `coverage`: a `"generic"` result applies
where the expressions in `coverage-guard` are nonzero. `nsolve` and
`nsolve-system` instead search from a supplied starting point and may fail to
converge or find a different root when that point changes.

= Worked examples

== Derive and fit a pendulum model

A pendulum has potential $V(theta)=kappa(1-cos theta)$. Suppose a torque sensor
also has an unknown offset $tau_0$. We will derive a small-angle model, then
fit $kappa$ and $tau_0$ to two measurements. This follows Symbolica's
#link("https://github.com/symbolica-dev/symbolica#pendulum-calibration")[pendulum-calibration example].

=== Differentiate and expand

The restoring torque is $-partial V/partial theta$. Keep terms through
$theta^3$ to include the first correction to the linear approximation.

```worked
#let angle = literal($theta$)
#let stiffness = literal($kappa$)
#let offset = literal($tau_0$)
#let potential = parse(
  $#stiffness (1 - cos(#angle))$
)
#let torque = neg(derivative(potential, angle))
#let model = add(series(torque, angle, 0, 3), offset)

$ tau(theta) = #to-typst(torque) $\
$ tau_"model"(theta) = #to-typst(model) $
>>>#assert.eq(canonical(expand(subtract(model, parse($-#stiffness #angle + #stiffness #angle^3 / 6 + #offset$)))), "0")
```

`series` returns the truncated expression; it does not include an order term.
This model is approximate in $theta$ but linear in its unknown parameters.

=== Solve for the parameters

Let the measurements be $(theta_1,tau_1)$ and $(theta_2,tau_2)$. Substitute each
angle, subtract the measured torque, and pass both residuals to `solve`.
Each input expression is understood to equal zero.

```worked
>>>#let angle = literal($theta$)
>>>#let stiffness = literal($kappa$)
>>>#let offset = literal($tau_0$)
>>>#let potential = parse($#stiffness (1 - cos(#angle))$)
>>>#let torque = neg(derivative(potential, angle))
>>>#let model = add(series(torque, angle, 0, 3), offset)
#let angle1 = literal($theta_1$)
#let angle2 = literal($theta_2$)
#let torque1 = literal($tau_1$)
#let torque2 = literal($tau_2$)
#let equations = (
  subtract(replace(model, angle, angle1), torque1),
  subtract(replace(model, angle, angle2), torque2),
)
#let solution = solve(equations, (stiffness, offset))
#let fit = solution.branches.first().values

$ kappa = #to-typst(fit.at(0)) $\
$ tau_0 = #to-typst(fit.at(1)) $
>>>#for equation in equations {
>>>  let residual = replace(replace(equation, stiffness, fit.at(0)), offset, fit.at(1))
>>>  assert.eq(canonical(together(residual)), "0")
>>>}
```

`values` follows the variable order passed to `solve`: stiffness first, then
offset. These formulas require distinct values of the angle-dependent
coefficient; two identical readings cannot determine both parameters.

=== Evaluate at the measurements

For readings $(0.10,-0.4697)$ and $(0.20,-0.9545)$, evaluate both fitted
parameters in one call. `evaluate-many` returns a row per input point and a
complex value for each expression; `.re` reads the real part.

```worked
>>>#let angle = literal($theta$)
>>>#let stiffness = literal($kappa$)
>>>#let offset = literal($tau_0$)
>>>#let potential = parse($#stiffness (1 - cos(#angle))$)
>>>#let torque = neg(derivative(potential, angle))
>>>#let model = add(series(torque, angle, 0, 3), offset)
>>>#let angle1 = literal($theta_1$)
>>>#let angle2 = literal($theta_2$)
>>>#let torque1 = literal($tau_1$)
>>>#let torque2 = literal($tau_2$)
>>>#let equations = (subtract(replace(model, angle, angle1), torque1), subtract(replace(model, angle, angle2), torque2))
>>>#let fit = solve(equations, (stiffness, offset)).branches.first().values
#let fitted = evaluate-many(
  fit, (angle1, angle2, torque1, torque2),
  ((0.10, 0.20, -0.4697, -0.9545),),
).first()
#let (k, b) = fitted.map(value => value.re)
#let predictions = evaluate-many(
  model, (angle, stiffness, offset),
  ((0.10, k, b), (0.20, k, b)),
)

$ kappa approx #calc.round(k, digits: 6) $\
$ tau_0 approx #calc.round(b, digits: 6) $\
Predicted torques:
#predictions.map(row =>
  str(calc.round(row.first().re, digits: 4))
).join(", ")
>>>#for (row, observed) in predictions.zip((-0.4697, -0.9545)) {
>>>  assert(calc.abs(row.first().re - observed) < 1e-12)
>>>}
```

The predictions reproduce the two readings. More measurements would let us
test whether the cubic approximation describes the data well.

== Rewrite a trigonometric identity <rewrite-guide>

To use $sin^2(a)+cos^2(a)=1$ with any argument, represent $a$ by a wildcard.
Both occurrences must capture the same expression. This is the repeated
wildcard rule described in Symbolica's
#link("https://symbolica.io/docs/pattern_matching.html")[pattern-matching guide].

```worked
#let sine = function-head("sin", namespace: "symbolica")
#let cosine = function-head("cos", namespace: "symbolica")
#let a = wild("a")
#let pattern = parse($#sine(a)^2 + #cosine(a)^2$)
#let source = parse(
  $3 (#sine("x")^2 + #cosine("x")^2)
   + #sine($x+y$)^2 + #cosine($x+y$)^2$
)
#let result = replace(source, pattern, 1, repeat: true)

$ #to-typst(source) = #to-typst(result) $
>>>#assert.eq(canonical(subtract(result, 4)), "0")
```

The two matches capture $x$ and $x+y$. `repeat: true` runs further passes until
nothing changes. Use it only for rules that terminate: a rule that changes
$a$ into $a+1$ would run forever.

Matching follows the expression's structure. For example, $x^2$ occurs in
`expand(parse($(x+1)^2$))`, but is not a subexpression of the unexpanded square.

== Cancel factors and take partial fractions

This rational function contains a removable factor $s+3$. After cancellation,
`apart` separates it into terms with simple poles. `together` combines those
terms over a common denominator.

```worked
#let s = literal("s")
#let response = parse(
  $((s+3)(2s+5)) / (s^3+6s^2+11s+6)$
)
#let reduced = cancel-factors(response)
#let fractions = apart(reduced, s)
#let residual = together(subtract(fractions, reduced))

$ H(s) &= #to-typst(reduced) \
       &= #to-typst(fractions) $\
Difference: $#to-typst(residual)$
>>>#assert.eq(canonical(residual), "0")
```

The remaining poles are $s=-1$ and $s=-2$. Cancellation does not restore the
original function's value at $s=-3$: that input was excluded by its denominator.

== Find the intersections of a circle and a line <solving-guide>

The equations $x^2+y^2=25$ and $x-y=1$ have two intersections. Write each with
zero on the right and ask `solve` for all real branches.

```worked
#let x = literal("x")
#let y = literal("y")
#let system = (
  parse($x^2 + y^2 - 25$),
  parse($x - y - 1$),
)
#let solutions = solve(system, (x, y), domain: "real")

#for branch in solutions.branches [
  $ x = #to-typst(branch.values.at(0)),
    quad y = #to-typst(branch.values.at(1)) $\
]
>>>#assert(solutions.branches.len() == 2)
>>>#for branch in solutions.branches {
>>>  for equation in system {
>>>    assert.eq(canonical(expand(replace(replace(equation, x, branch.values.at(0)), y, branch.values.at(1)))), "0")
>>>  }
>>>}
```

A numerical solve instead starts from one point and searches for a root.
These two starting points converge to different intersections.

```worked
>>>#let x = literal("x")
>>>#let y = literal("y")
>>>#let system = (parse($x^2 + y^2 - 25$), parse($x - y - 1$))
#let roots = ((3.0, 3.0), (-3.0, -3.0)).map(start =>
  nsolve-system(system, (x, y), start, prec: 1e-10)
)
#let residuals = evaluate-many(system, (x, y), roots)

#for root in roots [
  $#calc.round(root.at(0), digits: 6),
    quad #calc.round(root.at(1), digits: 6)$\
]
>>>#for row in residuals {
>>>  for value in row { assert(calc.abs(value.re) < 1e-8) }
>>>}
```

`residuals` evaluates both equations at each root; its entries should be close
to zero. Unlike the exact solve above, a numerical solve does not enumerate
all solutions, and another starting point may fail to converge.

== Interpolate with a matrix

Find the quadratic $p(t)=a_0+a_1 t+a_2 t^2$ through $(0,1)$, $(1,3)$, and
$(2,8)$. Each point supplies a row of $A a=b$. `matrix-solve` keeps the
coefficients exact.

```worked
#let A = matrix(((1, 0, 0), (1, 1, 1), (1, 2, 4)))
#let b = vector((1, 3, 8))
#let coefficients = matrix-solve(A, b)
#let (a0, a1, a2) = range(3).map(i =>
  to-typst(matrix-at(coefficients, i, 0))
)
#let polynomial = parse($#a0 + #a1 t + #a2 t^2$)

$ a = #to-typst(coefficients) $\
$ p(t) = #to-typst(polynomial) $
>>>#assert(matrix-is-zero(matrix-sub(matrix-mul(A, coefficients), b)))
>>>#assert(canonical(determinant(A)) != "0")
```

`matrix-at` uses zero-based row and column indices. Multiplying $A a$ returns
$b$, and the nonzero determinant confirms that this interpolant is unique.
For larger symbolic systems, Symbolica's
#link("https://symbolica.io/docs/matrices.html")[matrix documentation]
describes the underlying exact arithmetic.

== Sample a surface and its gradient <sampling-guide>

For $f(x,y)=x^2+x y+y^2$, differentiate first, then evaluate the height and
both gradient components over a grid. This avoids approximating derivatives
from sampled values.

```worked
#let x = literal("x")
#let y = literal("y")
#let f = parse($x^2 + x y + y^2$)
#let fx = derivative(f, x)
#let fy = derivative(f, y)
#let grid = evaluate-grid(
  (f, fx, fy), (x, y),
  (domain(-1, 1, samples: 3), domain(-1, 1, samples: 3)),
)

$ partial_x f = #to-typst(fx) $\
$ partial_y f = #to-typst(fy) $
>>>#let cells = range(grid.points.len()).map(index => {
>>>  let point = grid.points.at(index)
>>>  let values = grid.values.at(index)
>>>  ([#point.at(0)], [#point.at(1)], [#values.at(0).re], [#values.at(1).re], [#values.at(2).re])
>>>}).flatten()
>>>#table(
>>>  columns: 5, inset: 4pt, stroke: 0.35pt + rgb("#d5dbe1"),
>>>  table.header([$x$], [$y$], [$f$], [$partial_x f$], [$partial_y f$]),
>>>  ..cells,
>>>)
>>>#assert(grid.points.len() == 9)
>>>#let origin = grid.points.position(point => point == (0.0, 0.0))
>>>#assert(grid.values.at(origin).all(value => value.re == 0.0))
```

Each row of `grid.values` corresponds to the same row of `grid.points`, with
values ordered as `(f, fx, fy)`. The last axis varies fastest. For an irregular
set of points, pass those points to `evaluate-many` instead.

#pagebreak()

= Symbolic integration <integration-guide>

Symbolica uses Rubi's integration rules to find antiderivatives. You can pass
the same expressions and variables used for algebra and differentiation.

== Compute an antiderivative

Find an antiderivative with `integrate(expression, variable)`. Here we
integrate $x/(x+1)$ and check the answer by differentiating it:

```worked
#let x = literal("x")
#let f = parse($x / (x + 1)$)
#let primitive = integrate(f, x)
#let check = together(subtract(derivative(primitive, x), f))

$ integral #to-typst(f) dif x = #to-typst(primitive) + C $\
$ F'(x) - f(x) = #to-typst(check) $
>>>#assert.eq(canonical(check), "0")
```

The integration variable must be a single symbol. Add the integration
constant $C$ when displaying an indefinite integral; it is not included in
the returned expression. Preserve any domain restrictions in your derivation.

#callout(
  [First integration call],
  [
    The first call to `integrate` or `integrate-with-steps` compiles and
    initializes the rule sets, which may take about 10 seconds. Both functions
    share the cached rules, so later calls and ordinary edits reuse them.
    Restarting the compiler or clearing its cache repeats this setup.
  ],
)

== Inspect a worked rule trace

Use `integrate-with-steps` when you want to show how Rubi transformed the
integral. `result` holds the antiderivative and `steps` holds the sequence of
rule applications. The example below displays each transformation and indents
the integrals introduced by earlier steps:

```worked
#let x = parse($x$)
#let f = parse($x / (x + 1)$)
#let explanation = integrate-with-steps(f, x)
#let step-notation = notation(
  calls: (
    "symbolica_integrate::rubi_int": ctx => {
      let (body, variable) = ctx.visual-arguments
      $ integral #body dif #variable $
    },
  ),
)
#let show-step(expression) = to-typst(
  expression, notation: step-notation,
)

$ integral #to-typst(f) dif x = #to-typst(explanation.result) + C $

#for step in explanation.steps [
  #h(step.depth * 1.15em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.15em)
  $#show-step(step.input) = #show-step(step.output)$
  #linebreak()
]
```

Check `complete` before treating the result as a finished antiderivative.
If it is `false`, some integrals remain unevaluated. Each step's `input` and
`output` is a symbolic expression that you can display with `to-typst` or use
in another calculation.

== Step fields

Each step contains:

- `rule`: the Rubi rule number, or `none` for an auxiliary transformation;
- `depth`: nesting depth of the rewritten integral;
- `description`, `references`, and `source`: Rubi's explanatory metadata; and
- `input` and `output`: portable Atom payload bytes for the immediate rewrite.

Steps run from an outer transformation into its nested integrals. Use `depth`
to show that nesting and `rule` to identify the Rubi rule applied.

= Limitations <limitations>

The plugin covers the operations in this manual. When a calculation needs
more, Symbolica's Python and Rust interfaces expose additional algorithms
and configuration options. In Typst, keep these limits in mind:

- Integration returns a best-effort antiderivative without an integration
  constant. Unsupported parts remain unevaluated; inspect `complete` in
  `integrate-with-steps` to check whether the entire expression was integrated.

- Exact system solving is intended for linear and polynomial equations.
  Numerical solving needs a starting point. Check the answer by substituting
  it into the original equations.

- Decimal literals remain floating-point values. A current upstream Wasm bug
  can mis-evaluate them inside analytic functions. Write exact fractions for
  those inputs—for example, `cos(1/2)`—before numerical evaluation.

- `to-typst` can omit digits or misplace the decimal point in floating-point
  results. Use `evaluate` and display its `re` and `im` fields, or use
  `canonical` for a text representation of an approximate expression.

- Matrix entries must be rational-polynomial expressions, and their dimensions
  must agree for the requested operation.

- Algebraic transformations do not keep a separate list of assumptions. If a
  manipulation is valid only under a condition such as
  $x != 1$, preserve that condition in the surrounding document.

- Some unusual Typst math structures may not parse. `array-tree` can help show
  what the parser received.

== Troubleshooting

#table(
  columns: (1.35fr, 2.05fr, 2.2fr),
  inset: 5pt,
  stroke: 0.4pt + border,
  table.header([*Symptom*], [*Likely cause*], [*What to try*]),
  [`expected content, found bytes`],
  [A symbolic result was inserted directly into `$...$`.],
  [Display it with `to-typst`.],
  [A symbolic result fails in another API],
  [The bytes hold a matrix or came from an incompatible package version.],
  [Use the matching package version and keep matrix values in matrix APIs.],
  [A function cannot be evaluated numerically],
  [It has no numerical definition, or an argument still contains a free variable.],
  [Use a built-in constructor and supply all variable values to `evaluate`.],
  [An analytic function of a decimal gives an unexpected value],
  [Its floating-point argument encountered the current upstream Wasm bug.],
  [Use an exact fraction such as `cos(1/2)`, then call `evaluate`.],
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
  [Inspect `matrix-shape`, `determinant`, and the coefficient expressions.],
)

= API reference

This section lists all public functions, grouped by operation. It is
generated from the documentation in `lib.typ`.

#let reference-groups = (
  (
    title: [Parsing, symbols, and rendering],
    names: (
      "parse", "literal", "function-head", "wild", "array-tree", "canonical",
      "notation", "merge-notation", "to-typst-source", "to-typst", "to-latex", "to-float",
    ),
  ),
  (
    title: [Built-in special functions],
    names: ("gamma-function", "polygamma", "polylog", "zeta-function", "bessel-j", "bessel-y", "bessel-i", "bessel-k"),
  ),
  (
    title: [Algebra and calculus],
    names: (
      "simplify", "expand", "factor", "together", "cancel-factors", "apart",
      "collect", "coefficient", "coefficient-list", "summands",
      "indeterminates", "contains", "is-constant",
      "derivative", "series", "integrate", "integrate-with-steps",
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
      "matrix", "vector", "identity", "eye", "matrix-add", "matrix-sub",
      "matrix-mul", "matrix-div-scalar", "transpose", "determinant", "inv",
      "matrix-solve", "matrix-solve-any", "row-reduce", "augment",
      "split-col", "primitive-part", "matrix-content", "matrix-at",
      "matrix-shape", "matrix-is-zero", "matrix-is-diagonal",
      "matrix-derivative",
    ),
  ),
  (
    title: [Scalar constructors],
    names: ("add", "mul", "neg", "subtract", "divide", "pow"),
  ),
  (
    title: [Document helpers],
    names: ("logo",),
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

= Namespaces and parser grammars <engine-guide>

If you need separate groups of symbol names or a different way to read Typst
math, use `init()` to configure an engine. It returns a dictionary of functions
that share those settings. The directly imported functions use the default
settings.

== Keep symbol names separate <namespaces-guide>

Suppose two models both use $x$, but for different quantities. A namespace
adds a prefix to each symbol's internal name, such as `left::x` or `right::x`.
These are different variables, even though both normally display as $x$.
This is useful when combining calculations from different models or packages.

```worked
#let left = literal("x", namespace: "left")
#let right = literal("x", namespace: "right")
#let difference = subtract(left, right)

#raw(canonical(difference, namespaces: true))
>>>#assert(not is-constant(difference))
>>>#assert.eq(canonical(derivative(difference, left)), "1")
>>>#assert.eq(canonical(derivative(difference, right)), "-1")
```

Use `canonical(..., namespaces: true)` to inspect the full names. A namespace
is part of a symbol's mathematical identity.

The imported functions use `"typst"` by default. Pass `namespace` to `parse`,
`literal`, `function-head`, or `wild` for an individual call. For a calculation
that uses one namespace throughout, set it once:

```worked
#let model = init(namespace: "oscillator")
#let parse = model.parse
#let literal = model.literal
#let x = literal("x")
#let f = parse($x^2 + 1$)

$ f(x) = #to-typst(f), quad f'(x) = #to-typst(derivative(f, x)) $
>>>#assert.eq(canonical(derivative(f, x), namespaces: true), "2*oscillator::x")
```

Here `parse` and `literal` both create `oscillator::x`. The algebra functions accept
these expressions without changing their names. A per-call `namespace`
overrides the engine's default, and creating another engine with the same
namespace produces the same symbol identities.

In Typst math, a bare `$x$` does not read a `#let x` binding. The parser assigns
it the current namespace. Interpolating `$#x$` uses the bound value and preserves
its existing identity, even when the surrounding expression uses another
namespace.

`"symbolica"` is the namespace for built-in functions. Special-function
constructors such as `polylog` select it automatically. Their arguments keep
their own symbol identities. Symbolica's
#link("https://symbolica.io/docs/symbols.html#namespaces")[namespace documentation]
explains how the same names are used from Python and Rust.

== Define a parser grammar <grammar-guide>

A grammar tells the parser which operators to recognize and how to group
their arguments. For example, suppose you want $⊕$ to mean addition in your
input:

```worked
#let grammar = (
  add: (infix: $⊕$, prec: 1, assoc: true),
)
#let engine = init(grammar: grammar)
#let expression = (engine.parse)($x ⊕ y ⊕ x$)

$ #to-typst(expression) $
>>>#assert.eq(canonical(expression), canonical(parse($2 x + y$)))
>>>#assert.eq(canonical(parse($x ⊕ y$, grammar: grammar)), canonical(parse($x + y$)))
>>>#assert.eq(canonical((engine.parse)("x^2 + y")), canonical(parse($x^2 + y$)))
```

`infix` specifies the operator between arguments. `prec` sets its precedence
relative to other operators: higher values bind more tightly. `assoc: true`
collects a chain into one node. The rule name `add` tells the plugin to
interpret that node as addition.

A supplied grammar *replaces* the default content grammar. This small example
only defines $⊕$; it does not retain the package's rules for multiplication,
fractions, or function calls. Include rules for the syntax you want to accept.
To support both $+$ and $⊕$, use a second rule and rewrite its node to `add`:

```worked
#let grammar = (
  add: (infix: $+$, prec: 1, assoc: true),
  custom-plus: (
    infix: $⊕$, prec: 1, assoc: true,
    rewrite: node => { node.head = "add"; node },
  ),
)
#let expression = parse($x + y ⊕ x$, grammar: grammar)
$ #to-typst(expression) $
>>>#assert.eq(canonical(expression), canonical(parse($2 x + y$)))
```

Use `parse(..., grammar: grammar)` for one call or `init(grammar: grammar)`
to reuse it. `array-tree(..., grammar: grammar)` displays the parsed structure
and helps check how a formula was grouped.

The parser is #link("https://typst.app/universe/package/parsely/")[Parsely].
Its grammars support prefix and postfix operators, content patterns, guards,
and rewrite functions. The resulting nodes must use the structure expected
by the plugin: recognized heads such as `add` and `mul` perform algebra;
other heads become symbolic function calls with their positional arguments.
A new rule does not add a numerical implementation or differentiation rules
for a new function.

Grammars apply to Typst content only. `parse("x^2 + y")` always uses Symbolica's
string syntax, even on an engine with a custom grammar. Passing an explicit
`grammar` to any non-content input is an error. For changes to the displayed output,
use `notation` instead.

#pagebreak()

= Compatibility and licensing <licensing>

This manual describes the official Symbolica Typst plugin #package-version,
using Symbolica 3.0. The package is tested with Typst 0.14 or newer.

The official `symbolica` plugin is free for all use within Typst, including
academic and commercial work. Its
#link(repository + "/blob/main/LICENSE-SYMBOLICA-TYPST.md")[Typst permission]
requires no payment, registration, or license key.

The original plugin source code is released under the
#link(repository + "/blob/main/LICENSE")[MIT License]. The bundled WebAssembly
engine is included with redistribution permission. The underlying Symbolica
computer algebra system is governed by its own
#link("https://symbolica.io/license/")[license terms] outside this Typst usage
permission.

For source, issues, and release history, visit
#link(repository)[github.com/symbolica-dev/symbolica-typst-plugin].

= Acknowledgements

The algebra engine is #link("https://symbolica.io/")[Symbolica]. Integration
uses Symbolica Integrate and the Rubi ruleset. The explanations and examples
in this manual draw on the #link(symbolica-guide)[Symbolica documentation],
adapted to the Typst interface.

#link("https://typst.app/universe/package/parsely/")[Parsely] parses Typst math
content. #link("https://typst.app/universe/package/tidy/")[Tidy] generates the
manual's examples and API reference.

Thanks to the contributors to these projects.
