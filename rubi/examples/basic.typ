#import "../lib.typ": integrate, integrate-with-steps
#import "../../typst/lib.typ" as sym

#set page(width: 150mm, height: auto, margin: 16mm)
#set text(size: 10.5pt)

= Rubi as a separate integration engine

#let x = sym.math($x$)
#let integrand = sym.math($x / (x - 2+y)$)
#let primitive = integrate(integrand, x)
#let explanation = integrate-with-steps(integrand, x)
#let residual = sym.together(sym.sub(sym.derivative(primitive, x), integrand))

#assert(type(primitive) == bytes)
#assert(explanation.complete)
#assert(type(explanation.result) == bytes)
#assert(explanation.steps.len() > 0)
#assert.eq(sym.canonical(primitive), sym.canonical(explanation.result))
#assert.eq(sym.canonical(residual), "0")

$ integral #sym.to-typst(integrand) dif x
  = #sym.to-typst(primitive) + C $

#for step in explanation.steps [
  #h(step.depth * 1.2em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.2em)
  $#sym.to-typst(step.input) = #sym.to-typst(step.output)$
  #linebreak()
]
