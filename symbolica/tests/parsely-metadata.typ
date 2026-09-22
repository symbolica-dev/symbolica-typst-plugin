// Maintainer regression coverage for Parsely and semantic metadata.
#import "@preview/parsely:0.1.1"
#import "../lib.typ": *

// Minimal Parsely parse example matching the Symbolica grammar shape.
#let grammar = (
  semantic-metadata: (postfix: metadata, prec: 5),
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  frac: (match: std.math.frac),
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  op: (match: std.math.op),
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>

// Element match rules in Parsely 0.1.1 expose named slots. Check that those
// slots still become the intended Symbolica expressions.
#assert.eq(canonical(parse($frac(3, 4)$)), canonical(divide(3, 4)))
#assert.eq(canonical(parse($sqrt(9)$)), canonical(3))
#assert.eq(canonical(parse($root(3, 8)$)), canonical(2))

// Metadata must bind before the empty implicit-multiplication operator.
#let tagged = literal("q", namespace: "model", tags: ("model::positive", "model::parameter"))
#let annotated = parsely.parse($#tagged + 1$, grammar)
#let annotation = annotated.tree.args.first()
#assert.eq(annotation.head, "semantic-metadata")
#assert.eq(annotation.slots.value.protocol, "symbolica")
#assert.eq(annotation.slots.value.version, 1)
#assert.eq(annotation.slots.value.kind, "atom")
#assert(type(annotation.slots.value.atom) == bytes)
#assert.eq(annotation.slots.value.semantic.namespace, "model")
#assert.eq(annotation.slots.value.semantic.tags, ("model::positive", "model::parameter"))

// A function constructor annotates the whole call, not merely its head.
#let f = function-head("f", namespace: "model", tags: ("model::smooth",))
#let call = parsely.parse($#f(tagged)$, grammar).tree
#assert.eq(call.head, "semantic-metadata")
#assert.eq(call.slots.value.semantic.kind, "function-call")
#assert.eq(call.slots.value.semantic.head.namespace, "model")
#assert.eq(call.slots.value.semantic.head.tags, ("model::smooth",))
#assert(canonical(parse($#f(tagged)$), namespaces: true).contains("model"))

// Metadata from other packages remains visually transparent to the algebra.
#let foreign = std.math.attach(eval("x", mode: "math")) + metadata((
  protocol: "another-package",
  version: 1,
))
#assert.eq(
  canonical(parse($#foreign + 1$), namespaces: true),
  canonical(parse($x + 1$), namespaces: true),
)

// Custom grammars acquire the semantic postfix automatically.
#let lean-grammar = (
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
)
#let custom = init(namespace: "fallback", grammar: lean-grammar)
#let custom-q = (custom.literal)("q", namespace: "model")
#let custom-expression = (custom.parse)($#custom-q + 1$)
#assert((custom.canonical)(custom-expression, namespaces: true).contains("model"))

// Rendered Atom content retains exact payloads only at semantic leaves. The
// ordinary call and its arguments remain visible Parsely structure.
#let rendered-q = literal("q", namespace: "rendered_model")
#let rendered-f = function-head("f", namespace: "rendered_model")
#let original-call = rendered-f(rendered-q)
#let shown-call = to-typst(original-call)
#let shown-tree = parsely.parse($#shown-call$, grammar).tree
#assert.eq(shown-tree.head, "op-call")
#assert.eq(shown-tree.slots.op.head, "semantic-metadata")
#assert.eq(shown-tree.slots.op.slots.value.semantic.kind, "render-node")
#assert.eq(shown-tree.slots.op.slots.value.semantic.node-kind, "function-head")
#assert.eq(shown-tree.slots.args.head, "semantic-metadata")
#assert.eq(shown-tree.slots.args.slots.value.semantic.node-kind, "variable")

#let restored-call = parse($#shown-call$)
#assert.eq(
  canonical(restored-call, namespaces: true),
  canonical(original-call, namespaces: true),
)

#let composed-call = parse($2 #shown-call + 1$)
#let expected-call = add(mul(2, original-call), 1)
#assert.eq(
  canonical(composed-call, namespaces: true),
  canonical(expected-call, namespaces: true),
)

// A normal function's argument is not hidden by the exact head payload.
#let original-sum-call = rendered-f(add(rendered-q, 1))
#let shown-sum-call = to-typst(original-sum-call)
#let shown-sum-tree = parsely.parse($#shown-sum-call$, grammar).tree
#assert.eq(shown-sum-tree.head, "op-call")
#assert.eq(shown-sum-tree.slots.args.head, "add")
#assert.eq(
  canonical(parse($#shown-sum-call$), namespaces: true),
  canonical(original-sum-call, namespaces: true),
)

// Matrices have a separate payload format and remain ordinary display content.
#let shown-matrix = to-typst(matrix(((1, 2), (3, 4))))
#assert(type(shown-matrix) == std.content)

// A fractional coefficient followed by a power is a product, not a quotient
// containing the power. Preserve its value when parsing rendered content too.
#let fractional-power = parse($1/3 x^2$)
#assert.eq(
  canonical(fractional-power),
  canonical(mul(divide(1, 3), pow("x", 2))),
)

// Bare math elements and nested display equations split markup into separate
// lines. The public renderer must return one equation with an unwrapped body.
#let assert-no-equations(value) = {
  if type(value) == std.content {
    assert.ne(value.func(), std.math.equation)
    assert-no-equations(value.fields())
  } else if type(value) == dictionary {
    for child in value.values() { assert-no-equations(child) }
  } else if type(value) == array {
    for child in value { assert-no-equations(child) }
  }
}
#let rendering-cases = (
  fractional-power,
  parse($2 x y$),
  parse($x + 2 y - 3 z$),
  parse($(x + 1)^2$),
  parse($f(x, y + 1, 2 z)$),
  original-call,
  parse($1/3$),
)
#for expr in rendering-cases {
  let inline = to-typst(expr)
  let display = to-typst(expr, block: true)
  assert.eq(inline.func(), std.math.equation)
  assert.eq(inline.block, false)
  assert.eq(display.func(), std.math.equation)
  assert.eq(display.block, true)
  assert-no-equations(inline.body)
  assert-no-equations(display.body)
  for shown in (inline, display) {
    assert.eq(
      canonical(parse(shown), namespaces: true),
      canonical(expr, namespaces: true),
    )
  }
  assert.eq(
    canonical(parse($2 (#inline) + 1$), namespaces: true),
    canonical(add(mul(2, expr), 1), namespaces: true),
  )
  context {
    // Direct markup insertion must occupy the same line as explicit math.
    let direct = measure([Before #inline after.])
    let embedded = measure([Before $#inline$ after.])
    assert(calc.abs(direct.height - embedded.height) < 0.01pt)
    assert(calc.abs(direct.width - embedded.width) < 0.01pt)
  }
}
#assert.eq(shown-matrix.func(), std.math.equation)
#assert.eq(shown-matrix.block, false)
#assert-no-equations(shown-matrix.body)

// The custom-call default joins multiple visual arguments in the same way.
#let default-call-notation = notation(fallback-call: ctx => (ctx.default)())
#let multiple-args = parse($f(x, 2 y)$)
#let custom-call = to-typst(multiple-args, notation: default-call-notation)
#assert-no-equations(custom-call.body)
#assert.eq(
  canonical(parse(custom-call), namespaces: true),
  canonical(multiple-args, namespaces: true),
)

Before #to-typst(fractional-power) after.

// Native rendering follows the source printer's fraction, power, and spacing
// conventions while retaining a structured, reparsable equation body.
#let printer-cases = (
  $x^2/(1+x)$,
  $1/(1+x)$,
  $(x+1)/(y+1)$,
  $x/(y z)$,
  $1/(x y)$,
  $(1+x)/((1+y) (1+z))$,
  $x/(1+y)^2$,
  $2 x/(3 y)$,
  $-x/y$,
  $-1/(x y)$,
  $1-x/y$,
  $x^(-2)$,
  $x^(1/2)$,
  $x^(-2/3)$,
  $x/y^(1/2)$,
  $2 x y$,
  $-x y$,
  $x^(-y)$,
  $(1/2)^x$,
  $f(x/y, 1/(1+x))$,
)
#for input in printer-cases {
  let expr = parse(input)
  let shown = to-typst(expr)
  let source = to-typst-source(expr)
  let printed = eval(source, mode: "math", scope: (:))
  assert-no-equations(shown.body)
  assert.eq(
    canonical(parse(shown), namespaces: true),
    canonical(expr, namespaces: true),
    message: source,
  )
  context {
    for style in (std.math.inline, std.math.display) {
      let native-size = measure(std.math.equation(style(shown)))
      let printed-size = measure(std.math.equation(style(printed)))
      assert(calc.abs(native-size.width - printed-size.width) < 0.01pt, message: source + " width")
      assert(calc.abs(native-size.height - printed-size.height) < 0.01pt, message: source + " height")
    }
  }
}

// Notation still reaches both sides of a fraction. The display name must not
// replace the original namespaced symbol when the equation is parsed again.
#let fraction-x = literal("x", namespace: "fraction_model")
#let fraction-expr = parse($#fraction-x^2/(1+#fraction-x)$)
#let fraction-notation = notation(heads: ("fraction_model::x": $xi$))
#let fraction-shown = to-typst(fraction-expr, notation: fraction-notation)
#assert.eq(fraction-shown.body.func(), std.math.frac)
#assert.eq(
  canonical(parse(fraction-shown), namespaces: true),
  canonical(fraction-expr, namespaces: true),
)
#context {
  let actual = measure(fraction-shown)
  let expected = measure($xi^2/(1+xi)$)
  assert(calc.abs(actual.width - expected.width) < 0.01pt)
  assert(calc.abs(actual.height - expected.height) < 0.01pt)
}

// Whole-call overrides keep their exact call payload even in a denominator.
#let fraction-f = function-head("f", namespace: "fraction_model")
#let call-fraction = divide(1, fraction-f(fraction-x))
#let call-fraction-shown = to-typst(call-fraction, notation: notation(
  calls: ("fraction_model::f": ctx => $cal(F)(#ctx.visual-arguments.first())$),
  heads: ("fraction_model::x": $xi$),
))
#assert.eq(call-fraction-shown.body.func(), std.math.frac)
#assert.eq(
  canonical(parse(call-fraction-shown), namespaces: true),
  canonical(call-fraction, namespaces: true),
)
