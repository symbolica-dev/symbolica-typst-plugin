#import "../lib.typ": *

#assert("inspect" in init())
#assert("from-tree" in init())
#assert.eq(inspect("x").ok, false)
#assert.eq(inspect($x$).ok, false)
#assert.eq(inspect(bytes("other plugin")).ok, false)
#assert.eq(inspect(matrix(((1, 2), (3, 4)))).ok, false)

#let expr = parse($x^2/(1+x)$)
#let result = inspect(expr)
#assert(result.ok)
#assert.eq(result.tree.protocol, "symbolica")
#assert.eq(result.tree.version, 1)
#assert.eq(result.tree.kind, "expression-tree")
#assert.eq(result.tree.root.kind, "product")
#assert.eq(canonical(from-tree(result.tree)), canonical(expr))

// Exact values remain strings, including integers beyond Typst's integer range.
#let large = inspect(parse("123456789012345678901234567891/7")).tree
#assert.eq(large.root.kind, "rational")
#assert.eq(large.root.numerator, "123456789012345678901234567891")
#assert.eq(large.root.denominator, "7")
#let edited = inspect(parse($x^2$)).tree
#assert.eq(edited.root.kind, "power")
#assert.eq(edited.root.base.symbol.name, "typst::x")
#{ edited.root.exponent = (kind: "rational", numerator: "3", denominator: "1") }
#assert.eq(canonical(from-tree(edited)), canonical(parse($x^3$)))

// Decorated heads stay opaque symbols, and their labels survive reconstruction.
#let labeled = parse($h_(i)(x)+a_(j i)$)
#let rebuilt = from-tree(inspect(labeled).tree)
#assert.eq(canonical(rebuilt), canonical(labeled))
#assert.eq(to-typst-source(rebuilt), to-typst-source(labeled))
#assert.eq(canonical(parse(to-typst(rebuilt))), canonical(labeled))

#let floating = parse(1.25)
#let float-tree = inspect(floating).tree
#assert.eq(float-tree.root.kind, "float")
#assert.eq(float-tree.root.real, 1.25)
#assert.eq(float-tree.root.imaginary, 0.0)
#assert.eq(canonical(from-tree(float-tree)), canonical(floating))
#for value in (1.44496, -0.00125, 1e30, 1e-30, 1.2345678901234567) {
  let tree = inspect(parse(value)).tree
  assert(calc.abs(tree.root.real / value - 1) < 1e-15)
  assert.eq(canonical(from-tree(tree)), canonical(parse(value)))
}

// A small downstream evaluator consumes the semantic tree directly.
#let compute(node) = {
  if node.kind == "rational" {
    float(node.numerator) / float(node.denominator)
  } else if node.kind == "symbol" {
    assert.eq(node.symbol.name, "typst::x")
    2.0
  } else if node.kind == "sum" {
    node.terms.map(compute).sum()
  } else if node.kind == "product" {
    node.factors.map(compute).product()
  } else if node.kind == "power" {
    calc.pow(compute(node.base), compute(node.exponent))
  } else {
    panic("unsupported node: " + node.kind)
  }
}
#assert(calc.abs(compute(result.tree.root) - 4 / 3) < 1e-14)
