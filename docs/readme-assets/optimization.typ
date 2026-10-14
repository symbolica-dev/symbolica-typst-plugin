// Rebuild with scripts/readme-examples.py --update.
// Palette: docs/manual-assets/README.md.
// readme-artwork-dependency: optimization-valley
#import "@preview/symbolica:0.2.0": *

#let dark = sys.inputs.at("theme", default: "light") == "dark"
#let ink = if dark { rgb("#f2f3e9") } else { rgb("#18222f") }
#let card = if dark { rgb("#182f29") } else { white }

#set page(width: 240pt, height: 190pt, margin: 0pt, fill: none)
#set text(font: "DejaVu Sans", size: 9pt, fill: ink)
#let accent = if dark { rgb("#75d5b3") } else { rgb("#087f64") }
#let border = if dark { rgb("#3b5b4b") } else { rgb("#d8e3ef") }
#let x = literal("x")
#let y = literal("y")
#let F = parse($(1 - x)^2 + 100 (y - x^2)^2$)
#let stationary = solve((derivative(F,x),derivative(F,y)),(x,y),domain:"real")
#assert.eq(stationary.coverage,"complete")
#assert.eq(stationary.branches.len(),1)
#let minimum = stationary.branches.first().values
#assert.eq(minimum.map(canonical),("1","1"))

#place(top + left, rect(width: 240pt, height: 190pt, radius: 10pt,
  fill: card, stroke: 0.6pt + border))
#place(top + left, dx: 14pt, dy: 13pt,
  text(size: 12pt, weight: "bold", fill: accent)[Find the minimum])
#place(top + left, dx: 25pt, dy: 31pt,
  image("optimization-valley.svg", width: 190pt))
#place(top + left, dx: 0pt, dy: 173pt, block(width: 240pt)[
  #align(center)[#text(size: 10.5pt, fill: accent)[Minimum $(#to-typst(minimum.at(0)),#to-typst(minimum.at(1)))$]]
])
