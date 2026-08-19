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
)

#let input = $((y^x + 1)^2)/(1/a + "some" + "thing")$

#let parsed = parsely.parse(input, grammar)
#metadata(repr(parsed)) <parsed>

// Metadata must bind before the empty implicit-multiplication operator.
#let tagged = algebra.symbol("q", namespace: "model", tags: ("positive", "parameter"))
#let annotated = parsely.parse($#tagged + 1$, grammar)
#let annotation = annotated.tree.args.first()
#assert.eq(annotation.head, "semantic-metadata")
#assert.eq(annotation.slots.value.protocol, "tymbolica")
#assert.eq(annotation.slots.value.version, 1)
#assert.eq(annotation.slots.value.kind, "atom")
#assert(type(annotation.slots.value.atom) == bytes)
#assert.eq(annotation.slots.value.semantic.namespace, "model")
#assert.eq(annotation.slots.value.semantic.tags, ("positive", "parameter"))

// A function constructor annotates the whole call, not merely its head.
#let f = algebra.function("f", namespace: "model", tags: ("smooth",))
#let call = parsely.parse($#f(tagged)$, grammar).tree
#assert.eq(call.head, "semantic-metadata")
#assert.eq(call.slots.value.semantic.kind, "function-call")
#assert.eq(call.slots.value.semantic.head.namespace, "model")
#assert.eq(call.slots.value.semantic.head.tags, ("smooth",))
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
