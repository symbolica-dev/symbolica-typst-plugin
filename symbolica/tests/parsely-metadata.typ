// Maintainer regression coverage for Parsely and semantic metadata.
#import "@preview/parsely:0.1.1"
#import "../lib.typ" as algebra

// Minimal Parsely parse example matching the Symbolica grammar shape.
#let grammar = (
  semantic-metadata: (postfix: metadata, prec: 5),
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  frac: (match: math.frac),
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  op: (match: math.op),
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>

// Element match rules in Parsely 0.1.1 expose named slots. Check that those
// slots still become the intended Symbolica expressions.
#assert.eq(algebra.canonical(algebra.math($frac(3, 4)$)), algebra.canonical(algebra.div(3, 4)))
#assert.eq(algebra.canonical(algebra.math($sqrt(9)$)), algebra.canonical(3))
#assert.eq(algebra.canonical(algebra.math($root(3, 8)$)), algebra.canonical(2))

// Metadata must bind before the empty implicit-multiplication operator.
#let tagged = algebra.symbol("q", namespace: "model", tags: ("model::positive", "model::parameter"))
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
#let f = algebra.function("f", namespace: "model", tags: ("model::smooth",))
#let call = parsely.parse($#f(tagged)$, grammar).tree
#assert.eq(call.head, "semantic-metadata")
#assert.eq(call.slots.value.semantic.kind, "function-call")
#assert.eq(call.slots.value.semantic.head.namespace, "model")
#assert.eq(call.slots.value.semantic.head.tags, ("model::smooth",))
#assert(algebra.canonical(algebra.math($#f(tagged)$), namespaces: true).contains("model"))

// Metadata from other packages remains visually transparent to the algebra.
#let foreign = math.attach(eval("x", mode: "math")) + metadata((
  protocol: "another-package",
  version: 1,
))
#assert.eq(
  algebra.canonical(algebra.math($#foreign + 1$), namespaces: true),
  algebra.canonical(algebra.math($x + 1$), namespaces: true),
)

// Custom grammars acquire the semantic postfix automatically.
#let lean-grammar = (
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
)
#let custom = algebra.init(namespace: "fallback", grammar: lean-grammar)
#let custom-q = (custom.symbol)("q", namespace: "model")
#let custom-expression = (custom.math)($#custom-q + 1$)
#assert((custom.canonical)(custom-expression, namespaces: true).contains("model"))

// Rendered Atom content retains exact payloads only at semantic leaves. The
// ordinary call and its arguments remain visible Parsely structure.
#let rendered-q = algebra.symbol("q", namespace: "rendered_model")
#let rendered-f = algebra.function("f", namespace: "rendered_model")
#let original-call = rendered-f(rendered-q)
#let shown-call = algebra.to-typst(original-call)
#let shown-tree = parsely.parse($#shown-call$, grammar).tree
#assert.eq(shown-tree.head, "op-call")
#assert.eq(shown-tree.slots.op.head, "semantic-metadata")
#assert.eq(shown-tree.slots.op.slots.value.semantic.kind, "render-node")
#assert.eq(shown-tree.slots.op.slots.value.semantic.node-kind, "function-head")
#assert.eq(shown-tree.slots.args.head, "semantic-metadata")
#assert.eq(shown-tree.slots.args.slots.value.semantic.node-kind, "variable")

#let restored-call = algebra.math($#shown-call$)
#assert.eq(
  algebra.canonical(restored-call, namespaces: true),
  algebra.canonical(original-call, namespaces: true),
)

#let composed-call = algebra.math($2 #shown-call + 1$)
#let expected-call = algebra.add(algebra.mul(2, original-call), 1)
#assert.eq(
  algebra.canonical(composed-call, namespaces: true),
  algebra.canonical(expected-call, namespaces: true),
)

// A normal function's argument is not hidden by the exact head payload.
#let original-sum-call = rendered-f(algebra.add(rendered-q, 1))
#let shown-sum-call = algebra.to-typst(original-sum-call)
#let shown-sum-tree = parsely.parse($#shown-sum-call$, grammar).tree
#assert.eq(shown-sum-tree.head, "op-call")
#assert.eq(shown-sum-tree.slots.args.head, "add")
#assert.eq(
  algebra.canonical(algebra.math($#shown-sum-call$), namespaces: true),
  algebra.canonical(original-sum-call, namespaces: true),
)

// Matrices have a separate payload format and remain ordinary display content.
#let shown-matrix = algebra.to-typst(algebra.matrix(((1, 2), (3, 4))))
#assert(type(shown-matrix) == content)

// A fractional coefficient followed by a power is a product, not a quotient
// containing the power. Preserve its value when parsing rendered content too.
#let fractional-power = algebra.math($1/3 x^2$)
#assert.eq(
  algebra.canonical(fractional-power),
  algebra.canonical(algebra.mul(algebra.div(1, 3), algebra.pow("x", 2))),
)

// Bare math elements and nested display equations split markup into separate
// lines. The public renderer must return one equation with an unwrapped body.
#let assert-no-equations(value) = {
  if type(value) == content {
    assert.ne(value.func(), math.equation)
    assert-no-equations(value.fields())
  } else if type(value) == dictionary {
    for child in value.values() { assert-no-equations(child) }
  } else if type(value) == array {
    for child in value { assert-no-equations(child) }
  }
}
#let rendering-cases = (
  fractional-power,
  algebra.math($2 x y$),
  algebra.math($x + 2 y - 3 z$),
  algebra.math($(x + 1)^2$),
  algebra.math($f(x, y + 1, 2 z)$),
  original-call,
  algebra.math($1/3$),
)
#for expr in rendering-cases {
  let inline = algebra.to-typst(expr)
  let display = algebra.to-typst(expr, block: true)
  assert.eq(inline.func(), math.equation)
  assert.eq(inline.block, false)
  assert.eq(display.func(), math.equation)
  assert.eq(display.block, true)
  assert-no-equations(inline.body)
  assert-no-equations(display.body)
  for shown in (inline, display) {
    assert.eq(
      algebra.canonical(algebra.math(shown), namespaces: true),
      algebra.canonical(expr, namespaces: true),
    )
  }
  assert.eq(
    algebra.canonical(algebra.math($2 (#inline) + 1$), namespaces: true),
    algebra.canonical(algebra.add(algebra.mul(2, expr), 1), namespaces: true),
  )
  context {
    // Direct markup insertion must occupy the same line as explicit math.
    let direct = measure([Before #inline after.])
    let embedded = measure([Before $#inline$ after.])
    assert(calc.abs(direct.height - embedded.height) < 0.01pt)
    assert(calc.abs(direct.width - embedded.width) < 0.01pt)
  }
}
#assert.eq(shown-matrix.func(), math.equation)
#assert.eq(shown-matrix.block, false)
#assert-no-equations(shown-matrix.body)

// The custom-call default joins multiple visual arguments in the same way.
#let default-call-notation = algebra.notation(fallback-call: ctx => (ctx.default)())
#let multiple-args = algebra.math($f(x, 2 y)$)
#let custom-call = algebra.to-typst(multiple-args, notation: default-call-notation)
#assert-no-equations(custom-call.body)
#assert.eq(
  algebra.canonical(algebra.math(custom-call), namespaces: true),
  algebra.canonical(multiple-args, namespaces: true),
)

Before #algebra.to-typst(fractional-power) after.

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
  let expr = algebra.math(input)
  let shown = algebra.to-typst(expr)
  let source = algebra.to-typst-source(expr)
  let printed = eval(source, mode: "math", scope: (:))
  assert-no-equations(shown.body)
  assert.eq(
    algebra.canonical(algebra.math(shown), namespaces: true),
    algebra.canonical(expr, namespaces: true),
    message: source,
  )
  context {
    for style in (math.inline, math.display) {
      let native-size = measure(math.equation(style(shown)))
      let printed-size = measure(math.equation(style(printed)))
      assert(calc.abs(native-size.width - printed-size.width) < 0.01pt, message: source + " width")
      assert(calc.abs(native-size.height - printed-size.height) < 0.01pt, message: source + " height")
    }
  }
}

// Notation still reaches both sides of a fraction. The display name must not
// replace the original namespaced symbol when the equation is parsed again.
#let fraction-x = algebra.symbol("x", namespace: "fraction_model")
#let fraction-expr = algebra.math($#fraction-x^2/(1+#fraction-x)$)
#let fraction-notation = algebra.notation(heads: ("fraction_model::x": $xi$))
#let fraction-shown = algebra.to-typst(fraction-expr, notation: fraction-notation)
#assert.eq(fraction-shown.body.func(), math.frac)
#assert.eq(
  algebra.canonical(algebra.math(fraction-shown), namespaces: true),
  algebra.canonical(fraction-expr, namespaces: true),
)
#context {
  let actual = measure(fraction-shown)
  let expected = measure($xi^2/(1+xi)$)
  assert(calc.abs(actual.width - expected.width) < 0.01pt)
  assert(calc.abs(actual.height - expected.height) < 0.01pt)
}

// Whole-call overrides keep their exact call payload even in a denominator.
#let fraction-f = algebra.function("f", namespace: "fraction_model")
#let call-fraction = algebra.div(1, fraction-f(fraction-x))
#let call-fraction-shown = algebra.to-typst(call-fraction, notation: algebra.notation(
  calls: ("fraction_model::f": ctx => $cal(F)(#ctx.visual-arguments.first())$),
  heads: ("fraction_model::x": $xi$),
))
#assert.eq(call-fraction-shown.body.func(), math.frac)
#assert.eq(
  algebra.canonical(algebra.math(call-fraction-shown), namespaces: true),
  algebra.canonical(call-fraction, namespaces: true),
)
