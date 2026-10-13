#import "@preview/symbolica:0.1.0": *

// A mathematical workbench spilling beyond the banner.
// Supporting artwork is generated first by scripts/readme-examples.py.
// readme-artwork-dependency: algebraic-curves
// readme-artwork-dependency: optimization-valley
#let paper = rgb("#fff8ee")
#let ink = rgb("#2c4137")
#let green = rgb("#087f64")
#let coral = rgb("#bc6c57")
#let soft-green = rgb("#a0b3a0")
#let faint-ink = rgb("#c3b6a1")

#let fade(width, height) = rect(
  width: width, height: height, stroke: none,
  fill: gradient.radial(
    (paper, 0%), (paper, 58%), (paper.transparentize(100%), 100%),
    radius: 50%,
  ),
)

#set document(title: "Symbolica — Symbolica — Computer algebra in your document")
#set page(width: 800pt, height: 280pt, margin: 0pt, fill: paper)
#set text(font: "DejaVu Sans", size: 12pt, fill: ink)
#set par(leading: 0.65em)


// Real public names, deliberately jostled and cropped at the perimeter.
#let api-notes = (
  (x: -24pt, y: -4pt, angle: -7deg, size: 12pt, fill: green, name: "parse"),
  (x: 67pt, y: 12pt, angle: 5deg, size: 10pt, fill: faint-ink, name: "literal"),
  (x: 141pt, y: -3pt, angle: -3deg, size: 17pt, fill: green, name: "expand"),
  (x: 299pt, y: 8pt, angle: 4deg, size: 12pt, fill: coral, name: "integrate-with-steps"),
  (x: 594pt, y: -8pt, angle: 6deg, size: 15pt, fill: ink, name: "gamma-function"),
  (x: 729pt, y: 38pt, angle: 12deg, size: 15pt, fill: green, name: "bessel-j"),
  (x: -18pt, y: 81pt, angle: -8deg, size: 12pt, fill: coral, name: "replace-wildcards"),
  (x: 17pt, y: 112pt, angle: 5deg, size: 12pt, fill: green, name: "apart"),
  (x: -23pt, y: 141pt, angle: -4deg, size: 13pt, fill: ink, name: "cancel-factors"),
  (x: 77pt, y: 168pt, angle: 9deg, size: 12pt, fill: soft-green, name: "coefficient-list"),
  (x: -10pt, y: 197pt, angle: -6deg, size: 16pt, fill: green, name: "determinant"),
  (x: 32pt, y: 247pt, angle: 7deg, size: 18pt, fill: coral, name: "integrate"),
  (x: 192pt, y: 255pt, angle: -3deg, size: 13pt, fill: ink, name: "row-reduce"),
  (x: 352pt, y: 268pt, angle: 7deg, size: 11pt, fill: soft-green, name: "matrix-solve-any"),
  (x: 483pt, y: 263pt, angle: -5deg, size: 15pt, fill: green, name: "matrix-derivative"),
  (x: 622pt, y: 77pt, angle: 7deg, size: 13pt, fill: green, name: "evaluate-many"),
  (x: 692pt, y: 109pt, angle: -9deg, size: 18pt, fill: coral, name: "solve"),
  (x: 649pt, y: 139pt, angle: 4deg, size: 12pt, fill: ink, name: "nsolve-system"),
  (x: 726pt, y: 172pt, angle: 11deg, size: 15pt, fill: green, name: "polylog"),
  (x: 528pt, y: 235pt, angle: -9deg, size: 12pt, fill: coral, name: "evaluate-grid"),
  (x: 225pt, y: 29pt, angle: 1deg, size: 9pt, fill: soft-green, name: "to-latex"),
  (x: 451pt, y: 28pt, angle: -5deg, size: 10pt, fill: faint-ink, name: "replace-multiple"),
  (x: 107pt, y: 215pt, angle: 5deg, size: 10pt, fill: faint-ink, name: "function-head"),
  (x: 363pt, y: 275pt, angle: 0deg, size: 10pt, fill: faint-ink, name: "primitive-part"),
  (x: 677pt, y: 270pt, angle: 6deg, size: 14pt, fill: ink, name: "zeta-function"),
  (x: 52pt, y: 59pt, angle: 7deg, size: 10pt, fill: soft-green, name: "factor"),
  (x: 475pt, y: 16pt, angle: -6deg, size: 11pt, fill: green, name: "together"),
  (x: 11pt, y: 178pt, angle: -4deg, size: 12pt, fill: coral, name: "derivative"),
  (x: 134pt, y: 241pt, angle: -4deg, size: 10pt, fill: soft-green, name: "transpose"),
  (x: 696pt, y: 18pt, angle: 4deg, size: 10pt, fill: soft-green, name: "inv"),
)
#for note in api-notes {
  place(top + left, dx: note.x, dy: note.y,
    rotate(note.angle, text(font: "DejaVu Sans Mono", size: note.size,
      fill: note.fill, note.name + "(...)")))
}

// A few genuine computations mixed into the loose notes.
#let x = literal("x")
#let polynomial = parse($x^6 - 1$)
#let factored = factor(polynomial)
#let sine-squared = parse($sin(x^2)$)
#let differentiated = derivative(sine-squared, x)
#let taylor = series(parse($sin(x)$), x, 0, 7)
#let fixed-point = nsolve(parse($cos(x) - x$), x, 0.7, prec: 1e-13)
#let A = matrix($mat(2, 1; 1, -1)$)
#let determinant-value = determinant(A)

#place(top + left, dx: 9pt, dy: 43pt,
  rotate(-6deg, text(size: 13pt, fill: ink)[$ x^6 - 1 = #to-typst(factored) $]))
#place(top + left, dx: 539pt, dy: 33pt,
  rotate(5deg, text(size: 16pt, fill: green)[$ #to-typst(A) $]))
#place(top + left, dx: 203pt, dy: 226pt,
  rotate(4deg, text(size: 12pt, fill: green)[$ sin(x) = #to-typst(taylor) + O(x^9) $]))

#place(top + left, dx: 14pt, dy: 122pt,
  rotate(6deg, text(size: 8.5pt, fill: faint-ink)[
    #raw("#let f = parse(...)\nto-typst(factor(f))", block: true)
  ]))
#place(top + left, dx: 410pt, dy: 263pt,
  rotate(4deg, text(size: 8.5pt, fill: faint-ink)[
    #raw("series(...)  rule(...)\ncollect(...)  inspect(...)", block: true)
  ]))

// Both researched plots are transparent; radial paper halos blend with the field.
#place(top + left, dx: -42pt, dy: 69pt, fade(251pt, 236pt))
#place(top + left, dx: -38pt, dy: 93pt, image("algebraic-curves.svg", width: 233pt))
#place(top + left, dx: 586pt, dy: 83pt, fade(250pt, 197pt))
#place(top + left, dx: 618pt, dy: 103pt, image("optimization-valley.svg", width: 188pt))

// Soft paper fades keep the message clear without rectangular edges.
#place(top + left, dx: 134pt, dy: 34pt, fade(536pt, 205pt))
#place(top + left, dx: 173pt, dy: 64pt,
  block(width: 454pt, inset: 12pt)[
    #align(center, stack(
      dir: ttb,
      spacing: 0pt,
      grid(
        columns: (32pt, auto),
        column-gutter: 10pt,
        align: horizon,
        logo(size: 32pt),
        text(size: 32pt, weight: "bold")[Symbolica],
      ),
      v(18pt),
      text(font: "Libertinus Serif", size: 27pt, fill: green)[Computer algebra in your document],
      v(18pt),
      text(size: 12pt, fill: ink)[Explore. Explain. Evaluate.],
    ))
  ])
