// Maintainer regression coverage for the public core API.
#import "../lib.typ": init

#let sym = init()
#assert("integrate" in sym)
#assert("integrate-with-steps" in sym)
#assert("symbol" in sym)
#assert("function" in sym)
#assert("var" not in sym)
#let parse = sym.math
#let symbol = sym.symbol
#let symbolic-function = sym.function
#let wild = sym.wild
#let to-typst = sym.to-typst
#let to-float = sym.to-float
#let canonical = sym.canonical
#let add = sym.add
#let mul = sym.mul
#let sub = sym.sub
#let expand = sym.expand
#let factor = sym.factor
#let together = sym.together
#let cancel = sym.cancel
#let apart = sym.apart
#let collect = sym.collect
#let coefficient = sym.coefficient
#let coefficient-list = sym.coefficient-list
#let terms = sym.terms
#let indeterminates = sym.indeterminates
#let contains = sym.contains
#let is-constant = sym.is-constant
#let replace = sym.replace
#let replace-wildcards = sym.replace-wildcards
#let series = sym.series
#let evaluate = sym.evaluate
#let pi-value = evaluate((sym.math)($pi^2 + sin(pi / 4)$))
#assert(calc.abs(pi-value.re - (calc.pi * calc.pi + calc.sin(calc.pi / 4))) < 1e-12)
#assert.eq(pi-value.im, 0.0)
#let domain = sym.domain
#let evaluate-many = sym.evaluate-many
#let evaluate-grid = sym.evaluate-grid
#let solve = sym.solve
#let nsolve = sym.nsolve
#let nsolve-system = sym.nsolve-system
#let matrix = sym.matrix
#let make-vec = sym.vec
#let identity = sym.identity
#let eye = sym.eye
#let matrix-solve = sym.matrix-solve
#let matrix-sub = sym.matrix-sub
#let matrix-mul = sym.matrix-mul
#let det = sym.det
#let inv = sym.inv
#let transpose = sym.transpose
#let augment = sym.augment
#let row-reduce = sym.row-reduce
#let matrix-at = sym.matrix-at
#let matrix-shape = sym.matrix-shape
#let matrix-is-zero = sym.matrix-is-zero
#let matrix-is-diagonal = sym.matrix-is-diagonal
#let matrix-derivative = sym.matrix-derivative

#let x = symbol("x")
#let y = symbol("y")
#let xp = symbol("x", namespace: "physics")
#let namespaced = add(xp, x)
#let tagged = symbol("q", namespace: "model", tags: ("model::positive", "model::parameter"))
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
#assert.eq(canonical(expand(sub(collected, polynomial))), "0")
#assert.eq(canonical(sub(x2-coefficient, 1)), "0")
#assert.eq(canonical(expand(sub(rebuilt, polynomial))), "0")
#assert.eq(
  canonical(collect(parse($(1 + x)^2 x + (1 + y)^100$), x)),
  canonical(parse($x + 2 x^2 + x^3 + (1 + y)^100$)),
)
#assert.eq(terms(polynomial).len(), 4)
#assert.eq(indeterminates(polynomial).len(), 2)
#assert(contains(polynomial, x))
#assert(not contains(parse($x y z$), parse($x y$)))
#assert(is-constant(parse($cos(2) + 1/3$)))
#assert(not is-constant(parse($x + 1$)))
#let decimal-third = to-float(parse($1/3$), decimal-prec: 6)

#let rational = parse($((x + 3) (2 x + 5)) / (x^3 + 6 x^2 + 11 x + 6)$)
#let rational-reduced = cancel(rational)
#let rational-modes = apart(rational-reduced, x)
#assert.eq(canonical(together(sub(rational-modes, rational-reduced))), "0")
#assert.eq(canonical(expand(sub(factor(parse($x^2 + 1$), complex: true), parse($x^2 + 1$)))), "0")
#assert.eq(canonical(expand(sub(factor(parse($(x^2 - 1)^2$), square-free: true), parse($(x^2 - 1)^2$)))), "0")

#let builtin = init(namespace: "symbolica")
#let bparse = builtin.math
#let bsymbol = builtin.symbol
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
#let b = make-vec($vec(5, 1)$)
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

Det: #to-typst(det(A))

Inverse: #to-typst(inv(A))

Transpose: #to-typst(transpose(A))

A times I: #to-typst(matrix-mul(A, identity(2)))

Eye: #to-typst(eye((1, 2)))

Augment: #to-typst(augmented)

Row-reduce rank: #reduced.rank; matrix: #to-typst(reduced.matrix)

Matrix derivative: #to-typst(diagonal-prime)
