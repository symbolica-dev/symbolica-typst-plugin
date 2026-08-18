#import "../lib.typ": init, init-idenso

#set page(width: auto, height: auto, margin: 12pt)

#let sym = init(namespace: "spenso")
#let tensors = init-idenso()

#let input = (sym.math)(
  $g(op("mink")(4, 0), op("mink")(4, 1)) p(op("mink")(4, 1))$,
)
#let contracted = (tensors.simplify-metrics)(input)

Tymbolica creates the Atom, Idenso contracts the repeated Lorentz index, and
Tymbolica reads the returned Atom directly:

$
  #(sym.to-typst)(input)
  quad arrow.r.long
  #(sym.to-typst)(contracted)
$

// This final operation makes the cross-plugin round trip part of the test.
#(sym.to-typst)((sym.expand)(contracted))

// Symbolica's native Atom export preserves arbitrary-precision floats.
#let builtin = init(namespace: "symbolica")
#let approximation = (builtin.to-float)(
  (builtin.math)($cos(1/3) + 1/2$),
  decimal-prec: 6,
)
#let float-roundtrip = (tensors.simplify-metrics)(approximation)
#assert.eq((builtin.canonical)(float-roundtrip), "1.44496")
