#import "../lib.typ" as tensors
#import "../../typst/lib.typ" as algebra

#set page(width: auto, height: auto, margin: 12pt)

#let V = tensors.mink(4)
#let p = tensors.vector("p")
#let mu = tensors.slot(V, "mu")
#let nu = tensors.slot(V, "nu")
#let expression = tensors.mul(tensors.metric(V, mu, nu), p(nu))

Tydenso prints its own tensor notation:

$ #tensors.to-typst(expression) $

The same native Atom payload can be inspected or transformed by Tymbolica:

#let expanded = algebra.expand(expression)
#raw(algebra.canonical(expanded))

// Keep the cross-plugin payload contract in the compiled test suite.
#assert(type(expanded) == bytes)
#assert(type(tensors.inspect(expanded)) == dictionary)
