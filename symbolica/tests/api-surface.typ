// Maintainer regression coverage for the public core API.
#import "../lib.typ": init

#let engine = init()
#assert("integrate" in engine)
#assert("integrate-with-steps" in engine)
#assert("literal" in engine)
#assert("function-head" in engine)
#assert("parse" in engine)
#assert("atom" not in engine)
#assert("var" not in engine)
#let parse = engine.parse
#let literal = engine.literal
#let symbolic-function = engine.function-head
#let wild = engine.wild
#let to-typst = engine.to-typst
#let to-float = engine.to-float
#let canonical = engine.canonical
#let add = engine.add
#let mul = engine.mul
#let subtract = engine.subtract
#let expand = engine.expand
#let factor = engine.factor
#let together = engine.together
#let cancel-factors = engine.cancel-factors
#let apart = engine.apart
#let collect = engine.collect
#let coefficient = engine.coefficient
#let coefficient-list = engine.coefficient-list
#let summands = engine.summands
#let indeterminates = engine.indeterminates
#let contains = engine.contains
#let is-constant = engine.is-constant
#let replace = engine.replace
#let replace-wildcards = engine.replace-wildcards
#let series = engine.series
#let evaluate = engine.evaluate
#let pi-value = evaluate((engine.parse)($pi^2 + sin(pi / 4)$))
#assert(calc.abs(pi-value.re - (calc.pi * calc.pi + calc.sin(calc.pi / 4))) < 1e-12)
#assert.eq(pi-value.im, 0.0)
#let domain = engine.domain
#let evaluate-many = engine.evaluate-many
#let evaluate-grid = engine.evaluate-grid
#let solve = engine.solve
#let nsolve = engine.nsolve
#let nsolve-system = engine.nsolve-system
#let matrix = engine.matrix
#let vector = engine.vector
#let identity = engine.identity
#let eye = engine.eye
#let matrix-solve = engine.matrix-solve
#let matrix-sub = engine.matrix-sub
#let matrix-mul = engine.matrix-mul
#let determinant = engine.determinant
#let inv = engine.inv
#let transpose = engine.transpose
#let augment = engine.augment
#let row-reduce = engine.row-reduce
#let matrix-at = engine.matrix-at
#let matrix-shape = engine.matrix-shape
#let matrix-is-zero = engine.matrix-is-zero
#let matrix-is-diagonal = engine.matrix-is-diagonal
#let matrix-derivative = engine.matrix-derivative

#let x = literal("x")
#let y = literal("y")
#let xp = literal("x", namespace: "physics")
#let namespaced = add(xp, x)
#let tagged = literal("q", namespace: "model", tags: ("model::positive", "model::parameter"))
#let tagged-expression = parse($#tagged + x$)
#let f = symbolic-function("f", namespace: "model")
#let function-expression = parse($#f(tagged) + x$)
#assert(canonical(namespaced, namespaces: true).contains("physics"))
#assert(canonical(tagged-expression, namespaces: true).contains("model"))
#assert(canonical(function-expression, namespaces: true).contains("model"))

#let value = evaluate(parse($x^2 + y$), values: ((x, 2.0), (y, 3.0)))
#let many = evaluate-many((parse($x + y$), parse($x y$)), (x, y), ((1, 2), (3, 4)))
#let complex-many = evaluate-many(parse($x^2$), x, (1, (re: 0.0, im: 1.0)))
#let grid = evaluate-grid(parse($x^2 + y$), (x, y), (domain(-1, 1, samples: 3), domain(0, 1, samples: 2)))
#assert.eq(many.at(0).map(value => value.re), (3.0, 2.0))
#assert.eq(many.at(1).map(value => value.re), (7.0, 12.0))
#assert.eq(complex-many.map(row => row.first().re), (1.0, -1.0))
#assert.eq(grid.shape, (3, 2))
#assert.eq(grid.points.len(), 6)
#assert.eq(grid.points.first(), (-1.0, 0.0))
#assert.eq(grid.points.last(), (1.0, 1.0))
#assert.eq(grid.values.last().first().re, 2.0)

#let polynomial = parse($5 x + x y + x^2 + 5$)
#let collected = collect(polynomial, x)
#let x2-coefficient = coefficient(polynomial, parse($x^2$))
#let coefficients = coefficient-list(polynomial, x)
#let rebuilt = add(..coefficients.map(pair => mul(pair.at(0), pair.at(1))))
#assert.eq(canonical(expand(subtract(collected, polynomial))), "0")
#assert.eq(canonical(subtract(x2-coefficient, 1)), "0")
#assert.eq(canonical(expand(subtract(rebuilt, polynomial))), "0")
#assert.eq(
  canonical(collect(parse($(1 + x)^2 x + (1 + y)^100$), x)),
  canonical(parse($x + 2 x^2 + x^3 + (1 + y)^100$)),
)
#assert.eq(summands(polynomial).len(), 4)
#assert.eq(indeterminates(polynomial).len(), 2)
#assert(contains(polynomial, x))
#assert(not contains(parse($x y z$), parse($x y$)))
#assert(is-constant(parse($cos(2) + 1/3$)))
#assert(not is-constant(parse($x + 1$)))
#let decimal-third = to-float(parse($1/3$), decimal-prec: 6)

#let rational = parse($((x + 3) (2 x + 5)) / (x^3 + 6 x^2 + 11 x + 6)$)
#let rational-reduced = cancel-factors(rational)
#let rational-modes = apart(rational-reduced, x)
#assert.eq(canonical(together(subtract(rational-modes, rational-reduced))), "0")
#assert.eq(canonical(expand(subtract(factor(parse($x^2 + 1$), complex: true), parse($x^2 + 1$)))), "0")
#assert.eq(canonical(expand(subtract(factor(parse($(x^2 - 1)^2$), square-free: true), parse($(x^2 - 1)^2$)))), "0")

#let builtin = init(namespace: "symbolica")
#let bparse = builtin.parse
#let bsymbol = builtin.literal
#let bseries = builtin.series
#let bto-typst = builtin.to-typst
#let bto-float = builtin.to-float
#let bcanonical = builtin.canonical
#let sx = bsymbol("x")
#let ser = bseries(bparse($cos(x)/(x + 1)$), sx, 0, 3)
#let builtin-input = bparse($cos(1/3) + 1/2$)
#let builtin-approximation = bto-float(builtin-input, decimal-prec: 6)
#let exact-half-cosine = bparse($cos(1/2)$)
#let exact-half-cosine-approximation = bto-float(exact-half-cosine, decimal-prec: 6)
#let symbolic-approximation = bto-float(bparse($x/3 + cos(1/3)$), decimal-prec: 6)
#let complex-approximation = bto-float(bparse($(-1)^(1/2)$), decimal-prec: 6)
#let rerounded-approximation = bto-float(bparse($1.23456789$), decimal-prec: 3)
#let huge-approximation = bto-float(bparse($10^400$), decimal-prec: 6)
#let tiny-approximation = bto-float(bparse($10^(-400)$), decimal-prec: 6)
#let huge-rerounded = bto-float(
  bto-float(bparse($1234567890123456 10^385$), decimal-prec: 16),
  decimal-prec: 3,
)
#let tiny-rerounded = bto-float(
  bto-float(bparse($1234567890123456 / 10^415$), decimal-prec: 16),
  decimal-prec: 3,
)
#let huge-builtin-approximation = bto-float(bparse($sinh(1000)$), decimal-prec: 6)
#let mixed-unsafe = bparse($x/3 + sinh(1000)$)
#let mixed-float-unsafe = bparse($0.5 x + sinh(1000)$)
#assert.eq(bcanonical(bparse($0.5$)), "0.5")
#assert.eq(bcanonical(exact-half-cosine), "cos(1/2)")
#assert.eq(bcanonical(builtin-approximation), "1.44496")
#assert.eq(bcanonical(exact-half-cosine-approximation), "0.877583")
#assert.eq(bcanonical(rerounded-approximation), "1.23")
#assert.eq(bcanonical(huge-approximation), "(1*10^(400))")
#assert.eq(bcanonical(tiny-approximation), "(1*10^(-400))")
#assert.eq(bcanonical(huge-rerounded), "(1.23*10^(400))")
#assert.eq(bcanonical(tiny-rerounded), "(1.23*10^(-400))")
#assert.eq(
  bcanonical(huge-builtin-approximation),
  bcanonical(bparse($sinh(1000)$)),
)
#assert.eq(bcanonical(bto-float(mixed-unsafe, decimal-prec: 3)), bcanonical(mixed-unsafe))
#assert.eq(
  bcanonical(bto-float(mixed-float-unsafe, decimal-prec: 3)),
  bcanonical(mixed-float-unsafe),
)

#let expr = parse($f(x, y) + x$)
#let swapped = replace(expr, parse($f("a_", "b_")$), parse($g("b_", "a_")$))
#let wildcarded = replace-wildcards(parse($h("a_")$), ((wild("a"), parse($x + 1$)),))
#let rhs-only = replace(parse($f(x)$), parse($f("a_")$), parse($g("a_", "fresh_")$), allow-new-wildcards-on-rhs: true)

#let exact-set = solve((parse($2 x + y - 5$), parse($x - y - 1$)), (x, y))
#let exact-branch = exact-set.branches.first()
#let exact = exact-branch.values
#assert.eq(exact-set.coverage, "complete")
#assert.eq(exact-set.variables.map(canonical), ("x", "y"))
#assert.eq(exact-set.parameters, ())
#assert.eq(exact-set.dimension, 0)
#assert.eq(exact.map(canonical), ("2", "1"))
#assert.eq(exact-branch.dimension, 0)
#assert.eq(exact-branch.codimension, 2)
#assert(exact-branch.point and not exact-branch.conditional)
#let nonlinear = solve((parse($x + y$), parse($y^2 - 2$)), (x, y), domain: "real")
#assert.eq(nonlinear.coverage, "complete")
#assert.eq(nonlinear.branches.len(), 2)

// Free coordinates retain their position in values rather than disappearing.
#let family = solve((parse($x + y - 1$),), (x, y))
#let family-branch = family.branches.first()
#assert.eq(family.dimension, 1)
#assert.eq(family-branch.dimension, 1)
#assert.eq(family-branch.codimension, 1)
#assert.eq(family-branch.free-variables.map(canonical), ("y",))
#assert.eq(canonical(family-branch.values.at(1)), "y")
#assert.eq(canonical(add(family-branch.values.at(0), y)), "1")
#assert(not family-branch.point)
#let rational-family = solve((parse($x + y - 1$),), (x, y), domain: "rational")
#assert.eq(rational-family.dimension, none)
#assert.eq(rational-family.branches.first().dimension, none)
#assert.eq(rational-family.branches.first().codimension, none)

// The a=0 case is not covered: its whole x-axis must not disappear silently.
#let generic = solve((parse($a x$),), (x,))
#assert.eq(generic.coverage, "generic")
#assert.eq(generic.parameters.map(canonical), ("a",))
#assert.eq(generic.coverage-guard.map(canonical), ("a",))
#assert.eq(generic.dimension, none)
#assert.eq(generic.branches.first().values.map(canonical), ("0",))

// An additional parameter equation survives as a branch condition.
#let conditional = solve((parse($x - a$), parse($x - b$)), (x,))
#assert.eq(conditional.coverage, "complete")
#assert.eq(conditional.dimension, none)
#assert(conditional.branches.first().conditional)
#assert(not conditional.branches.first().point)
#assert(conditional.branches.first().conditions.any(condition => condition.kind == "zero"))
#let empty = solve((parse($x^2 + 1$),), (x,), domain: "real")
#assert.eq(empty.coverage, "complete")
#assert.eq(empty.branches, ())
#assert.eq(empty.dimension, -1)
#let root = nsolve(parse($x^2 - 2$), x, 1.0)
#let roots = nsolve-system((parse($x^2 + y - 3$), parse($x - y$)), (x, y), (1.0, 1.0))

#let A = matrix($mat(2, 1; 1, -1)$)
#let b = vector($vec(5, 1)$)
#let B = matrix(((1, 2), (3, 4)))
#let solved = matrix-solve(A, b)
#let reduced = row-reduce(A)
#let augmented = augment(A, b)
#let zero-matrix = matrix-sub(A, A)
#let diagonal = matrix(((x, 0), (0, parse($x^2$))))
#let diagonal-prime = matrix-derivative(diagonal, x)
#assert(matrix-is-zero(zero-matrix))
#assert(matrix-is-diagonal(diagonal))
#assert.eq(canonical(matrix-at(diagonal-prime, 1, 1)), canonical(parse($2 x$)))

Namespaces: #raw(canonical(namespaced, namespaces: true))

Tagged symbol: #raw(canonical(tagged-expression, namespaces: true))

Evaluate: #repr(value)

Evaluate many: #repr(many)

Evaluate grid: shape #repr(grid.shape), first #repr(grid.values.first())

Rational modes: #to-typst(rational-modes)

Coefficient pairs: #coefficients.map(pair => [#to-typst(pair.at(0)): #to-typst(pair.at(1))]).join[, ]

Decimal third: #to-typst(decimal-third)

Built-in approximation: #bto-typst(builtin-approximation)

Symbolic approximation: #bto-typst(symbolic-approximation)

Complex approximation: #bto-typst(complex-approximation)

Re-rounded approximation: #bto-typst(rerounded-approximation)

Large approximation: #bto-typst(huge-approximation)

Small approximation: #bto-typst(tiny-approximation)

Large built-in approximation: #bto-typst(huge-builtin-approximation)

Series: #bto-typst(ser)

Replace: #to-typst(swapped)

Wildcard replace: #to-typst(wildcarded)

RHS-only wildcard: #to-typst(rhs-only)

Exact solve: #exact.map(to-typst).join[, ]

Nonlinear solve: #repr(nonlinear.branches.map(solution => solution.values.map(to-typst)))

Numeric solve: #repr(root), #repr(roots)

Matrix A: #to-typst(A)

Matrix B: #to-typst(B)

A shape: #repr(matrix-shape(A)); A[0, 1]: #to-typst(matrix-at(A, 0, 1))

Solve A x = b: #to-typst(solved)

Det: #to-typst(determinant(A))

Inverse: #to-typst(inv(A))

Transpose: #to-typst(transpose(A))

A times I: #to-typst(matrix-mul(A, identity(2)))

Eye: #to-typst(eye((1, 2)))

Augment: #to-typst(augmented)

Row-reduce rank: #reduced.rank; matrix: #to-typst(reduced.matrix)

Matrix derivative: #to-typst(diagonal-prime)

// Direct built-ins select Symbolica's function identity even when arguments
// come from a custom namespace. Check the module and engine surfaces alike.
#import "../lib.typ" as public
#let direct-specials = (
  gamma-function: public.gamma-function,
  polygamma: public.polygamma,
  polylog: public.polylog,
  zeta-function: public.zeta-function,
  bessel-j: public.bessel-j,
  bessel-y: public.bessel-y,
  bessel-i: public.bessel-i,
  bessel-k: public.bessel-k,
)
#let special-cases = (
  (name: "gamma-function", native: "gamma", orders: (), point: public.divide(5, 6), value: 1.128787029908126),
  (name: "polygamma", native: "polygamma", orders: (1,), point: 1, value: calc.pi * calc.pi / 6),
  (name: "polylog", native: "polylog", orders: (2,), point: public.divide(1, 2), value: calc.pi * calc.pi / 12 - calc.pow(calc.ln(2), 2) / 2),
  (name: "zeta-function", native: "zeta", orders: (), point: 3, value: 1.202056903159594),
  (name: "bessel-j", native: "bessel_j", orders: (0,), point: 1, value: 0.7651976865579666),
  (name: "bessel-y", native: "bessel_y", orders: (0,), point: 1, value: 0.08825696421567696),
  (name: "bessel-i", native: "bessel_i", orders: (0,), point: 1, value: 1.2660658777520084),
  (name: "bessel-k", native: "bessel_k", orders: (0,), point: 1, value: 0.42102443824070833),
)
#for (api, namespace) in ((direct-specials, "typst"), (init(namespace: "special_model"), "special_model")) {
  for case in special-cases {
    let constructor = api.at(case.name)
    let call = constructor(..case.orders, "x")
    let arguments = (case.orders.map(str) + (namespace + "::x",)).join(",")
    let expected = "symbolica::" + case.native + "(" + arguments + ")"
    assert.eq(type(call), type([]))
    assert.eq(canonical(call, namespaces: true), expected)
    assert.eq(canonical(parse($#call$), namespaces: true), expected)
    assert.eq(canonical(parse($#to-typst(call)$), namespaces: true), expected)
    let value = evaluate(constructor(..case.orders, case.point))
    assert(calc.abs(value.re - case.value) < 1e-10, message: case.name)
    assert(calc.abs(value.im) < 1e-10, message: case.name)
  }
}

#let special-x = literal("x", namespace: "special_model")
#let special-derivative = public.derivative(public.polylog(2, special-x), special-x)
#assert.eq(
  canonical(special-derivative, namespaces: true),
  canonical(public.parse($-log(1-#special-x)/#special-x$, namespace: "symbolica"), namespaces: true),
)
#assert.eq(canonical(public.gamma-function(5)), "24")
#assert.eq(canonical(public.zeta-function(2)), canonical(public.parse($pi^2/6$)))
#assert.eq(canonical(public.polygamma(1, 1)), canonical(public.parse($pi^2/6$)))

// Engine notation and exact argument metadata are retained by the wrappers.
#let special-engine = init(namespace: "special_model", notation: public.notation(
  calls: ("symbolica::polylog": ctx => {
    let (order, argument) = ctx.visual-arguments
    $L_(#order)(#argument)$
  }),
))
#let foreign-x = literal("q", namespace: "external")
#let styled-special = (special-engine.polylog)(2, foreign-x)
#assert.eq(canonical(parse($#styled-special$), namespaces: true), "symbolica::polylog(2,external::q)")
#assert.eq(
  canonical((special-engine.polylog)(2, public.parse(foreign-x)), namespaces: true),
  canonical(styled-special, namespaces: true),
)
#context {
  let actual = measure($#styled-special$)
  let expected = measure($L_2(q)$)
  assert(calc.abs(actual.width - expected.width) < 0.01pt)
  assert(calc.abs(actual.height - expected.height) < 0.01pt)
}
