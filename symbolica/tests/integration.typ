#import "../examples/integration.typ": primitive, explanation, integrand, x
#import "../lib.typ": *

#let residual = together(subtract(derivative(primitive, x), integrand))

#assert(type(primitive) == bytes)
#assert(explanation.complete)
#assert(type(explanation.result) == bytes)
#assert(explanation.steps.len() > 0)
#assert.eq(canonical(primitive), canonical(explanation.result))
#assert.eq(canonical(residual), "0")

// Integration is also part of configured engines and accepts symbol handles.
#let configured = init(namespace: "integration_test")
#let variable = (configured.literal)("x")
#let expression = (configured.parse)($x^2$)
#let result = (configured.integrate)(expression, variable)
#assert.eq((configured.canonical)((configured.together)((configured.subtract)(
  (configured.derivative)(result, variable), expression,
))), "0")
#let trace = (configured.integrate-with-steps)($x^2$, variable)
#assert(trace.complete)
#assert.eq((configured.canonical)(result), (configured.canonical)(trace.result))
