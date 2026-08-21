#import "@preview/tidy:0.4.3"
#import "lib.typ" as tydenso

#let manifest = toml("typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/lcnbr/tymbolica"
#let spenso-guide = "https://symbolica.io/docs/python_api/community/spenso/index.html"
#let idenso-guide = "https://symbolica.io/docs/python_api/community/idenso/index.html"
#let accent = rgb("#704b7c")
#let pale-accent = rgb("#f5eef8")
#let muted = rgb("#61616b")

#set document(title: "Tydenso Manual", author: "Tydenso contributors")
#set page(
  paper: "a4",
  margin: (x: 20mm, top: 19mm, bottom: 18mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(size: 8pt, fill: muted)
      grid(columns: (1fr, auto), [Tydenso], [Version #package-version])
      line(length: 100%, stroke: 0.35pt + rgb("#d7cfda"))
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
#show link: set text(fill: accent)
#show raw: set text(font: "DejaVu Sans Mono")

#let callout(title, body) = block(
  width: 100%,
  breakable: true,
  inset: 9pt,
  radius: 3pt,
  fill: pale-accent,
  stroke: (left: 2.2pt + accent),
)[
  #text(weight: "bold", fill: accent)[#title]
  #body
]

#let example-preamble = "#import tydenso: *\n"
#let docs = tidy.parse-module(
  read("lib.typ"),
  name: "tydenso",
  scope: (tydenso: tydenso),
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
  scope: (tydenso: tydenso),
  preamble: example-preamble,
  mode: "markup",
  dir: ttb,
  scale-preview: 100%,
)
#show raw.where(lang: "worked"): worked-example

#align(center)[
  #v(25mm)
  #text(size: 35pt, weight: "bold", fill: accent)[Tydenso]
  #v(5mm)
  #text(size: 16pt, fill: muted)[Symbolic tensor algebra inside Typst]
  #v(13mm)
  #text(size: 11pt)[User manual · version #package-version]
  #v(23mm)
  #block(width: 76%, inset: 14pt, radius: 5pt, fill: pale-accent)[
    Build indexed tensors as readable Typst values, transform them with Idenso,
    and print them with Spenso's own notation.
  ]
  #v(34mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[MIT license] ·
  #link(spenso-guide)[Spenso Python API] ·
  #link(idenso-guide)[Idenso Python API]
]

#pagebreak()
#text(size: 22pt, weight: "bold", fill: accent)[Contents]
#v(5mm)
#outline(title: none, depth: 2, indent: auto)
#pagebreak()

= Start with a contraction

Tydenso is the tensor-focused companion to Tymbolica. It has its own package,
manual, printer, and WebAssembly engine. You do not need Tymbolica to construct,
simplify, inspect, or display a tensor expression.

The first example contracts a Minkowski metric with a vector. A representation
is an ordinary Typst dictionary; `slot` turns an index label into another
dictionary; `vector` creates a callable rank-one tensor name.

```worked
#let V = mink(4)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let p = vector("p")

#let before = math($#metric(V, mu, nu) #p(nu)$)
#let after = simplify-metrics(before)

$
  #to-typst(before)
  quad arrow.r.long
  #to-typst(after)
$
```

The constructor calls already produce readable math. Their hidden, versioned
metadata contains the exact Symbolica Atom, so `math` does not have to infer a
tensor from its appearance. It returns Atom bytes for algebra; `to-typst`
renders transformed results.

== Installation

Until Tydenso is published separately, install the `tydenso` directory from
this repository as its own local package. On Linux, place or symlink it at:

#raw(
  "~/.local/share/typst/packages/local/tydenso/" + package-version,
  lang: "text",
  block: true,
)

Then import it independently:

#raw(
  "#import \"@local/tydenso:" + package-version + "\": *",
  lang: "typ",
  block: true,
)

From a checkout, `#import "path/to/tymbolica/tydenso/lib.typ": *` works as
well. The package directory already contains its compressed engine and loader.

== How this corresponds to the Python API

The public concepts follow Spenso's Python surface, adapted to what is natural
in Typst. Python objects can overload calls; Typst dictionaries cannot, so
index construction is the explicit `slot(V, "mu")`. Tensor names are functions,
which keeps the pleasant `p(mu)` spelling.

#table(
  columns: (1fr, 1fr),
  inset: 7pt,
  stroke: 0.4pt + rgb("#d7cfda"),
  table.header([*Python*], [*Typst*]),
  [```python
V = Representation.mink(4)
mu = V("mu")
nu = V("nu")
p = TensorName("p")
expr = TensorName.g(mu, nu) * p(nu)
```],
  [```typst
#let V = mink(4)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let p = vector("p")
#let expr = math($#metric(V, mu, nu) #p(nu)$)
```],
)

Representations and slots stay transparent dictionaries. Symbols and tensor
calls are visible Typst math with exact Atom metadata; completed algebraic
expressions are Atom bytes.

= Build tensor expressions

== Representations and slots are data

Built-in constructors cover the representations initialized by Spenso and
Idenso: `mink`, `euc`, `lor`, `bis`, `spf`, `cof`, `coad`, and `cos`. Use
`representation` when a model introduces another representation.

A custom representation can name its automatic indices. The palette repeats;
each pass adds a subscript. Manually written indices use the same notation but
keep their own symbolic identity.

```worked
#let M = representation(
  "M",
  4,
  namespace: "example",
  self-dual: true,
  indices: ($mu$, $nu$),
)
#let T = tensor("T", namespace: "example")
#let expression = T(
  slot(M, 1),       // mu
  slot(M, 2),       // nu
  slot(M, 3),       // mu_1
  slot(M, $rho_2$), // exactly the index written here
)

#let detailed = print-settings(with-dim: true)
$ #to-typst(expression, settings: detailed) $
```

Here the qualified form distinguishes the indices as members of $M_4$. Leave
`indices` unset to keep the ordinary numeric display used by the built-in
representations.

```worked
#let color = cof(3)
#let a = slot(color, "a")
#let b-lower = slot(color, "b", dual: true)

#table(
  columns: (auto, 1fr),
  inset: 5pt,
  [representation], [#color.name],
  [dimension], [#color.dimension],
  [first index], [#a.index],
  [second index is dual], [#b-lower.dual],
)
```

Because those values are dictionaries, document code can validate dimensions,
generate index families in a loop, or attach its own metadata before asking the
plugin to construct an Atom. `dual-representation` returns the paired
representation; `metric`, `identity-tensor`, and `flat-tensor` accept either
index labels or already constructed slots.

== Tensor names carry symmetry

`tensor` mirrors the useful part of Spenso's `TensorName`: a name plus optional
symmetric, antisymmetric, cyclic, or linear behavior. Symbolica applies those
attributes while it constructs the function, not as a later cosmetic step.

```worked
#let V = euc(3)
#let mu-slot = slot(V, "mu")
#let nu-slot = slot(V, "nu")
#let F = tensor("F", antisymmetric: true)

#let cancellation = math($#F(mu-slot, nu-slot) + #F(nu-slot, mu-slot)$)
$ F^(mu nu) + F^(nu mu) = #to-typst(cancellation) $
```

Only one of `symmetric`, `antisymmetric`, and `cycle-symmetric` may be true for
one tensor name. The `linear` flag is independent.

== Write tensor algebra as math

Tensor and vector constructors are Typst functions. Interpolate their calls in
math with `#F(...)`; interpolate scalar symbols with `#mass`. Parsely reads the
ordinary arithmetic, while each annotated value contributes its exact Atom.

```worked
#let V = mink("D")
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let p = vector("p")
#let mass = symbol("m", namespace: "model")

#let shell = math($#metric(V, mu, nu) #p(mu) #p(nu) - #mass^2$)
$ #to-typst(shell) $
```

The structural `add`, `mul`, `neg`, `sub`, `div`, and `pow` functions remain
useful for generated expressions. They accept Atom bytes, annotated content,
numbers, slots, and representation dictionaries. Both paths construct the
same Atom; neither reconstructs tensors from printed subscripts.

== Build open and closed Spenso chains

The chain helpers construct Spenso's actual Atom heads; they are not drawing
commands. Compact endpoints are ordinary rank-one tensors carrying a
representation, while `gamma(mu)` supplies Spenso's `in` and `out`
placeholders for a chain factor.

```worked
#let M = mink(4)
#let B = bis(4)
#let mu = slot(M, "mu")
#let nu = slot(M, "nu")

#let p = vector("p")
#let q = vector("q")
#let u = vector("u")
#let v = vector("v")

#let scalar = dot(p(1, M), q(2, M))
#let open = chain(
  u(1, B),
  v(2, B),
  gamma(mu),
  gamma(p(1, M)),
  gamma(nu),
)
#let closed = trace(
  B,
  cyclic(gamma(mu), gamma(p(1, M)), gamma(nu)),
)

$ #to-typst(scalar) $
$ #to-typst(open) $
$ #to-typst(closed) $
```

Use `gamma(mu, a, b)` when the bispinor slots are explicit. `chain` keeps an
ordered open sequence; `cyclic` marks the factor list of a closed sequence;
and `trace` combines that cycle with its representation. The
#link(repository + "/blob/main/tydenso/examples/spenso-notation.typ")[standalone
notation example] also inspects the constructed trees and checks their exact
Spenso shapes.

= Transform tensors

Idenso provides the domain-specific transformations. The most common ones
simplify metrics, gamma matrices, and color structures; selective expanders and
index-wrapping functions are available for lower-level workflows.

== Contract a metric chain

The next expression contains two metrics and one vector. Simplifying metrics
eliminates both contracted dummy indices in one pass.

```worked
#let V = mink(4)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let rho = slot(V, "rho")
#let p = vector("p")

#let chain = mul(
  metric(V, mu, nu),
  metric(V, nu, rho),
  p(rho),
)
#let reduced = simplify-metrics(chain)

$
  #to-typst(chain)
  quad arrow.r.long
  #to-typst(reduced)
$
```

`list-dangling` returns the free indices as Atom payloads. Pass an element to
`to-typst`, `to-string`, or `inspect` just like any other expression.

== Know which family to call

#table(
  columns: (1.1fr, 2.2fr),
  inset: 6pt,
  stroke: 0.4pt + rgb("#d7cfda"),
  table.header([*Family*], [*Purpose*]),
  [`simplify-metrics`, `expand-metrics`, `expand-mink`, `expand-bis`],
  [Metric, Minkowski, and bispinor structure.],
  [`simplify-gamma`, `dirac-adjoint`],
  [Dirac chains and conjugation.],
  [`simplify-color`, `expand-color`],
  [Color generators and structure constants.],
  [`cook-function`, `cook-indices`, `wrap-dummies`, `wrap-indices`],
  [Canonical index organization and explicit wrappers.],
  [`to-dots`],
  [Rewrite supported contractions as dot products.],
)

The mathematical conventions and supported identities track
#link(idenso-guide)[Idenso's API].

= Print and inspect

== Use the Spenso printer directly

Tydenso does not send tensor output through Tymbolica's general printer.
`to-typst-source` uses Symbolica's Typst arithmetic mode together with Spenso's
custom tensor notation. `to-string` uses Spenso's compact Symbolica notation.

```worked
#let V = mink(4)
#let p = vector("p")
#let expression = p(1, slot(V, "mu"))

#let detailed = print-settings(with-dim: true, commas: true)

Rendered: $ #to-typst(expression, settings: detailed) $

Compact Spenso form:
#raw(to-string(expression), block: true)
```

Upper and lower indices align in matching columns. A plain self-dual slot is
placed on the top row; for a dualizable representation, its dual orientation
is placed on the bottom row.

`print-settings` exposes the real `SpensoPrintSettings` switches:
`with-dim`, `parens`, `commas`, `index-subscripts`, and `symbol-scripts`. Start
from either the `"typst"` or `"compact"` preset and override only what the
document needs.

== Inspect the Atom as CBOR data

Atom bytes remain the lossless transformation and interchange format. Before a
transformation, annotated constructor content carries those same bytes in
metadata. `inspect` accepts either form and decodes it to recursive Typst data.
Every node has a `kind`; function nodes also expose their full and short names,
arguments, and symmetry flags.

```worked
#let V = euc(3)
#let A = tensor("A", symmetric: true)
#let expression = A(slot(V, "i"), slot(V, "j"))
#let tree = inspect(expression)

#table(
  columns: (auto, 1fr),
  inset: 5pt,
  [node kind], [#tree.kind],
  [function], [#tree.short-name],
  [arguments], [#tree.arguments.len()],
  [symmetric], [#tree.symmetric],
)
```

#callout(
  [Why keep both forms?],
  [
    CBOR is convenient for layouts, diagnostics, and package-level APIs.
    Reconstructing algebra from that tree would lose Symbolica state and exact
    coefficient details, so transformations continue to consume Atom bytes.
  ],
)

= Work alongside Tymbolica

Tydenso and Tymbolica share the same Atom payload. A package can construct and
simplify tensors with Tydenso, then pass the result to Tymbolica for a general
algebraic operation. Custom representations keep the information Tydenso needs
to use them again, including their index palette.

```typst
#import "@local/tydenso:0.1.0" as tensors
#import "@local/tymbolica:0.1.0" as algebra

#let V = tensors.representation(
  "M",
  4,
  namespace: "model",
  self-dual: true,
  indices: ($mu$, $nu$),
)
#let p = tensors.vector("p")
#let expression = p(tensors.slot(V, 1))

#let expanded = algebra.expand(expression)
#tensors.to-typst(expanded)
```

Keep package versions aligned. Matrix payloads from Tymbolica are a different
format and are not Tydenso inputs.

= API reference

The groups below cover the imported top-level API. `init` returns the same
surface as a dictionary bound to a selected plugin module.

#let reference-groups = (
  (
    title: [Tensor construction],
    names: (
      "tensor", "vector", "symbol", "function", "representation", "mink", "euc", "lor", "bis",
      "spf", "cof", "coad", "cos", "slot", "metric", "identity-tensor",
      "flat-tensor", "dual-representation",
    ),
  ),
  (
    title: [Expression construction],
    names: ("math", "atom", "add", "mul", "neg", "sub", "div", "pow"),
  ),
  (
    title: [Spenso products and chains],
    names: ("dot", "gamma", "chain", "cyclic", "trace"),
  ),
  (
    title: [Printing and inspection],
    names: ("print-settings", "to-typst-source", "to-typst", "to-string", "inspect"),
  ),
  (
    title: [Metric and index transformations],
    names: (
      "cook-function", "cook-indices", "expand-bis", "expand-metrics",
      "expand-mink", "expand-mink-bis", "list-dangling", "simplify-metrics",
      "to-dots", "wrap-dummies", "wrap-indices",
    ),
  ),
  (
    title: [Dirac and color transformations],
    names: (
      "dirac-adjoint", "expand-color", "simplify-color", "simplify-gamma",
    ),
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

This manual describes Tydenso #package-version and Typst 0.14 or newer.
Tydenso's original source is released under the
#link(repository + "/blob/main/LICENSE")[MIT License]. The bundled engine also
contains Spenso, Idenso, and Symbolica; their own upstream terms continue to
apply. In particular, the MIT license for this interface does not relicense
Symbolica. Read #link("https://symbolica.io/license/")[Symbolica's license]
before redistributing or deploying the WebAssembly bundle.

= Acknowledgements

Tydenso would not exist without #link("https://symbolica.io/")[Symbolica], the
exact algebra engine beneath its Atom representation. Thank you to Symbolica's
contributors, and to the GammaLoop contributors for
#link(spenso-guide)[Spenso] and #link(idenso-guide)[Idenso], whose tensor model,
printers, and identities define this package's mathematics.

Thanks also to #link("https://typst.app/universe/package/tidy/")[Tidy], which
powers the worked examples and generated reference, and to
#link("https://typst.app/universe/package/parsely/")[Parsely], which parses
Typst math while preserving the semantic Atom metadata.
