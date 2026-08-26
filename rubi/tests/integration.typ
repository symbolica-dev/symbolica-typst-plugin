#import "../examples/basic.typ": primitive, explanation, integrand, x
#import "../../typst/lib.typ" as sym

#let residual = sym.together(sym.sub(sym.derivative(primitive, x), integrand))

#assert(type(primitive) == bytes)
#assert(explanation.complete)
#assert(type(explanation.result) == bytes)
#assert(explanation.steps.len() > 0)
#assert.eq(sym.canonical(primitive), sym.canonical(explanation.result))
#assert.eq(sym.canonical(residual), "0")
