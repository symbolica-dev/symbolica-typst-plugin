#import "@local/tymbolica:0.1.0": *

#set page(width: 120mm, height: auto, margin: 12mm)
#set text(size: 10pt)

= Local package smoke test

#let x = var("x")
#let polynomial = math($x^4 - 5 x^2 + 4$)

$ f(x) = #to-typst(polynomial) $

$ f(x) = #to-typst(factor(polynomial)) $

$ f'(x) = #to-typst(derivative(polynomial, x)) $

#let rubi = init(profile: "full")
#let rubi-var = rubi.var
#let rubi-math = rubi.math
#let rubi-integrate = rubi.integrate
#let rubi-render = rubi.to-typst
#let rubi-x = rubi-var("x")
#let antiderivative = rubi-integrate(rubi-math($x / (x + 1)$), rubi-x)

$ integral x / (x + 1) dif x = #rubi-render(antiderivative) $
