#import "@preview/parsely:0.1.0"
#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let V = euc(3)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let F = tensor("F", antisymmetric: true)
#let cancellation = add(F(mu, nu), F(nu, mu))
#let parsed-cancellation = math($#F(mu, nu) + #F(nu, mu)$)

$ #F(mu, nu) + #F(nu, mu) = #to-typst(cancellation) $

#assert.eq(to-string(cancellation), "0")
#assert.eq(to-string(parsed-cancellation), "0")

// The sidecar remains inspectable Typst data at the Parsely boundary.
#let T = tensor("T", tags: ("field-strength",))
#let annotation = parsely.parse(
  $#T(mu, nu)$,
  (
    semantic-metadata: (postfix: metadata, prec: 5),
    mul: (infix: $$, prec: 2.5, assoc: true),
  ),
).tree.slots.value
#assert.eq(annotation.protocol, "tymbolica")
#assert.eq(annotation.kind, "atom")
#assert(type(annotation.atom) == bytes)
#assert.eq(annotation.semantic.kind, "tensor")
#assert.eq(annotation.semantic.name, "T")
#assert.eq(annotation.semantic.tags, ("field-strength",))
#assert.eq(annotation.semantic.arguments.len(), 2)
#assert.eq(annotation.semantic.arguments.at(0).index, "mu")
#assert.eq(annotation.semantic.arguments.at(1).index, "nu")
#assert.eq(annotation.semantic.arguments.at(0).representation.name, "euc")
#assert.eq(annotation.semantic.arguments.at(0).representation.dimension, 3)
