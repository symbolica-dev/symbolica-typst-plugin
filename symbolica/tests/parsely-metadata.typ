// Maintainer regression coverage for Parsely and semantic metadata.
#import "@preview/parsely:0.1.0"
#import "../lib.typ" as algebra

// Minimal Parsely parse example matching the Symbolica grammar shape.
#let grammar = (
  semantic-metadata: (postfix: metadata, prec: 5),
  add: (infix: $+$, prec: 1, assoc: true),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  frac: math.frac,
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  op: math.op,
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>

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
