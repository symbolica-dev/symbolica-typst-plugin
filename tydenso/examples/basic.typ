#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let V = mink(4)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let p = tensor("p")

#let expression = mul(metric(V, mu, nu), p(nu))
#let contracted = simplify-metrics(expression)

$
  #to-typst(expression)
  quad arrow.r.long
  #to-typst(contracted)
$

// Representation and slot metadata remain ordinary Typst values.
#assert.eq(mu.kind, "slot")
#assert.eq(mu.representation.name, "mink")
#assert.eq(mu.representation.dimension, 4)

#let color-dual = dual-representation(cof(3))
#assert(color-dual.is-dual)
#assert(slot(color-dual, "a").dual)

// Atom internals are available as a recursive CBOR-decoded tree.
#let tree = inspect(contracted)
#assert(tree.kind in ("function", "product"))

// Tydenso's printer is independent of Tymbolica.
#assert(type(to-typst-source(contracted)) == str)
#assert(type(to-string(contracted)) == str)
