#import "lib.typ" as sym

#let manifest = toml("../typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/symbolica-dev/symbolica-typst-plugin"
#let accent = rgb("#704b7c")
#let pale-accent = rgb("#f5eef8")
#let muted = rgb("#61616b")

#set document(title: "Symbolica Integration Guide", author: "Symbolica contributors")
#set page(
  paper: "a4",
  margin: (x: 21mm, top: 20mm, bottom: 19mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(size: 8pt, fill: muted)
      grid(columns: (1fr, auto), [Symbolica Integration], [Version #package-version])
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
  #text(size: 32pt, weight: "bold", fill: accent)[Symbolica Integration]
  #v(4mm)
  #text(size: 15pt, fill: muted)[Symbolic integration with genuine Rubi steps]
  #v(12mm)
  #block(width: 78%, inset: 13pt, radius: 5pt, fill: pale-accent)[
    Symbolic integration within Symbolica: compute an antiderivative
    or inspect the complete nested Rubi rule trace.

    The official integration plugin, powered by Symbolica 3.0.
    Free for any use within Typst. No license or license key needed.
  ]
  #v(18mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[MIT license]
]

#pagebreak()

= Use Rubi from Symbolica

Symbolica includes Rubi integration alongside parsing, algebra, differentiation,
and rendering in one package and one WebAssembly engine. Integration accepts
the same expression values and symbol handles as the rest of the API.

It is free to use for any use within Typst, including academic and commercial
work. No Symbolica license or license key is needed.

Import Symbolica once to use algebra and integration together:

```typst
#import "@preview/symbolica:0.1.0" as sym

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#let primitive = sym.integrate(f, x)

$ integral #sym.to-typst(f) dif x = #sym.to-typst(primitive) + C $
```

The first call to `integrate` or `integrate-with-steps` compiles and initializes
the integration rule sets, which may take about 10 seconds. Both functions share
the cached rules, so later calls and ordinary edits reuse them. Restarting the
compiler or clearing its cache repeats this setup.

== A worked rule trace

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#let explanation = sym.integrate-with-steps(f, x)
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

This guide covers the two integration functions in the Symbolica API.

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

The expression may be an Atom payload or another supported expression value.
The integration variable must represent one symbol. Both `math($x$)` and
`symbol("x")` are accepted directly. Use `init(namespace: "model")` when your
integration and other algebra operations need a shared custom namespace.

== Step fields

Each step contains:

- `rule`: the Rubi rule number, or `none` for an auxiliary transformation;
- `depth`: nesting depth of the rewritten integral;
- `description`, `references`, and `source`: Rubi's explanatory metadata; and
- `input` and `output`: portable Atom payload bytes for the immediate rewrite.

The array is ordered from the outer rewrite into nested integrals. Use the
`depth` field to indent it, group it, or build a custom explanation layout.

= Development installation

For development before publication, expose the repository root as one local
package and use `@local` in place of `@preview`. On Linux:

```shell
mkdir -p ~/.local/share/typst/packages/local/symbolica
ln -s /path/to/symbolica-typst-plugin \
  ~/.local/share/typst/packages/local/symbolica/0.1.0
```

The package contains one uncompressed `symbolica.wasm`, loaded directly by
Typst. Package downloads use archive compression. Rule tables are constructed
on first integration use; algebra-only documents do not prepare them.

Symbolica Integrate uses Symbolica's integration interface and the Rubi ruleset.
Thanks to the Symbolica and Rubi contributors for making the engine possible.
