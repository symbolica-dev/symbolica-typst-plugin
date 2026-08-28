#import "../lib.typ": *
#import "../../typst/lib.typ": canonical

#set page(width: auto, height: auto, margin: 8pt)

#let M = mink(4)
#let B = bis(4)
#let mu = slot(M, $u$)
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
#let color-t = function("t")
#let ordinary-f = function("ordinary", namespace: "leaf_test")
#let ordinary-x = symbol("x", namespace: "leaf_test")

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
  v(p(2)),
  u(p(1)),
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
  gamma(slot(M, $alpha$)),
  A("in", "out", mu, p(3, M), nu),
  gamma(slot(M, $beta$)),
)
#let middle-representation-product = p(1, q(2,M))

// #let middle-representation-product = dot(p(1, M, 2), q(3, M, 4))
#let marker-factor = K(mu, cin, a, cout, nu)
#let reversed-marker-factor = K(mu, cout, a, cin, nu)
#let gamma0-factor = gamma0()
#let reversed-gamma0-factor = gamma0(cout, cin)
#let gamma5-factor = gamma5()
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



// Presentation is not serialization. `to-typst` carries the exact Atom as
// metadata, so even custom bra-ket notation composes losslessly.
#let shown-open-chain = to-typst(open-chain)
#let restored-open-chain = math($#shown-open-chain$)
#assert.eq(inspect(restored-open-chain), inspect(open-chain))

#let shown-product = to-typst(product)
#let composed-rendering = math($#shown-open-chain + #shown-product$)
#let expected-composition = add(open-chain, product)
#assert.eq(inspect(composed-rendering), inspect(expected-composition))

// Normal function heads retain their exact namespace while their argument
// expression stays visible and editable.
#let ordinary-call = ordinary-f(add(ordinary-x, 1))
#assert.eq(inspect(math($#to-typst(ordinary-call)$)), inspect(ordinary-call))

#let shown-block = to-typst(open-chain, block: true)
#assert(shown-block.fields().block)
#assert.eq(inspect(math(shown-block)), inspect(open-chain))

#let labelled-momentum = p($arrow(x + y)$, M)
#let restored-momentum = math($#to-typst(labelled-momentum)$)
#assert.eq(inspect(restored-momentum), inspect(labelled-momentum))

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

// Every specialized layout remains an exact, composable Atom. These checks
// deliberately exercise the content API rather than a private source string.
#let rendered-cases = (
  product,
  middle-representation-product,
  gamma(p(1, M)),
  open-chain,
  nested-chain,
  closed-chain,
  explicit-slot-chain,
  two-mink-ports,
  mixed-polarity,
  heterogeneous-bra,
  dot(p(1, M), q(2, mink(5))),
  dot(p(1, M), q(2, B)),
  dot(p(1, L), q(2, L)),
  marker-factor,
  reversed-marker-factor,
  gamma0-factor,
  reversed-gamma0-factor,
  gamma5-factor,
  reversed-gamma5-factor,
  projector,
  color-generator,
)
#for expression in rendered-cases {
  let shown = to-typst(expression)
  assert.eq(inspect(math($#shown$)), inspect(expression))
}

// Exact heads, complete calls, tags, and Symbolica attribute classes can all
// be styled in Typst. A complete-call renderer sees the visible arguments;
// Tydenso adds the exact call metadata after the closure returns.
#let special = function(
  "weighted",
  namespace: "leaf_test",
  tags: ("display::operator",),
)
#let F = tensor("F", namespace: "leaf_test", antisymmetric: true)
#let custom-display = notation(
  heads: ("leaf_test::x": $xi$),
  calls: (
    "leaf_test::ordinary": ctx => {
      let (argument,) = ctx.visual-arguments
      $cal(F)[#argument]$
    },
  ),
  tags: (
    "display::operator": ctx => {
      let (argument,) = ctx.visual-arguments
      $bold(W)(#argument)$
    },
  ),
  classes: (
    "antisymmetric": ctx => {
      let arguments = ctx.visual-arguments.join($,$)
      $cal(A)[#arguments]$
    },
  ),
)
#let custom-call = to-typst(ordinary-call, notation: custom-display)
#let custom-tag = to-typst(special(ordinary-x), notation: custom-display)
#let custom-class = to-typst(F(mu, nu), notation: custom-display)

#grid(
  columns: 3,
  gutter: 1em,
  [explicit rows], $ #to-typst(interleaved) $,canonical(interleaved),
  [gamma tensor], $ #to-typst(gamma-tensor) $,canonical(gamma-tensor),
  [dot product], $ #to-typst(product) $,canonical(product),
  [open chain], $ #to-typst(open-chain) $,canonical(open-chain),
  [explicit-slot chain], $ #to-typst(explicit-slot-chain) $,canonical(explicit-slot-chain),
  [closed trace], $ #to-typst(closed-chain) $,canonical(closed-chain),
  [two Mink ports], $ #to-typst(two-mink-ports) $,canonical(two-mink-ports),
  [nested factor], $ #to-typst(nested-chain) $,canonical(nested-chain),
  [heterogeneous bra], $ #to-typst(heterogeneous-bra) $,canonical(heterogeneous-bra),
  [heterogeneous ket], $ #to-typst(heterogeneous-ket) $,canonical(heterogeneous-ket),
  [mixed polarity], $ #to-typst(mixed-polarity) $,canonical(mixed-polarity),
  [typed ports], $ #to-typst(
    heterogeneous-bra,
    notation: notation(with-dim: true),
  ) $,canonical(heterogeneous-bra),
  [middle rep], $ #to-typst(middle-representation-product) $,canonical(middle-representation-product),
  [reversed factor], $ #to-typst(reversed-marker-factor) $,canonical(middle-representation-product),
  [Idenso heads], $ #to-typst(projector) #to-typst(gamma0-factor) #to-typst(gamma5-factor) $,[#raw(canonical(projector)),#raw(canonical(gamma0-factor)),#raw(canonical(gamma5-factor))],
  [color rows], $ #to-typst(color-generator) $,canonical(color-generator),
  [custom exact call], $ #custom-call $,canonical(custom-call),
  [custom tag], $ #custom-tag $,canonical(custom-tag),
  [custom class], $ #custom-class $,[],
)
