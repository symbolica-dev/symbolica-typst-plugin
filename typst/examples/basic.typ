#import "../lib.typ": *




= Exact algebra in one document

Tymbolica turns native Typst mathematics into an exact Symbolica expression and
places the computed result back into the document.symbol
#let x = symbol("x")
#let polynomial = math($x^4 - 5 x^10 + 4$)
#let slope = derivative(polynomial, x)
#let area = integrate(polynomial, x)
#let factored = factor(polynomial)
$
  f(x) &= #to-typst(polynomial) \
       &= #to-typst(factored) quad "factored" \
  f'(x) &= #to-typst(slope)
$

All coefficients remain exact; the compressed engine is loaded transparently.
