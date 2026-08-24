#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 12pt)

#let V = mink(4)
#let mu-slot = slot(V, 1)
#let nu-slot = slot(V, 2)
#let p = vector("p")

#let expression = mul(metric(V, mu-slot, nu-slot), p(nu-slot))
#let contracted = simplify-metrics(expression)
#let parsed-expression = math($#metric(V, mu-slot, nu-slot) #p(nu-slot)$)
#let parsed-contracted = simplify-metrics(parsed-expression)
#let W = lor(4)
#let T = tensor("T")
#let mixed = T(slot(W, 1), slot(W, 2, dual: true))
#let mass = symbol("m", namespace: "model", tags: ("parameter",))
#let kernel = function("K", namespace: "model")

// Generic representations inherit an independently initialized API's namespace.
#let namespaced = init(namespace: "representation_namespace_example")
#let W = (namespaced.representation)("W", 2, self-dual: true)
#assert.eq(W.namespace, "representation_namespace_example")
#assert.eq(
  ((namespaced.inspect)((namespaced.construct)(W))).name,
  "representation_namespace_example::W",
)
#let generic-call = math($#kernel(mass)$)

$
  #to-typst(expression)
  quad arrow.r.long
  #to-typst(contracted)
$

// Representation and slot metadata remain ordinary Typst values.
#assert.eq(mu-slot.kind, "slot")
#assert.eq(mu-slot.representation.name, "mink")
#assert.eq(mu-slot.representation.dimension, 4)

#let M = representation(
  "M",
  4,
  namespace: "palette_example",
  self-dual: true,
  indices: ($mu$, $nu$),
)
#let M-dual = dual-representation(M)
#assert.eq(M-dual.indices, M.indices)
#assert.eq(M-dual.index-start, 1)
#assert.eq(M-dual.name, M.name)
#let X = tensor("X", namespace: "palette_example")
#let named-indices = X(slot(M, 1), slot(M, 2), slot(M, 3), slot(M, $rho_2$))
#let named-source = to-typst-source(named-indices)
#assert(named-source.contains("t:μ ν"))
#assert(named-source.contains("attach(μ,b:1)"))
#assert(named-source.contains("attach(ρ,b:upright(\"2\"))"))
#let qualified-source = to-typst-source(named-indices, settings: print-settings(with-dim: true))
#assert(qualified-source.contains("attach(attach(μ,b:1),t:attach(M,b:4))"))

// A dual orientation retains the same representation identity. Only the slot
// variance changes, so metric contraction still recognizes both orientations.
#let R = representation("R", 4, namespace: "dual_example")
#let R-dual = dual-representation(R)
#let i-name = $i$
#let j-name = $j$
#let i = slot(R, i-name)
#let j = slot(R, j-name)
#let j-dual = slot(R-dual, j-name)
#let q = vector("q", namespace: "dual_example")
#let custom-dual-contraction = simplify-metrics(mul(metric(R, i, j-dual), q(j)))
#assert.eq(inspect(custom-dual-contraction), inspect(q(i)))

// Constructor metadata on index content keeps the exact namespaced Atom.
#let exact-index = symbol("i", namespace: "exact_index_example")
#assert.eq(
  inspect(p(slot(V, exact-index))),
  inspect(p(slot(V, atom(exact-index)))),
)

// Raw Typst math is one portable display label in tensor and vector calls.
// Wrap the same input in `math` when it should be Symbolica algebra instead.
#let display-label = p($arrow(x + y)$, V)
#let algebraic-argument = p(math($x + y$), V)
#assert(to-typst-source(display-label).contains("accent("))
#assert(inspect(display-label) != inspect(algebraic-argument))

// A string is a semantic identifier; math content owns its written notation.
#let identifier-mu = p("mu", V)
#let notation-mu = p($std.sym.mu$, V)
#assert(to-typst-source(identifier-mu).contains("\"mu\""))
#assert(to-typst-source(notation-mu).contains("b:μ"))
#assert(inspect(identifier-mu) != inspect(notation-mu))

#let color-dual = dual-representation(cof(3))
#assert(color-dual.is-dual)
#assert(slot(color-dual, 1).dual)

// Atom internals are available as a recursive CBOR-decoded tree.
#let tree = inspect(contracted)
#assert(tree.kind in ("function", "product"))
#assert.eq(inspect(parsed-contracted), inspect(p(mu-slot)))
#assert.eq(inspect(generic-call).kind, "function")

// Tydenso's printer is independent of Tymbolica.
#assert(type(to-typst-source(contracted)) == str)
#assert(to-typst-source(p(mu-slot)).starts-with("attach("))
#assert(to-typst-source(mixed).contains("b:std.hide(mu) nu"))
#assert(type(to-string(contracted)) == str)
