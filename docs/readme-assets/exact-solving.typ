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
#let y = literal("y")
#let system = (parse($x^2 + y^2 - 5$), parse($x y - 2$))
#let solutions = solve(system, (x, y), domain: "real")
#assert.eq(solutions.branches.len(), 4)
#let pairs = solutions.branches.map(branch => {
  let values = branch.values
  $(#to-typst(values.at(0)), #to-typst(values.at(1)))$
})

#place(top + left, dx: 14pt, dy: 13pt,
  text(size: 12pt, weight: "bold", fill: accent)[Solve exactly])
#place(top + left, dx: 14pt, dy: 32pt,
  text(size: 8pt, fill: muted)[Circle meets hyperbola. Four real solutions.])
#place(top + left, dx: 14pt, dy: 50pt,
  block(width: 212pt, inset: 9pt, radius: 5pt, fill: pale)[
    #set text(size: 15pt)
    #align(center)[$x^2 + y^2 = 5 quad x y = 2$]
  ])
#place(top + left, dx: 14pt, dy: 96pt, [
  #text(size: 8pt, fill: muted)[Exact solution pairs $(x,y)$]
  #v(8pt)
  #set text(size: 16pt)
  #grid(columns: (94pt, 94pt), column-gutter: 8pt, row-gutter: 7pt,
    align: center, ..pairs)
])
#place(top + left, dx: 14pt, dy: 174pt,
  text(size: 8pt, fill: muted)[Every pair returned by the symbolic solver.])
