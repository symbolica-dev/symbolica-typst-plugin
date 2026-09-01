#import "../typst/lib.typ" as sym
#import "lib.typ": integrate, integrate-with-steps

#let manifest = toml("typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/lcnbr/tymbolica"
#let accent = rgb("#704b7c")
#let pale-accent = rgb("#f5eef8")
#let muted = rgb("#61616b")

#set document(title: "Tymbolica Rubi Manual", author: "Tymbolica contributors")
#set page(
  paper: "a4",
  margin: (x: 21mm, top: 20mm, bottom: 19mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(size: 8pt, fill: muted)
      grid(columns: (1fr, auto), [Tymbolica Rubi], [Version #package-version])
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
#set heading(numbering: "1.1")
#show heading: set text(fill: accent)
#show link: set text(fill: accent)
#show raw: set text(font: "DejaVu Sans Mono")

#align(center)[
  #v(19mm)
  #text(size: 32pt, weight: "bold", fill: accent)[Tymbolica Rubi]
  #v(4mm)
  #text(size: 15pt, fill: muted)[Symbolic integration with genuine Rubi steps]
  #v(12mm)
  #block(width: 78%, inset: 13pt, radius: 5pt, fill: pale-accent)[
    A focused companion to Tymbolica: pass in portable Symbolica Atom
    payloads, receive an antiderivative or the complete nested rule trace.
  ]
  #v(18mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[MIT license]
]

#pagebreak()

= Use Rubi from Tymbolica

Tymbolica Rubi is a separate package and WebAssembly engine. It deliberately
does not parse expressions, declare symbols, or print mathematics. Tymbolica
and compatible extensions create the versioned Atom payloads it consumes and
can continue working with the result.

Import the Rubi package explicitly alongside Tymbolica:

```typst
#import "@local/tymbolica:0.1.0" as sym
#import "@local/tymbolica-rubi:0.1.0": integrate, integrate-with-steps

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#let primitive = integrate(f, x)

$ integral #sym.to-typst(f) dif x = #sym.to-typst(primitive) + C $
```

Evaluating either integration function constructs the fixed bundled module
through a pure zero-argument function. There is no loaded flag or call-order
dependence: every new instance starts from the same Wizer-preinitialized Rubi
tables.

== A worked rule trace

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#let explanation = integrate-with-steps(f, x)
#let step-notation = sym.notation(
  calls: (
    "symbolica_integrate::rubi_int": ctx => {
      let (body, variable) = ctx.visual-arguments
      $ integral #body dif #variable $
    },
  ),
)
#let show-step(expression) = sym.to-typst(expression, notation: step-notation)

$ integral #sym.to-typst(f) dif x
  = #sym.to-typst(explanation.result) + C $

#for step in explanation.steps [
  #h(step.depth * 1.15em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.15em)
  $#show-step(step.input) = #show-step(step.output)$
  #linebreak()
]

`complete` is `true` when Rubi eliminated every integral. A partial result is
still returned when it is `false`. No integration constant is added. Symbol
namespaces, attributes, and portable attachments are retained in the result
and in each step payload.

= Public API

The package exports exactly two public functions.

#table(
  columns: (auto, 1fr),
  inset: 7pt,
  stroke: 0.4pt + rgb("#d7cfda"),
  table.header([*Function*], [*Result*]),
  [`integrate(expression, variable)`],
  [The best-effort antiderivative as portable Atom payload bytes.],
  [`integrate-with-steps(expression, variable)`],
  [A dictionary containing `result`, `complete`, and nested `steps`.],
)

Both arguments must be portable Tymbolica Atom payload bytes, and `variable`
must encode one symbol. Invalid input produces an error instead of being
reparsed from its printed form. Thus `math($x$)` is a valid variable, while the
annotated Typst content returned directly by `symbol("x")` must first be passed
through Tymbolica's `atom` function.

== Step fields

Each step contains:

- `rule`: the Rubi rule number, or `none` for an auxiliary transformation;
- `depth`: nesting depth of the rewritten integral;
- `description`, `references`, and `source`: Rubi's explanatory metadata; and
- `input` and `output`: portable Atom payload bytes for the immediate rewrite.

The array is ordered from the outer rewrite into nested integrals. Use the
`depth` field to indent it, group it, or build a custom explanation layout.

= Installation from this repository

Install `tymbolica` and `tymbolica-rubi` as independent local packages. On
Linux, expose the repository root as Tymbolica and its `rubi` directory as the
Rubi package:

```shell
mkdir -p ~/.local/share/typst/packages/local/tymbolica
mkdir -p ~/.local/share/typst/packages/local/tymbolica-rubi
ln -s /path/to/tymbolica \
  ~/.local/share/typst/packages/local/tymbolica/0.1.0
ln -s /path/to/tymbolica/rubi \
  ~/.local/share/typst/packages/local/tymbolica-rubi/0.1.0
```

The Rubi directory contains its own compressed Wizer-preinitialized engine and
its own inflater. Documents that use only Tymbolica do not construct this
engine.

Tymbolica Rubi uses Symbolica's integration interface and the Rubi ruleset.
Thanks to the Symbolica and Rubi contributors for making the engine possible.
