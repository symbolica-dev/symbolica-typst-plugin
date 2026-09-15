#import "../examples/integration.typ": primitive, explanation, integrand, x
#import "../lib.typ" as sym

#let residual = sym.together(sym.sub(sym.derivative(primitive, x), integrand))

#assert(type(primitive) == bytes)
#assert(explanation.complete)
#assert(type(explanation.result) == bytes)
#assert(explanation.steps.len() > 0)
#assert.eq(sym.canonical(primitive), sym.canonical(explanation.result))
#assert.eq(sym.canonical(residual), "0")

// Integration is also part of configured engines and accepts symbol handles.
#let configured = sym.init(namespace: "integration_test")
#let variable = (configured.symbol)("x")
#let expression = (configured.math)($x^2$)
#let result = (configured.integrate)(expression, variable)
#assert.eq((configured.canonical)((configured.together)((configured.sub)(
  (configured.derivative)(result, variable), expression,
))), "0")
#let trace = (configured.integrate-with-steps)($x^2$, variable)
#assert(trace.complete)
#assert.eq((configured.canonical)(result), (configured.canonical)(trace.result))
