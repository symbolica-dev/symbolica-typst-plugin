#import "@local/tymbolica:0.1.0" as sym
#import "@local/tymbolica-rubi:0.1.0": integrate, integrate-with-steps

#set page(width: auto, height: auto, margin: 12pt)

#let x = sym.math($x$)
#let integrand = sym.math($x^2$)
#let primitive = integrate(integrand, x)
#let explanation = integrate-with-steps(integrand, x)
#let residual = sym.expand(sym.sub(
  sym.derivative(explanation.result, x),
  integrand,
))

#assert(type(primitive) == bytes)
#assert(type(explanation.result) == bytes)
#assert(explanation.complete)
#assert.eq(sym.canonical(primitive), sym.canonical(explanation.result))
#assert.eq(sym.canonical(residual), "0")

$ integral #sym.to-typst(integrand) dif x
  = #sym.to-typst(explanation.result) + C $
