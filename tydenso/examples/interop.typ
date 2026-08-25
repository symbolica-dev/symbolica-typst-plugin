#import "../lib.typ" as tensors
#import "../../typst/lib.typ" as algebra
#import "../../rubi/lib.typ" as calculus

#set page(width: auto, height: auto, margin: 12pt)

#let V = tensors.mink(4)
#let p = tensors.vector("p")
#let mu = tensors.slot(V, 1)
#let nu = tensors.slot(V, 2)
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
#let a = tensors.slot(W, 1)
#let b = tensors.slot(W, 2)
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

// The core algebra plugin can round-trip an Idenso representation without
// linking Idenso or Spenso itself.
#let B = tensors.bis(4)
#let u = tensors.vector("u")
#let spinor = u(tensors.slot(B, 1))
#let spinor-roundtrip = algebra.expand(spinor)
#assert.eq(tensors.to-string(spinor-roundtrip), tensors.to-string(spinor))

// Structured Typst notation is a portable attachment too. Core preserves it
// opaquely, then Tydenso restores it before printing the imported Atom.
#let routed = p($arrow(x + y)$, V)
#let routed-roundtrip = algebra.expand(routed)
#assert.eq(tensors.inspect(routed-roundtrip), tensors.inspect(routed))
#assert.eq(
  tensors.inspect(tensors.math($#tensors.to-typst(routed-roundtrip)$)),
  tensors.inspect(routed),
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
#assert.eq(tensors.inspect(custom-roundtrip), tensors.inspect(custom))

// Core and Rubi do not link Spenso. They carry the custom declaration as an
// opaque attachment, and Tydenso is the only plugin that interprets it again.
// Rubi intentionally accepts the shared byte payload rather than Tydenso's
// annotated Typst content wrapper.
#let x = algebra.math($x$)
#let custom-primitive = calculus.integrate(tensors.atom(custom), x)
#assert(type(tensors.to-typst(custom-primitive)) == content)

#grid(
  columns: 2,
  gutter: 0.8em,
  [display metadata after Tymbolica], $ #tensors.to-typst(routed-roundtrip) $,
  [representation metadata after Tymbolica], $ #tensors.to-typst(custom-roundtrip) $,
  [representation metadata after Rubi], $ #tensors.to-typst(custom-primitive) $,
)
