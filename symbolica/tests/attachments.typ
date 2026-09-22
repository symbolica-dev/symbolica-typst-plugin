// Decorated symbols retain their identity, ordered display content, and calls.
#import "../lib.typ": *

#let namespaced-canonical(expr) = canonical(expr, namespaces: true)
#let same(lhs, rhs) = assert.eq(namespaced-canonical(lhs), namespaced-canonical(rhs))
#let different(lhs, rhs) = assert.ne(namespaced-canonical(lhs), namespaced-canonical(rhs))

// Original report: subscripts must not disappear and primes must not turn a
// function call into a product. These are labels, not automatic derivatives.
#let a0 = parse($a_0$)
#let a1 = parse($a_1$)
#different(a0, a1)
#different(a0, parse($a$))
#different(parse($a_0-a_1$), 0)
#same(parse($a_0-a_1$), subtract(a0, a1))
#different(parse($C_(i j)-C_(j i)$), 0)
#different(parse($C_(i j)$), parse($C_(j i)$))
#different(parse($K_(1 2)$), parse($K_2$))
#let pfaffian = parse($4 (K_(1 2) K_(3 4)-K_(1 3) K_(2 4)+K_(1 4) K_(2 3))$)
#same(pfaffian, mul(4, add(
  mul(parse($K_(1 2)$), parse($K_(3 4)$)),
  neg(mul(parse($K_(1 3)$), parse($K_(2 4)$))),
  mul(parse($K_(1 4)$), parse($K_(2 3)$)),
)))
#different(pfaffian, parse($4 K^2$))
#let plain-call = parse($h(c)$)
#let prime-call = parse($h'(c)$)
#let double-prime-call = parse($h''(c)$)
#same(plain-call, (function-head("h"))(parse($c$)))
#different(prime-call, plain-call)
#different(double-prime-call, prime-call)
#different(prime-call, parse($h c$))
#same(parse($f_(x x)/2$), divide(parse($f_(x x)$), 2))
#different(parse($f_(x x)/2$), parse($f/2$))

// Upper t attachments retain algebraic exponent semantics; all other slots
// decorate the base symbol. No other slot may be lost when t is present.
#same(parse($attach(x,t:2)$), pow("x", 2))
#same(parse($a_0^2$), pow(a0, 2))
#same(parse($h'(c)^2$), pow(prime-call, 2))
#same(parse($(h'(c))^2$), pow(prime-call, 2))
#same(parse($h'^2$), pow(parse($h'$), 2))
#same(parse($x^2(y)$), mul(pow("x", 2), "y"))
#let slot-cases = (
  $attach(x,b:i)$,
  $attach(x,tl:i)$,
  $attach(x,tr:i)$,
  $attach(x,bl:i)$,
  $attach(x,br:i)$,
)
#for (index, expr) in slot-cases.enumerate() {
  different(parse(expr), parse($x$))
  for previous in slot-cases.slice(0, index) {
    different(parse(expr), parse(previous))
  }
}
#let empty-cases = (
  $attach(x,t:"")$,
  $attach(x,t:"",b:"",tl:"",tr:"",bl:"",br:"")$,
  $attach(x,b:"")$, $attach(x,tl:"")$, $attach(x,tr:"")$,
  $attach(x,bl:"")$, $attach(x,br:"")$,
  $attach(x,b:"",tl:"",tr:"",bl:"",br:"")$,
)
#let combined = $attach(x,b:i,tl:a,tr:b,bl:c,br:d)$
#let combined-power = $attach(x,t:2,b:i,tl:a,tr:b,bl:c,br:d)$
#same(parse(combined-power), pow(parse(combined), 2))
#same(parse($h_(i)(c)$), parse($attach(h,b:i)(c)$))
#different(parse($h_(i)(c)$), plain-call)
#different(parse($attach(h,t:"")(c)$), plain-call)
#different(parse($h'$), parse($h''$))
#different(parse($a_(i_j)$), parse($a_(i j)$))
#different(parse($a_(i+1)$), parse($a_(1+i)$))
#different(parse($a_("x")$), parse($a_x$))

// Typed display trees preserve nested layouts and ordered array-valued fields.
#let nested-cases = (
  $a_("label")$,
  $a_("x")$,
  $a_(lr([i+j]))$,
  $attach(attach(h,tr:primes(#1)),b:i)^2$,
  $a_(frac(i,j))$,
  $a_((i+j))$,
  $a_(lr((i+j)))$,
  $a_((i+j)^2)$,
  $a_(h(i,j))$,
  $a_(op("foo")(i,j))$,
  $a_(sqrt(i_0))$,
  $a_(overline(i_0))$,
  $a_(accent(i_0,arrow))$,
  $a_(vec(i_0,j_1))$,
  $a_(mat(i_0,j_1;i_1,j_0))$,
  $attach(a,b:attach(i,tl:j,tr:k,bl:l,br:m))$,
)
#let original-cases = (
  $a_0$, $a_1$, $a_0-a_1$, $C_(i j)-C_(j i)$,
  $4 (K_(1 2) K_(3 4)-K_(1 3) K_(2 4)+K_(1 4) K_(2 3))$,
  $h(c)$, $h'(c)$, $h''(c)$, $f_(x x)/2$,
)
#for input in original-cases + slot-cases + empty-cases + nested-cases + (
  combined, combined-power, $a_0^2$, $h_(i)(c)$, $h'(c)^2$, $h'^2$,
  $attach(h,t:"")(c)$,
) {
  let expr = parse(input)
  same(parse(input), expr)
  same(parse(to-typst(expr)), expr)
  same(parse(eval(to-typst-source(expr), mode: "math")), expr)
  [#input $arrow.r$ #to-typst(expr)\ ]
}

// Exact annotations retain the original namespace in another parser engine.
#let model = init(namespace: "attachment_model")
#let consumer = init(namespace: "attachment_consumer")
#let namespaced = (model.parse)($h'(a_0)+a_1$)
#same((consumer.parse)(to-typst(namespaced)), namespaced)

// Ordinary decorated function calls keep their arguments outside head metadata.
#let shown-prime = to-typst(prime-call)
#let shown-head = shown-prime.body.children.first()
#assert.eq(shown-head.func(), std.math.op)
#let edited = parse(shown-head + $(d+1)$.body)
#same(edited, replace(prime-call, parse($c$), parse($d+1$)))

// Display labels are opaque to algebra; an index spelling is not a free symbol.
#same(replace(parse($C_(i j)$), parse($i$), parse($k$)), parse($C_(i j)$))
