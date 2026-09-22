// Portable display labels are ordinary symbols, distinct from pattern wildcards.
#import "../lib.typ": *

#let same(lhs, rhs) = assert.eq(canonical(lhs, namespaces: true), canonical(rhs, namespaces: true))
#let different(lhs, rhs) = assert.ne(canonical(lhs, namespaces: true), canonical(rhs, namespaces: true))

#same(literal($x$), literal("x"))
#same(literal($a_0$), parse($a_0$))
#same(literal($C_(i j)$), parse($C_(i j)$))
#different(literal($C_(i j)$), literal($C_(j i)$))
#same(literal([hello world]), literal($"hello world"$))

#let label = literal($x+y$)
#different(label, parse($x+y$))
#same(label, literal($x+y$))
#different(label, literal($y+x$))
#same(replace(label, literal("x"), 7), label)
#same(replace(label, label, 7), 7)
#same(derivative(label, literal("x")), 0)
#same(derivative(pow(label, 2), label), mul(2, label))
#same(expand(pow(label, 2)), pow(label, 2))
#same(parse($#label^2+1$), add(pow(label, 2), 1))
#same(replace(label, wild("anything"), 7), 7)
#same(replace(literal("x"), label, 7), literal("x"))
#assert.eq(canonical(wild("args", level: 2)), "args__")
#assert.eq(canonical(wild("args", level: 3)), "args___")

// Numeric and power-shaped labels remain one symbolic variable.
#let empty-attachment = literal(math.attach($a$.body, t: [ ]))
#same(empty-attachment, literal($attach(a,t:"")$))
#different(empty-attachment, literal("a"))
#for display in (math.attach($a$.body, b: [ ]), math.frac([ ], $a$), math.overbrace($a$, [ ])) {
  let value = literal(display)
  same(parse(to-typst(value)), value)
}
#let embedded = literal($K_(#label)$)
#same(replace(embedded, label, 7), embedded)
#let escaped = literal(text("a \"quote\" and #hash"))
#same(parse(to-typst(escaped)), escaped)
#let two = literal($2$)
#different(two, 2)
#same(derivative(two, two), 1)
#let power-label = literal($x^2$)
#different(power-label, pow("x", 2))
#same(expand(pow(power-label, 3)), pow(power-label, 3))

// Explicit identity can distinguish two variables with the same appearance.
#let first = literal($a_0$, name: "first", namespace: "literal_model", tags: ("model::parameter",))
#let second = literal($a_0$, name: "second", namespace: "literal_model")
#different(first, second)
#same(first, literal($a_0$, name: "first", namespace: "literal_model", tags: ("model::parameter",)))
#assert.eq(canonical(first, namespaces: true), "literal_model::first")
#assert.eq(to-typst-source(first), to-typst-source(second))
#let custom = to-typst(first, notation: notation(tags: ("model::parameter": $theta$)))
#same(parse(custom), first)

// The configured namespace matters; algebra grammar overrides do not rewrite labels.
#let engine = init(namespace: "literal_namespace", grammar: (:))
#let namespaced = (engine.literal)($x+y$)
#different(namespaced, label)
#same(namespaced, literal($x+y$, namespace: "literal_namespace"))
#same((engine.literal)($x$, namespace: "typst"), literal("x"))
#let consumer = init(namespace: "literal_consumer")
#same((consumer.parse)(to-typst(namespaced)), namespaced)

// Exact annotated output keeps opaque identity in every algebraic context.
#let labels = (
  label, power-label, literal($x y$), two, first, second, embedded, empty-attachment,
  literal([hello world]), literal($frac(x,y)$), literal($sqrt(x)$),
  literal($vec(x,y)$), literal($mat(x,y;y,x)$), literal($h'(c)$),
  literal($attach(x,t:2,b:i,tl:a,tr:b,bl:c,br:d)$),
)
#for value in labels {
  for expr in (value, pow(value, 2), pow(value, -2), pow(value, divide(1, 2)), pow(value, divide(-2, 3)), mul(3, "z", value), divide(value, add(value, 1))) {
    let shown = to-typst(expr)
    let source = to-typst-source(expr)
    let printed = eval(source, mode: "math", scope: (:))
    same(parse(shown), expr)
    same((consumer.parse)(shown), expr)
    context {
      let native = measure(shown)
      let legacy = measure(printed)
      assert(calc.abs(native.width - legacy.width) < 0.01pt, message: source + " width")
      assert(calc.abs(native.height - legacy.height) < 0.01pt, message: source + " height")
    }
    [#shown $arrow.r$ #printed\ ]
  }
}

// Source has visual grouping but no metadata: parsing it reads the visible math.
#same(parse(eval(to-typst-source(pow(label, 2)), mode: "math")), parse($(x+y)^2$))
#same(parse(eval(to-typst-source(mul("z", label)), mode: "math")), parse($z (x+y)$))
#same(parse(eval(to-typst-source(pow(power-label, 3)), mode: "math")), parse($(x^2)^3$))
