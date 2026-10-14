// Supporting art for the Universe README banner.
#import "@preview/symbolica:0.2.0": *
#let dark = sys.inputs.at("theme", default: "light") == "dark"
#let ink = if dark { rgb("#f2f3e9") } else { rgb("#2c4137") }

// A family of real cubic curves: y^2 = x^3 - x + c.
// Mathematical reference: https://mathworld.wolfram.com/EllipticCurve.html
// At c = +/-2/(3 sqrt(3)), two real roots coalesce; those singular members
// are not elliptic curves. The plotted family includes both topologies.
// Every contour below comes from a Symbolica-evaluated polynomial field.
#set document(title: "A family of real algebraic cubic curves")
#set page(width: 260pt, height: 220pt, margin: 0pt, fill: none)
#set text(font: "DejaVu Sans", size: 12pt, fill: ink)
#set par(leading: 0.65em)

#let x = literal("x")
#let y = literal("y")
#let polynomial = expand(parse($y^2 - x^3 + x$))
#let x-min = -1.65
#let x-max = 1.9
#let y-min = -1.5
#let y-max = 1.5
#let nx = 161
#let ny = 121
#let field = evaluate-grid(polynomial, (x, y), (
  domain(x-min, x-max, samples: nx),
  domain(y-min, y-max, samples: ny),
))
#assert.eq(field.shape, (nx, ny))
#let samples = field.values.map(row => row.first().re)

#let plot-width = 260.0
#let plot-height = 220.0
#let dx = plot-width / (nx - 1)
#let dy = plot-height / (ny - 1)
#let point(ix, iy) = (ix * dx, plot-height - iy * dy)
#let number(value) = str(calc.round(value, digits: 3))
#let coord(p) = number(p.at(0)) + " " + number(p.at(1))
#let crossing(a, b, va, vb, level) = {
  let t = (level - va) / (vb - va)
  (a.at(0) + t * (b.at(0) - a.at(0)), a.at(1) + t * (b.at(1) - a.at(1)))
}

// A regular-grid marching-squares renderer is ordinary Typst code.
// The ambiguous diagonal case uses the bilinear center's field value.
#let faded-segment(a, b, paint, width) = {
  let mx = (a.at(0) + b.at(0)) / 2
  let my = (a.at(1) + b.at(1)) / 2
  let fade = calc.max(0, calc.min(1,
    mx / (0.1 * plot-width), (plot-width - mx) / (0.1 * plot-width),
    my / (0.1 * plot-height), (plot-height - my) / (0.1 * plot-height)))
  let opacity = calc.pow(calc.sin(fade * 90deg), 2)
  ("<path d='M " + coord(a) + " L " + coord(b) + "' fill='none' stroke='"
    + paint + "' stroke-width='" + str(width) + "' stroke-opacity='"
    + number(opacity) + "' stroke-linecap='round'/>")
}
#let contour(level, paint, width) = {
  let segments = ()
  for ix in range(nx - 1) {
    for iy in range(ny - 1) {
      let points = (point(ix, iy), point(ix + 1, iy), point(ix + 1, iy + 1), point(ix, iy + 1))
      let values = (
        samples.at(ix * ny + iy),
        samples.at((ix + 1) * ny + iy),
        samples.at((ix + 1) * ny + iy + 1),
        samples.at(ix * ny + iy + 1),
      )
      let crossings = ()
      for edge in range(4) {
        let next = calc.rem(edge + 1, 4)
        let va = values.at(edge)
        let vb = values.at(next)
        if (va < level) != (vb < level) {
          crossings.push((edge, crossing(points.at(edge), points.at(next), va, vb, level)))
        }
      }
      if crossings.len() == 2 {
        segments.push(faded-segment(crossings.at(0).at(1), crossings.at(1).at(1), paint, width))
      } else if crossings.len() == 4 {
        let center = values.sum() / 4
        let pair-01 = (values.at(0) < level) == (center < level)
        let pairs = if pair-01 { ((0, 1), (2, 3)) } else { ((0, 3), (1, 2)) }
        for pair in pairs {
          segments.push(faded-segment(crossings.at(pair.at(0)).at(1),
            crossings.at(pair.at(1)).at(1), paint, width))
        }
      }
    }
  }
  segments.join(" ")
}

#let paints = if dark {
  ("#c2addf", "#ae98cf", "#91c8b8", "#7ccdb0", "#75d5b3",
    "#e19a80", "#dfb28d", "#d8ca9a", "#b1d1ba")
} else {
  ("#9b8fb2", "#81729f", "#609a89", "#289375", "#087f64",
    "#bb6d58", "#c28c64", "#b6a173", "#95b0a1")
}
#let curves = (
  (c: -0.55, paint: paints.at(0), width: 0.9),
  (c: -0.4, paint: paints.at(1), width: 1.0),
  (c: -0.25, paint: paints.at(2), width: 1.0),
  (c: -0.1, paint: paints.at(3), width: 1.0),
  (c: 0.0, paint: paints.at(4), width: 1.3),
  (c: 0.1, paint: paints.at(5), width: 1.1),
  (c: 0.25, paint: paints.at(6), width: 1.0),
  (c: 0.4, paint: paints.at(7), width: 1.0),
  (c: 0.55, paint: paints.at(8), width: 0.9),
)
#let paths = curves.map(curve => contour(curve.c, curve.paint, curve.width))
#let svg = ("<svg xmlns='http://www.w3.org/2000/svg' width='" + str(plot-width)
  + "' height='" + str(plot-height) + "' viewBox='0 0 " + str(plot-width)
  + " " + str(plot-height) + "'>" + paths.join("") + "</svg>")

#image(bytes(svg), width: plot-width * 1pt, height: plot-height * 1pt)
