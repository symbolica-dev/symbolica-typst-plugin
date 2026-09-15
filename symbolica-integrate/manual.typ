#import "../symbolica/lib.typ" as sym
#import "lib.typ": integrate, integrate-with-steps

#let manifest = toml("typst.toml")
#let package-version = manifest.package.version
#let repository = "https://github.com/symbolica-dev/symbolica-typst-plugin"
#let accent = rgb("#704b7c")
#let pale-accent = rgb("#f5eef8")
#let muted = rgb("#61616b")

#set document(title: "Symbolica Integrate Manual", author: "Symbolica contributors")
#set page(
  paper: "a4",
  margin: (x: 21mm, top: 20mm, bottom: 19mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(size: 8pt, fill: muted)
      grid(columns: (1fr, auto), [Symbolica Integrate], [Version #package-version])
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
  #text(size: 32pt, weight: "bold", fill: accent)[Symbolica Integrate]
  #v(4mm)
  #text(size: 15pt, fill: muted)[Symbolic integration with genuine Rubi steps]
  #v(12mm)
  #block(width: 78%, inset: 13pt, radius: 5pt, fill: pale-accent)[
    A focused companion to Symbolica: pass in portable Symbolica Atom
    payloads, receive an antiderivative or the complete nested rule trace.

    The official integration plugin, powered by Symbolica 3.0.
    Free for any use within Typst. No license or license key needed.
  ]
  #v(18mm)
  #link(repository)[Repository] ·
  #link(repository + "/blob/main/LICENSE")[MIT license]
]

#pagebreak()

= Use Rubi from Symbolica

Symbolica Integrate is a separate package and WebAssembly engine. It deliberately
does not parse expressions, declare symbols, or print mathematics. Symbolica
and compatible extensions create the versioned Atom payloads it consumes and
can continue working with the result.

It is free to use for any use within Typst, including academic and commercial
work. No Symbolica license or license key is needed.

Import the Rubi package explicitly alongside Symbolica:

```typst
#import "@local/symbolica:0.1.0" as sym
#import "@local/symbolica-integrate:0.1.0": integrate, integrate-with-steps

#let x = sym.math($x$)
#let f = sym.math($x / (x + 1)$)
#let primitive = integrate(f, x)

$ integral #sym.to-typst(f) dif x = #sym.to-typst(primitive) + C $
```

The first integration call prepares Rubi's rules through Typst's cached plugin
transition. Later calls and ordinary edits reuse the prepared module. Additional
instances receive its initialized memory snapshot. Restarting the compiler or
losing its cache requires preparation again.

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

Both arguments must be portable Symbolica Atom payload bytes, and `variable`
must encode one symbol. Invalid input produces an error instead of being
reparsed from its printed form. Thus `math($x$)` is a valid variable, while the
annotated Typst content returned directly by `symbol("x")` must first be passed
through Symbolica's `atom` function.

== Step fields

Each step contains:

- `rule`: the Rubi rule number, or `none` for an auxiliary transformation;
- `depth`: nesting depth of the rewritten integral;
- `description`, `references`, and `source`: Rubi's explanatory metadata; and
- `input` and `output`: portable Atom payload bytes for the immediate rewrite.

The array is ordered from the outer rewrite into nested integrals. Use the
`depth` field to indent it, group it, or build a custom explanation layout.

= Installation from this repository

Install `symbolica` and `symbolica-integrate` as independent local packages. On
Linux, expose the repository root as Symbolica and its `symbolica-integrate`
directory as the integration package:

```shell
mkdir -p ~/.local/share/typst/packages/local/symbolica
mkdir -p ~/.local/share/typst/packages/local/symbolica-integrate
ln -s /path/to/symbolica-typst-plugin \
  ~/.local/share/typst/packages/local/symbolica/0.1.0
ln -s /path/to/symbolica-typst-plugin/symbolica-integrate \
  ~/.local/share/typst/packages/local/symbolica-integrate/0.1.0
```

The integration directory contains its own compressed engine and inflater.
The rule tables are constructed at runtime instead of being stored in the
download. Documents that use only Symbolica do not construct this engine.

Symbolica Integrate uses Symbolica's integration interface and the Rubi ruleset.
Thanks to the Symbolica and Rubi contributors for making the engine possible.
