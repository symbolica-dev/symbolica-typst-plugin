// Native Symbolica strings and Typst notation share the public parse API.
#import "../lib.typ": *

#let same(lhs, rhs) = assert.eq(
  canonical(lhs, namespaces: true), canonical(rhs, namespaces: true),
)
#same(parse("x^2/(1+x)"), parse($x^2/(1+x)$))
#same(parse("1/3"), divide(1, 3))
#assert.eq(canonical(parse("123456789012345678901234567890")), "123456789012345678901234567890")
#same(parse("alpha+alpha"), mul(2, literal("alpha")))
#same(parse("f(x,y)"), (function-head("f"))(literal("x"), literal("y")))
#same(parse("model::x+x", namespace: "other"), add(
  literal("x", namespace: "model"), literal("x", namespace: "other"),
))
#same(parse("gamma(6)"), 120)
#same(parse("symbolica::gamma(6)"), 120)
#same(parse("gamma(6)", namespace: "symbolica"), 120)

// Numeric inputs retain exact integers and floating-point coefficients.
#for (value, expected) in ((0, "0"), (42, "42"), (-42, "-42"), (9007199254740993, "9007199254740993"), (-9007199254740993, "-9007199254740993")) {
  assert.eq(type(parse(value)), bytes)
  assert.eq(canonical(parse(value)), expected)
}
#for (value, expected) in ((0.5, "0.5"), (-0.125, "-0.125"), (1.25, "1.25")) {
  assert.eq(type(parse(value)), bytes)
  same(parse(value), parse(expected))
  assert.eq(evaluate(parse(value)).re, value)
}
#assert.eq(canonical(parse(0.5)), "0.5")
#assert.eq(canonical(parse("1/2")), "1/2")

// A configured Typst grammar has no effect on native string parsing.
#let engine = init(namespace: "string_model", grammar: (:))
#same((engine.parse)("x^2+1"), add(pow(literal("x", namespace: "string_model"), 2), 1))
#same((engine.parse)("x", namespace: "override"), literal("x", namespace: "override"))
#same(parse("x"), literal("x"))
#assert.eq(canonical((engine.parse)(-9007199254740993)), "-9007199254740993")
#assert.eq(canonical((engine.parse)(0.5)), "0.5")
#same(parse($#literal("x") + 1$), parse("x+1"))
#same(parse(to-typst(parse("f(x)+x^2/(1+x)"))), parse("f(x)+x^2/(1+x)"))

// Payloads pass through unchanged, including when the consumer has a different
// namespace or grammar. Parsing a literal retains its identity, label, and tags.
#let tagged = literal($q_0$, name: "coordinate", namespace: "parse_metadata", tags: ("model::coordinate",))
#let saved = parse(tagged)
#assert.eq(parse(saved), saved)
#assert.eq(parse(saved, namespace: "other"), saved)
#assert.eq((engine.parse)(saved), saved)
#assert.eq((engine.parse)(saved, namespace: "override"), saved)
#assert.eq(canonical(saved, namespaces: true), "parse_metadata::coordinate")
#assert.eq(to-typst-source(saved), to-typst-source(tagged))
#let consumer = init(namespace: "parse_consumer")
#same((consumer.parse)($#tagged + x$), add(tagged, literal("x", namespace: "parse_consumer")))
#let tagged-display = to-typst((consumer.parse)(tagged), notation: notation(tags: ("model::coordinate": $theta$)))
#context { assert.eq(measure(tagged-display).width, measure($theta$).width) }

// Old engine names are removed so the public vocabulary stays consistent.
#for (old, new) in (
  math: "parse", atom: "parse", symbol: "literal", function: "function-head",
  terms: "summands", content: "matrix-content", vec: "vector",
  det: "determinant", cancel: "cancel-factors", sub: "subtract", div: "divide",
  gamma: "gamma-function", zeta: "zeta-function",
) {
  assert(old not in engine)
  assert(new in engine)
}

// Wildcard imports preserve Typst's native modules, types, and notation.
#assert.eq(type(literal("x")), content)
#assert.eq(type(function-head("f")), function)
#assert.eq(type(symbol("★")), symbol)
#assert.eq(sym.arrow.r, std.sym.arrow.r)
#assert.eq(math.vec, std.math.vec)
#assert.eq(terms, std.terms)
#assert.eq(sub, std.sub)
#assert.eq($vec(1, 2)$.body.func(), math.vec)
#assert.eq($cancel(x)$.body.func(), math.cancel)
#assert.eq($gamma zeta div det J$.body, $std.math.gamma std.math.zeta std.math.div #std.math.det J$.body)

$ #to-typst(parse("x^2/(1+x)")) quad vec(1, 2) quad gamma quad zeta $
