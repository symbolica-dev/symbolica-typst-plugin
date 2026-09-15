#import "@local/symbolica:0.1.0" as sym

#set page(width: auto, height: auto, margin: 12pt)

#let x = sym.math($x$)
#let integrand = sym.math($x^2$)
#let explanation = sym.integrate-with-steps(integrand, x)

$ integral #sym.to-typst(integrand) dif x
  = #sym.to-typst(explanation.result) + C $
