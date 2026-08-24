#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let cases = (
  (name: "mink", rep: mink(4), labels: ("mu", "nu", "rho", "sigma"), self-dual: true, row: "top"),
  (name: "euc", rep: euc(4), labels: ("i", "j", "k", "l"), self-dual: true, row: "top"),
  (name: "lor", rep: lor(4), labels: ("mu", "nu", "rho", "sigma"), self-dual: false, row: "top"),
  (name: "bis", rep: bis(4), labels: ("a", "b", "c", "d"), self-dual: true, row: "bottom"),
  (name: "spf", rep: spf(4), labels: ("alpha", "beta", "gamma", "delta"), self-dual: false, row: "top"),
  (name: "cof", rep: cof(3), labels: ("i", "j", "k", "l"), self-dual: false, row: "top"),
  (name: "coad", rep: coad(8), labels: ("a", "b", "c", "d"), self-dual: true, row: "top"),
  (name: "cos", rep: cos(6), labels: ("I", "J", "K", "L"), self-dual: false, row: "top"),
)

#for case in cases {
  assert.eq(case.rep.indices, case.labels)
  assert.eq(case.rep.index-start, 1)
  assert.eq(case.rep.self-dual, case.self-dual)
  assert.eq(case.rep.index-row, case.row)

  let dual = dual-representation(case.rep)
  assert.eq(dual.indices, case.labels)
  assert.eq(dual.index-start, 1)
  assert.eq(dual.is-dual, if case.self-dual { false } else { true })

  let T = tensor("T_" + case.name, namespace: "index_palette_example")
  let source = to-typst-source(T(
    slot(case.rep, 1),
    slot(case.rep, 2),
    slot(case.rep, 5),
  ))
  assert(source.contains(case.labels.first()))
  assert(source.contains("attach(" + case.labels.first() + ",b:1)"))
}

// Canonical built-ins behave the same through the generic constructor.
#let generic-mink = representation("mink", 4, self-dual: true)
#assert.eq(generic-mink.indices, ("mu", "nu", "rho", "sigma"))

// A manually named index keeps exactly the author's display metadata.
#let p = vector("p")
#let manual = p(slot(mink(4), $zeta_7$))
#assert(to-typst-source(manual).contains("attach(ζ,b:upright(\"7\"))"))

Automatic built-in indices: $ #to-typst(
  p(slot(mink(4), 1)),
) $, $ #to-typst(p(slot(mink(4), 5))) $.
