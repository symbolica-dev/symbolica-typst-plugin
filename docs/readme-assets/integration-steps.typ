// Rebuild with scripts/readme-examples.py --update.
// Palette: docs/manual-assets/README.md.
#import "@preview/symbolica:0.1.0": *

#let dark = sys.inputs.at("theme", default: "light") == "dark"
#let ink = if dark { rgb("#f2f3e9") } else { rgb("#18222f") }
#let card = if dark { rgb("#182f29") } else { white }

#set page(width: 240pt, height: 190pt, margin: 0pt, fill: none)
#set text(font: "DejaVu Sans", size: 9pt, fill: ink)
#set par(leading: 0.5em)
#let accent = if dark { rgb("#75d5b3") } else { rgb("#087f64") }
#let muted = if dark { rgb("#b2c4b8") } else { rgb("#445365") }
#let pale = if dark { rgb("#23483d") } else { rgb("#e9f5ef") }
#let border = if dark { rgb("#3b5b4b") } else { rgb("#d8e3ef") }
#place(top + left, rect(width: 240pt, height: 190pt, radius: 10pt,
  fill: card, stroke: 0.6pt + border))

#let x = literal("x")
#let f = parse($x / (x + 1)$)
#let explanation = integrate-with-steps(f, x)
#assert(explanation.complete)

// Large computed mathematics stays readable in a three-column README gallery.
#block(width: 240pt, height: 190pt, inset: 14pt)[
  #stack(
    dir: ttb,
    spacing: 0pt,
    text(size: 12pt, weight: "bold", fill: accent)[Integration with steps],
    v(18pt),
    block(width: 212pt, inset: (x: 6pt, y: 12pt), radius: 5pt, fill: pale)[
      #align(center, stack(
        dir: ttb,
        spacing: 16pt,
        text(size: 20pt)[$ integral #to-typst(f) dif x $],
        text(size: 18pt)[$ = #to-typst(explanation.result) + C $],
      ))
    ],
    v(16pt),
    align(center, text(size: 10pt, fill: muted)[Full rule trace in the example]),
  )
]
