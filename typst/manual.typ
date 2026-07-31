#import "@preview/tidy:0.4.3"
#import "lib.typ" as symbolica

#set document(title: "Tymbolica")
#set page(margin: 22mm)
#set text(font: "Libertinus Serif", size: 10.5pt)
#set heading(numbering: "1.1")

#let example-preamble = (
  "#let _sym = symbolica.init()",
  "#let init = symbolica.init",
  "#let math = _sym.math",
  "#let atom = _sym.atom",
  "#let var = _sym.var",
  "#let wild = _sym.wild",
  "#let array-tree = _sym.array-tree",
  "#let canonical = _sym.canonical",
  "#let to-typst-source = _sym.to-typst-source",
  "#let to-typst = _sym.to-typst",
  "#let to-latex = _sym.to-latex",
  "#let simplify = _sym.simplify",
  "#let expand = _sym.expand",
  "#let factor = _sym.factor",
  "#let derivative = _sym.derivative",
  "#let integrate = _sym.integrate",
  "#let integrate-with-steps = _sym.integrate-with-steps",
  "#let series = _sym.series",
  "#let rule = _sym.rule",
  "#let replace = _sym.replace",
  "#let replace-multiple = _sym.replace-multiple",
  "#let replace-wildcards = _sym.replace-wildcards",
  "#let evaluate = _sym.evaluate",
  "#let domain = _sym.domain",
  "#let evaluate-many = _sym.evaluate-many",
  "#let evaluate-grid = _sym.evaluate-grid",
  "#let solve-linear = _sym.solve-linear",
  "#let solve-system = _sym.solve-system",
  "#let nsolve = _sym.nsolve",
  "#let nsolve-system = _sym.nsolve-system",
  "#let matrix = _sym.matrix",
  "#let vec = _sym.vec",
  "#let identity = _sym.identity",
  "#let eye = _sym.eye",
  "#let matrix-add = _sym.matrix-add",
  "#let matrix-sub = _sym.matrix-sub",
  "#let matrix-mul = _sym.matrix-mul",
  "#let matrix-div-scalar = _sym.matrix-div-scalar",
  "#let transpose = _sym.transpose",
  "#let det = _sym.det",
  "#let inv = _sym.inv",
  "#let matrix-solve = _sym.matrix-solve",
  "#let matrix-solve-any = _sym.matrix-solve-any",
  "#let row-reduce = _sym.row-reduce",
  "#let augment = _sym.augment",
  "#let split-col = _sym.split-col",
  "#let primitive-part = _sym.primitive-part",
  "#let content = _sym.content",
  "#let matrix-at = _sym.matrix-at",
  "#let matrix-shape = _sym.matrix-shape",
  "#let add = _sym.add",
  "#let mul = _sym.mul",
  "#let neg = _sym.neg",
  "#let sub = _sym.sub",
  "#let div = _sym.div",
  "#let pow = _sym.pow",
).join("\n") + "\n"

#let docs = tidy.parse-module(read("lib.typ"), name: "symbolica", scope: (symbolica: symbolica), preamble: example-preamble)
#let worked-example(code) = tidy.styles.default.show-example(
  raw(code.text, lang: "typ", block: true),
  scope: (symbolica: symbolica),
  preamble: example-preamble,
  mode: "markup",
  dir: ttb,
  scale-preview: 100%,
)

= Tymbolica

`symbolica` exposes Symbolica's exact algebra engine to Typst through a WebAssembly plugin. Expressions and matrices are passed around as opaque byte payloads; use `to-typst` for document output and `canonical` for portable text.

== Setup

```typst
#import "@local/tymbolica:0.1.0": *

#let expr = math($(x + 1)^2$)
#to-typst(expand(expr))
```

For a source checkout, import the local library directly:

```typst
#import "lib.typ": *
```

== Worked API Examples

Tidy evaluates each example below and places its rendered output beside the source. Lines beginning with `>>>` provide hidden setup shared only within that example.

#show raw.where(lang: "worked"): worked-example

=== Engine, Parsing, and Symbols

```worked
#let sym = init(namespace: "physics")
#let svar = sym.var

#let x = var("x")
#let y = var("y")
#let a-wild = wild("a")
#let rhs-only = wild("fresh")
#let expr = math($(x + 1)^2 + y$)
#let literal = atom("z")
#let namespaced = add(svar("x"), x)

expr: #to-typst(expr)\
literal: #to-typst(literal)\
wildcards: #raw(canonical(a-wild)) and #raw(canonical(rhs-only))\
namespaces: #raw(canonical(namespaced, namespaces: true))
```

=== Parse Tree Inspection

```worked
#array-tree($(x + 1)^2$)
```

=== Rendering

```worked
#let expr = math($(x + 1)^2 + y$)

canonical: #raw(canonical(expr))\
to-typst-source: #raw(to-typst-source(expr))\
to-typst: #to-typst(expr)\
to-latex: #raw(to-latex(expr))
```

=== Algebra and Calculus

```worked
>>>#let x = var("x")
#let p = math($x^2 + 2 x + 1$)
#let q = math($(x + 1) (y + 2)$)
#let builtin = init(namespace: "symbolica")
#let bmath = builtin.math
#let bvar = builtin.var
#let bseries = builtin.series
#let bto-typst = builtin.to-typst
#let bx = bvar("x")
#let ser = bseries(bmath($cos(x)/(x + 1)$), bx, 0, 3)
#let integration = integrate-with-steps(p, x)

simplify: #to-typst(simplify(p))\
expand: #to-typst(expand(q))\
factor: #to-typst(factor(p))\
derivative: #to-typst(derivative(q, x))\
integral: #to-typst(integration.result)\
integration steps: #integration.steps.map(to-typst).join[, ]\
series: #bto-typst(ser)
```

=== Scalar Constructors and Arithmetic

```worked
>>>#let x = var("x")
>>>#let y = var("y")
>>>#let z = var("z")
#let arith = add(
  mul(2, x, y),
  neg(z),
  sub(x, y),
  div(1, x),
  pow(x, 3),
)
#to-typst(arith)
```

=== Replacement

```worked
>>>#let x = var("x")
>>>#let z = var("z")
#let src = math($f(x, y) + x$)
#let swapped = replace(src, math($f("a_", "b_")$), math($g("b_", "a_")$))
#let r1 = rule(math($f("a_")$), math($h("a_")$))
#let r2 = rule(x, z)
#let multiple = replace-multiple(math($f(x) + x$), (r1, r2))
#let wild-replaced = replace-wildcards(math($k("a_")$), ((wild("a"), math($x + 1$)),))

replace: #to-typst(swapped)\
replace-multiple: #to-typst(multiple)\
replace-wildcards: #to-typst(wild-replaced)
```

=== Evaluation and Solving

```worked
>>>#let x = var("x")
>>>#let y = var("y")
#let value = evaluate(math($x^2 + y$), values: ((x, 2.0), (y, 3.0)))
#let complex = evaluate(math($x^2$), values: ((x, (re: 2.0, im: 1.0)),))
#let many = evaluate-many((math($x + y$), math($x y$)), (x, y), ((1, 2), (3, 4)))
#let grid = evaluate-grid(math($x^2 + y$), (x, y), (domain(-1, 1, samples: 3), domain(0, 1, samples: 2)))
#let exact = solve-linear((math($2 x + y - 5$), math($x - y - 1$)), (x, y))
#let nonlinear = solve-system((math($x + y$), math($y^2 - 2$)), (x, y))
#let root = nsolve(math($x^2 - 2$), x, 1.0)
#let roots = nsolve-system((math($x^2 + y - 3$), math($x - y$)), (x, y), (1.0, 1.0))

value: #repr(value)\
complex: #repr(complex)\
evaluate-many: #repr(many)\
evaluate-grid shape: #repr(grid.shape); first: #repr(grid.values.first())\
solve-linear: #exact.map(to-typst).join[, ]\
solve-system: #nonlinear.map(row => row.map(to-typst).join[, ]).join[; ]\
nsolve: #repr(root)\
nsolve-system: #repr(roots)
```

=== Matrix Construction and Rendering

```worked
#let A = matrix($mat(2, 1; 1, -1)$)
#let B = matrix(((1, 2), (3, 4)))
#let b = vec((5, 1))
#let I = identity(2)
#let D = eye((1, 2))

A: #to-typst(A)\
B: #to-typst(B)\
b: #to-typst(b)\
I: #to-typst(I)\
D: #to-typst(D)
```

=== Matrix Operations

```worked
>>>#let A = matrix($mat(2, 1; 1, -1)$)
>>>#let B = matrix(((1, 2), (3, 4)))
>>>#let b = vec((5, 1))
>>>#let I = identity(2)
#let solved = matrix-solve(A, b)
#let any = matrix-solve-any(A, b)
#let reduced = row-reduce(B)
#let aug = augment(A, B)
#let parts = split-col(aug, 2)

add: #to-typst(matrix-add(A, B))\
sub: #to-typst(matrix-sub(B, A))\
mul: #to-typst(matrix-mul(A, I))\
scalar mul: #to-typst(matrix-mul(A, 3))\
scalar div: #to-typst(matrix-div-scalar(B, 2))\
transpose: #to-typst(transpose(A))\
det: #to-typst(det(A))\
inv: #to-typst(inv(A))\
solve: #to-typst(solved)\
solve-any: #to-typst(any)\
row-reduce: rank #reduced.rank, #to-typst(reduced.matrix)\
augment: #to-typst(aug)\
split-col: #to-typst(parts.at(0)) | #to-typst(parts.at(1))
```

=== Matrix Entries and Polynomial Content

```worked
>>>#let x = var("x")
>>>#let A = matrix($mat(2, 1; 1, -1)$)
#let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))

primitive-part: #to-typst(primitive-part(P))\
content: #to-typst(content(P))\
matrix-at: #to-typst(matrix-at(A, 0, 1))\
matrix-shape: #repr(matrix-shape(A))
```

== API Reference

The reference below is generated from the doc comments in `lib.typ` with `tidy`. Its examples use a preamble equivalent to importing the package with `: *`.

#tidy.show-module(
  docs,
  style: tidy.styles.default,
  omit-private-definitions: true,
  first-heading-level: 2,
)
