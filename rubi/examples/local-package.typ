#import "@local/symbolica:0.1.0" as sym
#import "@local/symbolica-integrate:0.1.0": integrate-with-steps

#set page(width: auto, height: auto, margin: 12pt)

#let x = sym.math($x$)
#let integrand = sym.math($x^2$)
#let explanation = integrate-with-steps(integrand, x)

$ integral #sym.to-typst(integrand) dif x
  = #sym.to-typst(explanation.result) + C $
