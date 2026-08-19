#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let V = euc(3)
#let mu-slot = slot(V, "mu")
#let nu-slot = slot(V, "nu")
#let F = tensor("F", antisymmetric: true)
#let cancellation = add(F(mu-slot, nu-slot), F(nu-slot, mu-slot))

$ F^(mu nu) + F^(nu mu) = #to-typst(cancellation) $

#assert.eq(to-string(cancellation), "0")
