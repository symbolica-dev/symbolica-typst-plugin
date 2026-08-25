#import "../lib.typ": *
#import "@preview/parsely:0.1.0"

#set page(width: auto, height: auto, margin: 8pt)

#let M = mink(4)
#let B = bis(4)
#let mu = slot(M, 1)
#let nu = slot(M, 2)
#let a = slot(B, 1)
#let b = slot(B, 2)

#let p = vector("p")
#let q = vector("q")
#let u = vector("u")
#let v = vector("v")
#let chi = vector("chi")
#let r = vector("r")
#let s = vector("s")
#let T = tensor("T")
#let A = tensor("A")
#let H = tensor("H")
#let C = tensor("C")
#let D = tensor("D")
#let K = tensor("K")
#let cin = symbol("in")
#let cout = symbol("out")
#let gamma0 = function("gamma0")
#let gamma5 = function("gamma5")
#let projp = function("projp")
#let color-t = function("t")
#let ordinary-f = function("ordinary", namespace: "leaf_test")
#let ordinary-x = symbol("x", namespace: "leaf_test")

#let leaf-grammar = (
  semantic-metadata: (postfix: metadata, prec: 5),
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  op: math.op,
)

#let atom-shape(node) = {
  if node.kind == "function" {
    node.short-name + "(" + node.arguments.map(atom-shape).join(",") + ")"
  } else if node.kind == "symbol" {
    node.short-name
  } else if node.kind == "number" {
    node.value
  } else {
    panic("unexpected Atom node in notation test: " + repr(node))
  }
}

#let product = dot(p(1, M), q(2, M))
#let gamma-factor = gamma(mu)
#let gamma-tensor = gamma(mu, a, b)
#let open-chain = chain(
  u(1, B),
  v(2, B),
  gamma(mu),
  gamma(p(1, M)),
  gamma(nu),
)
#let explicit-slot-chain = chain(
  slot(B, $std.sym.mu$),
  slot(B, "blou"),
  gamma(mu),
  gamma(p(1, M)),
  gamma(nu),
)
#let closed-chain = trace(
  B,
  cyclic(gamma(mu), gamma(p(1, M)), gamma(nu)),
)
#let interleaved = T(mu, a, nu, b)
#let two-mink-ports = A(mu, p(1, M), nu, q(2, M), slot(M, 3))
#let heterogeneous-bra = H(mu, p(1, M), a, chi(2, B))
#let L = lor(4)
#let heterogeneous-ket = C(
  slot(M, "i"), r(1, L), slot(M, "j"), s(2, L),
)
#let mixed-polarity = D(
  slot(M, "i"), p(1, L), slot(M, "j"), q(2, dual-representation(L)),
)
#let nested-chain = chain(
  u(1, B),
  v(2, B),
  gamma(slot(M, $std.sym.alpha$)),
  A("in", "out", mu, p(3, M), nu),
  gamma(slot(M, $std.sym.beta$)),
)
#let middle-representation-product = dot(p(1, M, 2), q(3, M, 4))
#let marker-factor = K(mu, cin, a, cout, nu)
#let reversed-marker-factor = K(mu, cout, a, cin, nu)
#let gamma0-factor = gamma0(cin, cout)
#let reversed-gamma0-factor = gamma0(cout, cin)
#let gamma5-factor = gamma5(cin, cout)
#let reversed-gamma5-factor = gamma5(cout, cin)
#let projector = projp(a, b)
#let Adj = coad("Na")
#let Fund = cof("Nc")
#let color-generator = color-t(
  slot(Adj, 3),
  slot(Fund, 1),
  slot(dual-representation(Fund), 2),
)

#assert(atom-shape(inspect(product)) == "dot(p(1,mink(4)),q(2,mink(4)))")
#assert(atom-shape(inspect(gamma-factor)) == "gamma(in,out,mink(4,1))")
#assert(
  atom-shape(inspect(gamma-tensor))
    == "gamma(bis(4,1),bis(4,2),mink(4,1))",
)
#assert(
  atom-shape(inspect(open-chain))
    == "chain(u(1,bis(4)),v(2,bis(4)),gamma(in,out,mink(4,1)),gamma(in,out,p(1,mink(4))),gamma(in,out,mink(4,2)))",
)

// The custom bra-ket rendering is not an inverse serialization. `to-typst`
// therefore carries the exact Atom as metadata for lossless composition.
#let shown-open-chain = to-typst(open-chain)
#let restored-open-chain = math($#shown-open-chain$)
#assert.eq(inspect(restored-open-chain), inspect(open-chain))

#let shown-product = to-typst(product)
#let composed-rendering = math($#shown-open-chain + #shown-product$)
#let expected-composition = add(open-chain, product)
#assert.eq(inspect(composed-rendering), inspect(expected-composition))

// The custom chains are opaque leaves, but the sum between them is ordinary
// editable syntax rather than one authoritative root payload.
#let shown-composition = to-typst(expected-composition)
#let shown-composition-tree = parsely.parse(
  shown-composition,
  leaf-grammar,
).tree
#assert.eq(shown-composition-tree.head, "add")
#assert(shown-composition-tree.args.all(node => node.head == "semantic-metadata"))

// Normal function heads retain their exact namespace independently while the
// argument expression remains visible Parsely structure.
#let ordinary-call = ordinary-f(add(ordinary-x, 1))
#let ordinary-tree = parsely.parse(to-typst(ordinary-call), leaf-grammar).tree
#assert.eq(ordinary-tree.head, "op-call")
#assert.eq(ordinary-tree.slots.op.head, "semantic-metadata")
#assert.eq(ordinary-tree.slots.args.head, "add")
#assert.eq(inspect(math($#to-typst(ordinary-call)$)), inspect(ordinary-call))

#let shown-block = to-typst(open-chain, block: true)
#assert(shown-block.fields().block)
#assert.eq(inspect(math(shown-block)), inspect(open-chain))

#let labelled-momentum = p($arrow(x + y)$, M)
#let restored-momentum = math($#to-typst(labelled-momentum)$)
#assert.eq(
  to-typst-source(restored-momentum),
  to-typst-source(labelled-momentum),
)

#let closed-tree = inspect(closed-chain)
#assert(closed-tree.kind == "function")
#assert(closed-tree.short-name == "trace")
#assert(closed-tree.arguments.len() == 2)
#assert(closed-tree.arguments.at(0).short-name == "bis")
#assert(closed-tree.arguments.at(1).short-name == "cyclic")
#assert(closed-tree.arguments.at(1).cycle-symmetric)
#assert(closed-tree.arguments.at(1).arguments.len() == 3)
#assert(
  atom-shape(inspect(mixed-polarity))
    == "D(mink(4,i),p(1,lor(4)),mink(4,j),q(2,dind(lor(4))))",
)
#assert(to-typst-source(product).contains("math.class(\"normal\",$dot$)"))
#assert(
  to-typst-source(middle-representation-product).contains(
    "math.class(\"normal\",$dot$)",
  ),
)
#assert(to-typst-source(gamma(p(1, M))).contains("cancel("))
#assert(to-typst-source(open-chain).contains("upright(\"⟨\")"))
#assert(to-typst-source(open-chain).contains("upright(\"⟩\")"))
#assert(not to-typst-source(nested-chain).contains("lr(("))
#assert(to-typst-source(closed-chain).contains("op(\"Tr\") lr(("))
#assert(to-typst-source(explicit-slot-chain).contains("blou"))
#assert(not to-typst-source(explicit-slot-chain).contains("upright(\"⟨\")"))
#assert(to-typst-source(two-mink-ports).contains("○"))
#assert(to-typst-source(mixed-polarity).contains("upright(\"⟨\")"))
#assert(to-typst-source(mixed-polarity).contains("upright(\"⟩\")"))
#assert(
  to-typst-source(
    heterogeneous-bra,
    settings: print-settings(with-dim: true),
  ).contains("attach(○,t:attach("),
)
#assert(
  not to-typst-source(dot(p(1, M), q(2, mink(5)))).contains(
    "math.class(\"normal\",$dot$)",
  ),
)
#assert(
  not to-typst-source(dot(p(1, M), q(2, B))).contains(
    "math.class(\"normal\",$dot$)",
  ),
)
#assert(
  not to-typst-source(dot(p(1, L), q(2, L))).contains(
    "math.class(\"normal\",$dot$)",
  ),
)
#assert(
  to-typst-source(marker-factor)
    == "attach(#($K$,std.hide($zws$)).join(),t:mu std.hide(a) nu,b:std.hide(mu) a std.hide(nu))",
)
#assert(
  to-typst-source(reversed-marker-factor)
    == "attach(attach(#($K$,std.hide($zws$)).join(),t:mu std.hide(a) nu,b:std.hide(mu) a std.hide(nu)),t:upright(\"T\"))",
)
#assert(to-typst-source(gamma0-factor) == "gamma_0")
#assert(to-typst-source(reversed-gamma0-factor) == "gamma_0")
#assert(to-typst-source(gamma5-factor) == "gamma_5")
#assert(
  to-typst-source(reversed-gamma5-factor)
    == "attach(gamma_5,t:upright(\"T\"))",
)
#assert(to-typst-source(projector).contains("ℙ_p"))
#assert(
  to-typst-source(color-generator)
    == "attach(#($t$,std.hide($zws$)).join(),t:c i std.hide(j),b:std.hide(c) std.hide(i) j)",
)

#grid(
  columns: 2,
  gutter: 1em,
  [explicit rows], $ #to-typst(interleaved) $,
  [gamma tensor], $ #to-typst(gamma-tensor) $,
  [dot product], $ #to-typst(product) $,
  [open chain], $ #to-typst(open-chain) $,
  [explicit-slot chain], $ #to-typst(explicit-slot-chain) $,
  [closed trace], $ #to-typst(closed-chain) $,
  [two Mink ports], $ #to-typst(two-mink-ports) $,
  [nested factor], $ #to-typst(nested-chain) $,
  [heterogeneous bra], $ #to-typst(heterogeneous-bra) $,
  [heterogeneous ket], $ #to-typst(heterogeneous-ket) $,
  [mixed polarity], $ #to-typst(mixed-polarity) $,
  [typed ports], $ #to-typst(
    heterogeneous-bra,
    settings: print-settings(with-dim: true),
  ) $,
  [middle rep], $ #to-typst(middle-representation-product) $,
  [reversed factor], $ #to-typst(reversed-marker-factor) $,
  [Idenso heads], $ #to-typst(projector) #to-typst(gamma0-factor) #to-typst(gamma5-factor) $,
  [color rows], $ #to-typst(color-generator) $,
)
