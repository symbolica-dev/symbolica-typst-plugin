#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let V = euc(3)
#let mu = slot(V, "mu")
#let nu = slot(V, "nu")
#let F = tensor("F", antisymmetric: true)
#let cancellation = add(F(mu, nu), F(nu, mu))

$ F_(mu nu) + F_(nu mu) = #to-typst(cancellation) $

#assert.eq(to-string(cancellation), "0")
