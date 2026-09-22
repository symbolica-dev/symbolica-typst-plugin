#import "@local/symbolica:0.1.0": *

#set page(width: auto, height: auto, margin: 12pt)

#let x = parse($x$)
#let integrand = parse($x^2$)
#let explanation = integrate-with-steps(integrand, x)

$ integral #to-typst(integrand) dif x
  = #to-typst(explanation.result) + C $
