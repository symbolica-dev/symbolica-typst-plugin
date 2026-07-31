#import "../lib.typ": *

#set page(width: 150mm, height: auto, margin: 18mm)
#set text(size: 11pt)
#set par(leading: 0.7em)

= Exact algebra in one document

Tymbolica turns native Typst mathematics into an exact Symbolica expression and
places the computed result back into the document.

#let x = var("x")
#let polynomial = math($x^4 - 5 x^2 + 4$)
#let factored = factor(polynomial)
#let slope = derivative(polynomial, x)
#let area = integrate(polynomial, x)

$
  f(x) &= #to-typst(polynomial) \
       &= #to-typst(factored) quad "factored" \
  f'(x) &= #to-typst(slope) \
  integral f(x) dif x &= #to-typst(area) + C
$

All coefficients remain exact. Tymbolica deliberately leaves the integration
constant to the surrounding mathematics.
