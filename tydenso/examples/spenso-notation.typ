#import "../lib.typ": *

#set page(width: auto, height: auto, margin: 8pt)

#let M = mink(4)
#let B = bis(4)
#let mu = slot(M, "mu")
#let nu = slot(M, "nu")
#let a = slot(B, "a")
#let b = slot(B, "b")

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
#let closed-chain = trace(
  B,
  cyclic(gamma(mu), gamma(p(1, M)), gamma(nu)),
)
#let interleaved = T(mu, a, nu, b)
#let two-mink-ports = A(mu, p(1, M), nu, q(2, M), slot(M, "rho"))
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
  gamma(slot(M, "alpha")),
  A("in", "out", mu, p(3, M), nu),
  gamma(slot(M, "beta")),
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
  slot(Adj, "c"),
  slot(Fund, "i"),
  slot(dual-representation(Fund), "j"),
)

#assert(atom-shape(inspect(product)) == "dot(p(1,mink(4)),q(2,mink(4)))")
#assert(atom-shape(inspect(gamma-factor)) == "gamma(in,out,mink(4,mu))")
#assert(
  atom-shape(inspect(gamma-tensor))
    == "gamma(bis(4,a),bis(4,b),mink(4,mu))",
)
#assert(
  atom-shape(inspect(open-chain))
    == "chain(u(1,bis(4)),v(2,bis(4)),gamma(in,out,mink(4,mu)),gamma(in,out,p(1,mink(4))),gamma(in,out,mink(4,nu)))",
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
#assert(to-typst-source(product).contains(" dot "))
#assert(to-typst-source(middle-representation-product).contains(" dot "))
#assert(to-typst-source(gamma(p(1, M))).contains("cancel("))
#assert(to-typst-source(open-chain).contains("upright(\"⟨\")"))
#assert(to-typst-source(open-chain).contains("upright(\"⟩\")"))
#assert(to-typst-source(two-mink-ports).contains("○"))
#assert(to-typst-source(mixed-polarity).contains("upright(\"⟨\")"))
#assert(to-typst-source(mixed-polarity).contains("upright(\"⟩\")"))
#assert(
  to-typst-source(
    heterogeneous-bra,
    settings: print-settings(with-dim: true),
  ).contains("attach(○,t:attach("),
)
#assert(not to-typst-source(dot(p(1, M), q(2, mink(5)))).contains(" dot "))
#assert(not to-typst-source(dot(p(1, M), q(2, B))).contains(" dot "))
#assert(not to-typst-source(dot(p(1, L), q(2, L))).contains(" dot "))
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
