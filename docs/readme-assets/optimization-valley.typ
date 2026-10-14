// Supporting art for the Universe README banner.
// Rosenbrock's banana valley, evaluated and differentiated by Symbolica.
// Definition: https://docs.scipy.org/doc/scipy/reference/generated/scipy.optimize.rosen.html
// Contours and damped Newton/Armijo iteration are local Typst code.
#import "@preview/symbolica:0.2.0": *
#let dark = sys.inputs.at("theme", default: "light") == "dark"
#let ink = if dark { rgb("#f2f3e9") } else { rgb("#2c4137") }
#let halo-paint = if dark { "#132c29" } else { "#fff8ee" }
#let path-paint = if dark { "#e19a80" } else { "#b96852" }
#let minimum-paint = if dark { "#75d5b3" } else { "#087f64" }
#set document(title: "Rosenbrock — a curved optimization valley")
#set page(width: 254pt, height: 186pt, margin: 0pt, fill: none)
#set text(font: "DejaVu Sans", size: 11pt, fill: ink)
#let x = literal("x")
#let y = literal("y")
#let F = parse($(1 - x)^2 + 100 (y - x^2)^2$)
#let gx = derivative(F,x)
#let gy = derivative(F,y)
#let hxx = derivative(gx,x)
#let hxy = derivative(gx,y)
#let hyy = derivative(gy,y)
#let stationary = solve((gx,gy),(x,y),domain:"real")
#assert.eq(stationary.coverage,"complete")
#assert.eq(stationary.branches.len(),1)
#let minimum = stationary.branches.first().values
#assert.eq(minimum.map(canonical),("1","1"))
#let at-minimum(expr) = replace(replace(expr,x,minimum.at(0)),y,minimum.at(1))
#let H = matrix(((at-minimum(hxx),at-minimum(hxy)),(at-minimum(hxy),at-minimum(hyy))))
#assert.eq(canonical(determinant(H)),"400")
#assert.eq(canonical(at-minimum(hxx)),"802")
#assert.eq(canonical(at-minimum(F)),"0")

// Every landscape sample comes from the actual polynomial field.
#let x-min = -1.6
#let x-max = 1.6
#let y-min = -0.5
#let y-max = 2.5
#let nx = 161
#let ny = 151
#let field = evaluate-grid(F,(x,y),(
  domain(x-min,x-max,samples:nx),domain(y-min,y-max,samples:ny),
))
#let samples = field.values.map(row => row.first().re)
#let plot-width = 254.0
#let plot-height = 186.0
#let dx = plot-width/(nx - 1)
#let dy = plot-height/(ny - 1)
#let point(ix,iy) = (ix*dx,plot-height - iy*dy)
#let screen(point) = ((point.at(0) - x-min)*plot-width/(x-max - x-min),
  plot-height - (point.at(1) - y-min)*plot-height/(y-max - y-min))

// Local damped Newton method, with a real Armijo backtracking line search.
// Symbolica supplies every gradient/Hessian entry and function evaluation.
#let path = {
  let state = (-1.2,1.0)
  let result = (state,)
  for iteration in range(30) {
    let values = evaluate-many((F,gx,gy,hxx,hxy,hyy),(x,y),(state,)).first().map(value => value.re)
    let g = (values.at(1),values.at(2))
    if calc.sqrt(g.at(0)*g.at(0)+g.at(1)*g.at(1)) < 1e-8 { break }
    let a = values.at(3)
    let b = values.at(4)
    let c = values.at(5)
    let det = a*c - b*b
    assert(det > 0)
    let direction = ((c*g.at(0)-b*g.at(1))/det,(a*g.at(1)-b*g.at(0))/det)
    let descent = g.at(0)*direction.at(0)+g.at(1)*direction.at(1)
    let alpha = 1.0
    let next = state
    for attempt in range(30) {
      next = (state.at(0)-alpha*direction.at(0),state.at(1)-alpha*direction.at(1))
      let trial = evaluate-many(F,(x,y),(next,)).first().first().re
      if trial <= values.first()-1e-4*alpha*descent { break }
      alpha /= 2
    }
    state = next
    result.push(state)
  }
  assert(calc.abs(state.at(0)-1) < 1e-6)
  assert(calc.abs(state.at(1)-1) < 1e-6)
  result
}
#let number(value) = str(calc.round(value, digits: 3))
#let coord(p) = number(p.at(0)) + " " + number(p.at(1))
#let crossing(a, b, va, vb, level) = {
  let t = (level - va) / (vb - va)
  (a.at(0) + t * (b.at(0) - a.at(0)), a.at(1) + t * (b.at(1) - a.at(1)))
}

// A regular-grid marching-squares renderer is ordinary Typst code.
// The ambiguous diagonal case uses the bilinear center's field value.
#let contour(level) = {
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
        segments.push("M " + coord(crossings.at(0).at(1)) + " L " + coord(crossings.at(1).at(1)))
      } else if crossings.len() == 4 {
        let center = values.sum() / 4
        let pair-01 = (values.at(0) < level) == (center < level)
        let pairs = if pair-01 { ((0, 1), (2, 3)) } else { ((0, 3), (1, 2)) }
        for pair in pairs {
          segments.push("M " + coord(crossings.at(pair.at(0)).at(1))
            + " L " + coord(crossings.at(pair.at(1)).at(1)))
        }
      }
    }
  }
  segments.join(" ")
}


#let levels = (0.1,0.3,1,3,8,20,50,100,250,500,1000)
#let paints = if dark {
  ("#75d5b3","#65c7a6","#58b999","#72c2ac","#8dc9b8",
    "#b7c2b2","#dcc19a","#e1ad92","#d6a2bb","#c2a5d6","#ab98cf")
} else {
  ("#087f64","#138e71","#2f9f84","#57ae96","#82b9a8",
    "#aeaba4","#c6a089","#c08e80","#ad889c","#9487b1","#8077aa")
}
#let curves = levels.enumerate().map(((index,level)) => (
  "<path d='"+contour(level)+"' fill='none' stroke='"+paints.at(index)
  +"' stroke-width='1.15' stroke-linecap='round' stroke-linejoin='round'/>"
))
#let path-data = "M "+path.map(state => coord(screen(state))).join(" L ")
#let trajectory = ("<path d='"+path-data+"' fill='none' stroke='"+halo-paint+"' stroke-width='4'/>"
  +"<path d='"+path-data+"' fill='none' stroke='"+path-paint+"' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'/>")
#let steps = path.slice(0,path.len() - 1).map(state => {
  let p = screen(state)
  "<circle cx='"+number(p.at(0))+"' cy='"+number(p.at(1))+"' r='1.8' fill='"+path-paint+"' stroke='"+halo-paint+"' stroke-width='.65'/>"
}).join("")
#let optimum = screen((1,1))
#let dot = ("<circle cx='"+number(optimum.at(0))+"' cy='"+number(optimum.at(1))+"' r='3.5' fill='"+minimum-paint+"' stroke='"+halo-paint+"' stroke-width='1.2'/>"
  +"<path d='M"+number(optimum.at(0)+5)+" "+number(optimum.at(1)-4)+" l8 -10' fill='none' stroke='"+minimum-paint+"' stroke-width='.8'/>")
#let svg = ("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 "+str(plot-width)+" "+str(plot-height)+"'>"
  +"<defs><radialGradient id='fade'><stop offset='0' stop-color='white'/><stop offset='.72' stop-color='white'/><stop offset='1' stop-color='black'/></radialGradient>"
  +"<mask id='edge'><rect width='100%' height='100%' fill='url(#fade)'/></mask></defs>"
  +"<g mask='url(#edge)'>"+curves.join("")+trajectory+steps+"</g>"+dot+"</svg>")
#image(bytes(svg), width: 254pt, height: 186pt)
#metadata((minimum:minimum.map(canonical),minimum-value:canonical(at-minimum(F)),
  gradient:(canonical(gx),canonical(gy)),hessian-determinant:canonical(determinant(H)),
  field-points:samples.len(),newton-steps:path.len() - 1,last:path.last())) <data>
