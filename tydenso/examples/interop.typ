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

// Tydenso content is accepted directly by Tymbolica. Antisymmetry survives the
// round trip because the exact Atom is carried instead of being reparsed.
#let W = tensors.euc(3)
#let a = tensors.slot(W, "a")
#let b = tensors.slot(W, "b")
#let F = tensors.tensor("Finterop", antisymmetric: true)
#let carried = algebra.expand(F(a, b))
#assert.eq(tensors.to-string(tensors.add(carried, F(b, a))), "0")

// The other direction retains a namespace that is not visible in the glyph.
#let mass = algebra.symbol("m", namespace: "model")
#let roundtrip = tensors.add(mass, 0)
#assert.eq(
  algebra.canonical(roundtrip, namespaces: true),
  algebra.canonical(mass, namespaces: true),
)

// A custom representation also survives a trip through the algebra plugin,
// including the palette that turns its first index into mu.
#let M = tensors.representation(
  "M",
  3,
  namespace: "interop_representation",
  self-dual: true,
  indices: ($std.sym.mu$, $std.sym.nu$),
)
#let q = tensors.vector("q", namespace: "interop_representation")
#let custom = q(tensors.slot(M, 1))
#let custom-roundtrip = algebra.expand(custom)
#assert(tensors.to-typst-source(custom-roundtrip).contains("t:μ"))
