#import "@preview/parsely:0.1.0"

#let _default_grammar = (
  arg: (infix: $,$, assoc: true, prec: 4),
  add: (infix: $+$, prec: 1, assoc: true),
  sub: (infix: $-$, prec: 1),
  plus: (prefix: $+$, prec: 2),
  neg: (prefix: $-$, prec: 2),
  times: (infix: $times$, prec: 2),
  dot: (infix: $dot$, prec: 2),
  factorial: (postfix: $#parsely.tight !$, prec: 3),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  pow: (match: $#parsely.slot("base")^#parsely.slot("exp")$),
  union: (infix: $union$, prec: 1),
  inter: (infix: $inter$, prec: 1),
  attach: math.attach,
  frac: math.frac,
  lr: math.lr,
  mat: math.mat,
  vec: math.vec,
  root: math.root,
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  call: (match: $#parsely.slot("fn") #parsely.tight (#parsely.slot("body*"))$),
  op: math.op,
)
#let _typst_math = math

#let _leaf(value) = {
  if type(value) == str {
    value
  } else if type(value) == content {
    let fields = value.fields()
    if "text" in fields {
      fields.text
    } else if "children" in fields {
      fields.children.map(_leaf).join("")
    } else {
      repr(value)
    }
  } else {
    repr(value)
  }
}

#let _node_to_ast(node) = (
  head: node.head,
  args: node.args,
  slots: node.slots,
)

#let _is_space(value) = {
  if type(value) == str { return value.trim() == "" }
  if type(value) == content {
    if repr(value.func()) == "space" { return true }
    if repr(value.func()) == "symbol" and "text" in value.fields() {
      return value.fields().text.trim() == ""
    }
  }
  false
}

#let _content_positional_fields = (
  attach: ("base",),
  equation: ("body",),
  frac: ("num", "denom"),
  lr: ("body",),
  mat: (),
  vec: (),
  root: ("index", "radicand"),
)

#let _call_content(fn, fields) = {
  let kind = repr(fn)
  if kind == "sequence" and "children" in fields {
    return fields.children.join()
  }

  let pos = ()
  for field in _content_positional_fields.at(kind, default: ()) {
    if field in fields {
      pos.push(fields.remove(field))
    }
  }
  fn(..pos, ..fields)
}

#let _trim_math(value) = {
  let trim_array(values) = {
    let values = values.map(_trim_math)
    while values.len() > 0 and _is_space(values.first()) {
      values = values.slice(1)
    }
    while values.len() > 0 and _is_space(values.last()) {
      values = values.slice(0, values.len() - 1)
    }
    values
  }

  if type(value) == array { return trim_array(value) }
  if type(value) != content { return value }

  let kind = repr(value.func())
  if kind != "sequence" and kind not in _content_positional_fields {
    return value
  }

  let fields = (:)
  for (key, field) in value.fields() {
    fields.insert(key, _trim_math(field))
  }
  _call_content(value.func(), fields)
}

#let _namespace(engine, namespace) = if namespace == none { engine.namespace } else { namespace }
#let _namespace_bytes(engine, namespace: none) = cbor.encode(_namespace(engine, namespace))

#let _ast_bytes(eqn, grammar) = {
  let parsed = parsely.parse(_trim_math(eqn), grammar)
  let tree = parsely.walk(parsed.tree, post: _node_to_ast, leaf: _leaf)
  cbor.encode(tree)
}

#let _from_math(engine, eqn, grammar: none, namespace: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  engine.plugin.from_ast(_ast_bytes(eqn, grammar), _namespace_bytes(engine, namespace: namespace))
}

#let _array_tree(engine, eqn, grammar: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  let parsed = parsely.parse(_trim_math(eqn), grammar)
  parsely.walk(parsed.tree, post: it => (
    strong(raw(it.head)),
    ..it.args,
    ..it.slots.pairs().map(((slot, it)) => {
      (text(gray, 0.8em, raw(slot)), it)
    }),
  ), leaf: _typst_math.equation)
}

#let _var(engine, name, namespace: none) = engine.plugin.symbol(cbor.encode(name), _namespace_bytes(engine, namespace: namespace))
#let _wild(engine, name, level: 1, namespace: none) = {
  let suffix = ""
  for _ in range(level) { suffix += "_" }
  _var(engine, name + suffix, namespace: namespace)
}

#let _expr_bytes(engine, expr, namespace: none) = {
  if type(expr) == bytes {
    expr
  } else if type(expr) == content {
    _from_math(engine, expr, namespace: namespace)
  } else {
    engine.plugin.from_ast(cbor.encode(expr), _namespace_bytes(engine, namespace: namespace))
  }
}

#let _atom_array(engine, values, namespace: none) = cbor.encode(values.map(value => _expr_bytes(engine, value, namespace: namespace)))
#let _payload_bytes(engine, value, namespace: none) = if type(value) == bytes { value } else { _expr_bytes(engine, value, namespace: namespace) }

#let _canonical(engine, expr, namespaces: false) = str(engine.plugin.canonical(_payload_bytes(engine, expr), cbor.encode(namespaces)))
#let _to_typst_source(engine, expr) = str(engine.plugin.to_typst(_payload_bytes(engine, expr)))
#let _to_typst(engine, expr, block: false) = {
  let eqn = eval(_to_typst_source(engine, expr), mode: "math")
  if block { _typst_math.equation(eqn.body, block: true) } else { eqn }
}
#let _to_latex(engine, expr) = str(engine.plugin.to_latex(_payload_bytes(engine, expr)))

#let _simplify(engine, expr) = engine.plugin.simplify_expr(_expr_bytes(engine, expr))
#let _expand(engine, expr) = engine.plugin.expand(_expr_bytes(engine, expr))
#let _factor(engine, expr) = engine.plugin.factor(_expr_bytes(engine, expr))
#let _derivative(engine, expr, var) = engine.plugin.derivative(_expr_bytes(engine, expr), _expr_bytes(engine, var))
#let _integrate(engine, expr, var) = engine.plugin.integrate(_expr_bytes(engine, expr), _expr_bytes(engine, var))
#let _integrate_with_steps(engine, expr, var) = cbor(engine.plugin.integrate_with_steps(
  _expr_bytes(engine, expr), _expr_bytes(engine, var),
))
#let _series(engine, expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) = {
  engine.plugin.series(cbor.encode((
    expr: _expr_bytes(engine, expr),
    var: _expr_bytes(engine, var),
    expansion-point: _expr_bytes(engine, expansion-point),
    depth: depth,
    depth-denom: depth-denom,
    depth-is-absolute: depth-is-absolute,
  )))
}

#let _replacement_options(engine,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = (
  non-greedy-wildcards: non-greedy-wildcards.map(w => _expr_bytes(engine, w)),
  min-level: min-level,
  max-level: max-level,
  level-range: level-range,
  level-is-tree-depth: level-is-tree-depth,
  partial: partial,
  allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
  rhs-cache-size: rhs-cache-size,
)

#let _rule(engine, pattern, rhs,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = (
  pattern: _expr_bytes(engine, pattern),
  rhs: _expr_bytes(engine, rhs),
  .._replacement_options(engine,
    non-greedy-wildcards: non-greedy-wildcards,
    min-level: min-level,
    max-level: max-level,
    level-range: level-range,
    level-is-tree-depth: level-is-tree-depth,
    partial: partial,
    allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
    rhs-cache-size: rhs-cache-size,
  ),
)

#let _replace(engine, expr, pattern, rhs,
  repeat: false,
  once: false,
  bottom-up: false,
  nested: false,
  non-greedy-wildcards: (),
  min-level: 0,
  max-level: none,
  level-range: none,
  level-is-tree-depth: false,
  partial: true,
  allow-new-wildcards-on-rhs: false,
  rhs-cache-size: 100,
) = {
  engine.plugin.replace(cbor.encode((
    expr: _expr_bytes(engine, expr),
    once: once,
    repeat: repeat,
    bottom-up: bottom-up,
    nested: nested,
    .._rule(engine, pattern, rhs,
      non-greedy-wildcards: non-greedy-wildcards,
      min-level: min-level,
      max-level: max-level,
      level-range: level-range,
      level-is-tree-depth: level-is-tree-depth,
      partial: partial,
      allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs,
      rhs-cache-size: rhs-cache-size,
    ),
  )))
}

#let _replace_multiple(engine, expr, rules, repeat: false, once: false, bottom-up: false, nested: false) = {
  engine.plugin.replace_multiple(cbor.encode((
    expr: _expr_bytes(engine, expr),
    rules: rules,
    once: once,
    repeat: repeat,
    bottom-up: bottom-up,
    nested: nested,
  )))
}

#let _replace_wildcards(engine, pattern, replacements) = {
  let pairs = replacements.map(pair => (
    _expr_bytes(engine, pair.at(0)),
    _expr_bytes(engine, pair.at(1)),
  ))
  engine.plugin.replace_wildcards(cbor.encode((pattern: _expr_bytes(engine, pattern), replacements: pairs)))
}

#let _eval_value(value) = {
  if type(value) == dictionary and "re" in value {
    (re: value.re, im: value.at("im", default: 0.0))
  } else {
    value
  }
}
#let _eval_values(engine, values) = values.map(pair => (_expr_bytes(engine, pair.at(0)), _eval_value(pair.at(1))))
#let _evaluate(engine, expr, values: ()) = cbor(engine.plugin.evaluate(cbor.encode((expr: _expr_bytes(engine, expr), values: _eval_values(engine, values)))))

#let _domain(min, max, samples: 200) = {
  assert(min < max, message: "min must be less than max")
  assert(samples > 0, message: "samples must be positive")
  (min: float(min), max: float(max), samples: samples)
}

#let _expr_array(engine, values) = {
  let value = values
  if type(value) == bytes { return value }
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _expr_array(engine, fields.body) }
    if kind == "vec" and "children" in fields {
      return fields.children.map(child => _expr_bytes(engine, child))
    }
  }
  if type(value) == array { return value.map(item => _expr_bytes(engine, item)) }
  ( _expr_bytes(engine, value), )
}

#let _evaluate_many(engine, expressions, variables, points) = {
  let expressions = _expr_array(engine, expressions)
  let variables = _expr_array(engine, variables)
  let variable-count = if type(variables) == bytes { 1 } else { variables.len() }
  let points = points.map(point => {
    let point = if variable-count == 1 and type(point) != array { (point,) } else { point }
    assert.eq(type(point), array, message: "each point must be an array")
    point.map(_eval_value)
  })
  cbor(engine.plugin.evaluate_many(cbor.encode((
    expressions: expressions,
    variables: variables,
    points: points,
  ))))
}

#let _evaluate_grid(engine, expressions, variables, domains) = {
  cbor(engine.plugin.evaluate_grid(cbor.encode((
    expressions: _expr_array(engine, expressions),
    variables: _expr_array(engine, variables),
    domains: domains,
  ))))
}

#let _solve_linear(engine, system, variables) = cbor(engine.plugin.solve_linear(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
))))
#let _solve_system(engine, system, variables) = cbor(engine.plugin.solve_system(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
))))
#let _nsolve(engine, expr, var, init, prec: 1e-4, max-iterations: 1000) = cbor(engine.plugin.nsolve(cbor.encode((
  expr: _expr_bytes(engine, expr),
  var: _expr_bytes(engine, var),
  init: init,
  prec: prec,
  max-iterations: max-iterations,
))))
#let _nsolve_system(engine, system, variables, init, prec: 1e-4, max-iterations: 1000) = cbor(engine.plugin.nsolve_system(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
  init: init,
  prec: prec,
  max-iterations: max-iterations,
))))

#let _matrix_rows(engine, value) = {
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _matrix_rows(engine, fields.body) }
    if kind == "mat" and "rows" in fields {
      return fields.rows.map(row => row.map(cell => _expr_bytes(engine, cell)))
    }
    if kind == "vec" and "children" in fields {
      return fields.children.map(cell => (_expr_bytes(engine, cell),))
    }
  }
  if type(value) == array {
    if value.len() == 0 { return () }
    if type(value.first()) == array {
      return value.map(row => row.map(cell => _expr_bytes(engine, cell)))
    }
    return value.map(cell => (_expr_bytes(engine, cell),))
  }
  ((_expr_bytes(engine, value),),)
}

#let _matrix(engine, value) = {
  if type(value) == bytes { value } else { engine.plugin.matrix_from_nested(cbor.encode(_matrix_rows(engine, value))) }
}
#let _matrix_source(value) = {
  if type(value) == bytes { return true }
  if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "equation" { return _matrix_source(fields.body) }
    return kind == "mat" or kind == "vec"
  }
  type(value) == array and value.len() > 0 and type(value.first()) == array
}
#let _vec(engine, values) = {
  if type(values) == bytes or type(values) == content { return _matrix(engine, values) }
  engine.plugin.matrix_vec(cbor.encode(values.map(value => _expr_bytes(engine, value))))
}
#let _identity(engine, n) = engine.plugin.matrix_identity(cbor.encode(n))
#let _eye(engine, diag) = engine.plugin.matrix_eye(_atom_array(engine, diag))
#let _matrix_add(engine, lhs, rhs) = engine.plugin.matrix_add(_matrix(engine, lhs), _matrix(engine, rhs))
#let _matrix_sub(engine, lhs, rhs) = engine.plugin.matrix_sub(_matrix(engine, lhs), _matrix(engine, rhs))
#let _matrix_mul(engine, lhs, rhs) = {
  let rhs = if _matrix_source(rhs) { _matrix(engine, rhs) } else { _expr_bytes(engine, rhs) }
  engine.plugin.matrix_mul(_matrix(engine, lhs), rhs)
}
#let _matrix_div_scalar(engine, lhs, rhs) = engine.plugin.matrix_div_scalar(_matrix(engine, lhs), _expr_bytes(engine, rhs))
#let _transpose(engine, matrix) = engine.plugin.transpose(_matrix(engine, matrix))
#let _det(engine, matrix) = engine.plugin.det(_matrix(engine, matrix))
#let _inv(engine, matrix) = engine.plugin.inv(_matrix(engine, matrix))
#let _matrix_solve(engine, A, b) = engine.plugin.matrix_solve(_matrix(engine, A), _matrix(engine, b))
#let _matrix_solve_any(engine, A, b) = engine.plugin.matrix_solve_any(_matrix(engine, A), _matrix(engine, b))
#let _row_reduce(engine, matrix, max-col: none) = {
  let request = (matrix: _matrix(engine, matrix))
  if max-col != none { request.insert("max-col", max-col) }
  cbor(engine.plugin.row_reduce(cbor.encode(request)))
}
#let _augment(engine, lhs, rhs) = engine.plugin.augment(_matrix(engine, lhs), _matrix(engine, rhs))
#let _split_col(engine, matrix, index) = cbor(engine.plugin.split_col(cbor.encode((matrix: _matrix(engine, matrix), index: index))))
#let _primitive_part(engine, matrix) = engine.plugin.primitive_part(_matrix(engine, matrix))
#let _content(engine, matrix) = engine.plugin.content(_matrix(engine, matrix))
#let _matrix_at(engine, matrix, row, col) = engine.plugin.matrix_at(cbor.encode((matrix: _matrix(engine, matrix), row: row, col: col)))
#let _matrix_shape(engine, matrix) = cbor(engine.plugin.matrix_shape(_matrix(engine, matrix)))

#let _add(engine, ..terms) = engine.plugin.add(_atom_array(engine, terms.pos()))
#let _mul(engine, ..factors) = engine.plugin.mul(_atom_array(engine, factors.pos()))
#let _neg(engine, expr) = engine.plugin.neg(_expr_bytes(engine, expr))
#let _sub(engine, lhs, rhs) = engine.plugin.sub(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _div(engine, lhs, rhs) = engine.plugin.div(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _pow(engine, base, exp) = engine.plugin.power(_expr_bytes(engine, base), _expr_bytes(engine, exp))

/// Create an independent set of Tymbolica functions.
///
/// The returned dictionary exposes the same parsing, algebra, evaluation,
/// solving, and matrix operations as the top-level API. Use it when you need a
/// different symbol namespace, plugin location, or parser grammar; ordinary
/// calculations can use the imported top-level functions directly.
///
/// ```example
/// #let sym = init(namespace: "physics")
/// #let v = sym.var
/// #let render = sym.canonical
/// #raw(render(v("x"), namespaces: true))
/// ```
///
/// -> dictionary
#let init(
  /// Default namespace for symbols parsed from Typst math or strings. A
  /// per-call `namespace` passed to `math` or `var` takes precedence.
  /// -> str
  namespace: "typst",
  /// WebAssembly plugin path passed to Typst's `plugin` constructor. Relative
  /// paths are resolved by Typst from this source file.
  /// -> str
  source: "tymbolica.wasm",
  /// Parser grammar used by `math` and `array-tree` unless they receive an
  /// explicit override.
  /// -> dictionary
  grammar: _default_grammar,
) = {
  let engine = (
    plugin: plugin(source),
    grammar: grammar,
    namespace: namespace,
  )

  (
    math: (eqn, grammar: none, namespace: none) => _from_math(engine, eqn, grammar: grammar, namespace: namespace),
    atom: value => _expr_bytes(engine, value),
    var: (name, namespace: none) => _var(engine, name, namespace: namespace),
    wild: (name, level: 1, namespace: none) => _wild(engine, name, level: level, namespace: namespace),
    array-tree: (eqn, grammar: none) => _array_tree(engine, eqn, grammar: grammar),
    canonical: (expr, namespaces: false) => _canonical(engine, expr, namespaces: namespaces),
    to-typst-source: expr => _to_typst_source(engine, expr),
    to-typst: (expr, block: false) => _to_typst(engine, expr, block: block),
    to-latex: expr => _to_latex(engine, expr),
    simplify: expr => _simplify(engine, expr),
    expand: expr => _expand(engine, expr),
    factor: expr => _factor(engine, expr),
    derivative: (expr, var) => _derivative(engine, expr, var),
    integrate: (expr, var) => _integrate(engine, expr, var),
    integrate-with-steps: (expr, var) => _integrate_with_steps(engine, expr, var),
    series: (expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) => _series(engine, expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute),
    rule: (pattern, rhs, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _rule(engine, pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace: (expr, pattern, rhs, repeat: false, once: false, bottom-up: false, nested: false, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _replace(engine, expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace-multiple: (expr, rules, repeat: false, once: false, bottom-up: false, nested: false) => _replace_multiple(engine, expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested),
    replace-wildcards: (pattern, replacements) => _replace_wildcards(engine, pattern, replacements),
    evaluate: (expr, values: ()) => _evaluate(engine, expr, values: values),
    domain: _domain,
    evaluate-many: (expressions, variables, points) => _evaluate_many(engine, expressions, variables, points),
    evaluate-grid: (expressions, variables, domains) => _evaluate_grid(engine, expressions, variables, domains),
    solve-linear: (system, variables) => _solve_linear(engine, system, variables),
    solve-system: (system, variables) => _solve_system(engine, system, variables),
    nsolve: (expr, var, init, prec: 1e-4, max-iterations: 1000) => _nsolve(engine, expr, var, init, prec: prec, max-iterations: max-iterations),
    nsolve-system: (system, variables, init, prec: 1e-4, max-iterations: 1000) => _nsolve_system(engine, system, variables, init, prec: prec, max-iterations: max-iterations),
    matrix: value => _matrix(engine, value),
    vec: values => _vec(engine, values),
    identity: n => _identity(engine, n),
    eye: diag => _eye(engine, diag),
    matrix-add: (lhs, rhs) => _matrix_add(engine, lhs, rhs),
    matrix-sub: (lhs, rhs) => _matrix_sub(engine, lhs, rhs),
    matrix-mul: (lhs, rhs) => _matrix_mul(engine, lhs, rhs),
    matrix-div-scalar: (lhs, rhs) => _matrix_div_scalar(engine, lhs, rhs),
    transpose: matrix => _transpose(engine, matrix),
    det: matrix => _det(engine, matrix),
    inv: matrix => _inv(engine, matrix),
    matrix-solve: (A, b) => _matrix_solve(engine, A, b),
    matrix-solve-any: (A, b) => _matrix_solve_any(engine, A, b),
    row-reduce: (matrix, max-col: none) => _row_reduce(engine, matrix, max-col: max-col),
    augment: (lhs, rhs) => _augment(engine, lhs, rhs),
    split-col: (matrix, index) => _split_col(engine, matrix, index),
    primitive-part: matrix => _primitive_part(engine, matrix),
    content: matrix => _content(engine, matrix),
    matrix-at: (matrix, row, col) => _matrix_at(engine, matrix, row, col),
    matrix-shape: matrix => _matrix_shape(engine, matrix),
    add: (..terms) => _add(engine, ..terms),
    mul: (..factors) => _mul(engine, ..factors),
    neg: expr => _neg(engine, expr),
    sub: (lhs, rhs) => _sub(engine, lhs, rhs),
    div: (lhs, rhs) => _div(engine, lhs, rhs),
    pow: (base, exp) => _pow(engine, base, exp),
  )
}
#let _default_engine = init()

/// Parse Typst math content into an opaque Symbolica atom payload.
///
/// Arithmetic, fractions, powers, roots, absolute values, calls, and common
/// Typst math structures are translated through the configured grammar. Matrix-valued
/// `mat(...)` and `vec(...)` content must instead be passed to `matrix` or
/// `vec`. Keep the returned bytes opaque and use this module's functions to
/// inspect or transform them.
///
/// ```example
/// #let expr = math($x + 1$)
/// #to-typst(expr)
/// ```
///
/// -> bytes
#let math(
  /// Math content, normally written as `$...$`.
  /// -> content
  eqn,
  /// Parser grammar override. `none` uses the default engine grammar.
  /// -> dictionary | none
  grammar: none,
  /// Namespace for parsed symbols. `none` uses the engine namespace (`"typst"`
  /// for the top-level function).
  /// -> str | none
  namespace: none,
) = (_default_engine.math)(eqn, grammar: grammar, namespace: namespace)

/// Convert a supported Typst value or math expression into an atom payload.
///
/// Existing atom bytes pass through unchanged. Content is parsed like `math`;
/// integers and floats become numbers. Strings are parsed as leaf values:
/// numeric strings become numbers and other strings become symbols in the
/// engine's namespace. Matrix payloads are not atom payloads and should not be
/// passed to atom-only algebra functions.
///
/// ```example
/// #to-typst(atom("x"))
/// ```
///
/// -> bytes
#let atom(
  /// Value to convert.
  /// -> bytes | content | int | float | str
  value,
) = (_default_engine.atom)(value)

/// Construct a named Symbolica variable.
///
/// Unlike `wild`, this is an ordinary mathematical symbol and therefore does
/// not capture subexpressions during pattern matching.
///
/// ```example
/// #to-typst(var("x"))
/// ```
///
/// -> bytes
#let var(
  /// Symbol name without a namespace prefix or wildcard suffix.
  /// -> str
  name,
  /// Namespace override. `none` uses the engine namespace.
  /// -> str | none
  namespace: none,
) = (_default_engine.var)(name, namespace: namespace)

/// Construct a Symbolica pattern wildcard.
///
/// This creates the symbol `name` followed by `level` underscores. A wildcard
/// is a pattern placeholder used by `rule`, `replace`, and
/// `replace-wildcards`; it is not an ordinary unknown for algebra or solving.
/// Use `var` for a mathematical variable.
///
/// ```example
/// #raw(canonical(wild("a")))
/// ```
///
/// -> bytes
#let wild(
  /// Base name of the wildcard, without trailing underscores.
  /// -> str
  name,
  /// Number of underscore levels to append. `1` creates a conventional single
  /// wildcard such as `a_`; `0` appends no underscore and therefore creates an
  /// ordinary symbol instead.
  /// -> int
  level: 1,
  /// Namespace override. `none` uses the engine namespace.
  /// -> str | none
  namespace: none,
) = (_default_engine.wild)(name, level: level, namespace: namespace)

/// Render the parse tree for a Typst math expression.
///
/// This is a diagnostic view of the tree consumed by `math`; it does not create
/// a Symbolica atom.
///
/// ```example
/// #array-tree($(x + 1)^2$)
/// ```
///
/// -> content
#let array-tree(
  /// Math content to inspect.
  /// -> content
  eqn,
  /// Parser grammar override. `none` uses the engine grammar.
  /// -> dictionary | none
  grammar: none,
) = (_default_engine.array-tree)(eqn, grammar: grammar)

/// Render an atom or matrix payload as Symbolica source text.
///
/// ```example
/// #raw(canonical(math($x + 1$)))
/// ```
///
/// -> str
#let canonical(
  /// Atom or matrix payload. Other supported expression values are first
  /// converted as by `atom`.
  /// -> bytes | content | int | float | str
  expr,
  /// Include symbol namespaces in the output.
  /// -> bool
  namespaces: false,
) = (_default_engine.canonical)(expr, namespaces: namespaces)

/// Render an atom or matrix payload as Typst math source.
///
/// ```example
/// #raw(to-typst-source(math($x + 1$)))
/// ```
///
/// -> str
#let to-typst-source(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.to-typst-source)(expr)

/// Render an atom or matrix payload as evaluated Typst math content.
///
/// The payload is first printed as Typst math source and then evaluated in math
/// mode. Use `to-typst-source` when you need the source string instead.
///
/// ```example
/// #to-typst(math($x + 1$))
/// ```
///
/// -> content
#let to-typst(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
  /// Render as a block equation instead of inline math.
  /// -> bool
  block: false,
) = (_default_engine.to-typst)(expr, block: block)

/// Render an atom or matrix payload as LaTeX source.
///
/// ```example
/// #raw(to-latex(math($x^2 + 1$)))
/// ```
///
/// -> str
#let to-latex(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.to-latex)(expr)

/// Re-import and re-export an atom in Symbolica's canonical internal form.
///
/// This operation does not perform algebraic simplification; it currently acts
/// as a normalization and validation round-trip for atom payloads.
///
/// ```example
/// #to-typst(simplify(math($x + 0$)))
/// ```
///
/// -> bytes
#let simplify(
  /// Atom payload or supported expression value to normalize.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.simplify)(expr)

/// Expand an expression through Symbolica's polynomial expansion.
///
/// ```example
/// #to-typst(expand(math($(x + 1)^2$)))
/// ```
///
/// -> bytes
#let expand(
  /// Atom payload or supported expression value to expand.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.expand)(expr)

/// Factor an expression with Symbolica's exact factorization routine.
///
/// ```example
/// #to-typst(factor(math($x^2 + 2 x + 1$)))
/// ```
///
/// -> bytes
#let factor(
  /// Atom payload or supported expression value to factor.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.factor)(expr)

/// Differentiate an expression exactly with respect to an indeterminate.
///
/// ```example
/// #let x = var("x")
/// #to-typst(derivative(math($(x + 1)^2$), x))
/// ```
///
/// -> bytes
#let derivative(
  /// Expression to differentiate.
  /// -> bytes | content | int | float | str
  expr,
  /// Symbolica indeterminate, normally created with `var`.
  /// -> bytes | content | str
  var,
) = (_default_engine.derivative)(expr, var)

/// Integrate a polynomial exactly with respect to a variable.
///
/// The expression must be polynomial in `var`; rational functions with a
/// non-constant denominator and general transcendental integrands are rejected.
/// No arbitrary integration constant is added.
///
/// ```example
/// #let x = var("x")
/// #to-typst(integrate(math($3 x^2 - 4 x + 7$), x))
/// ```
///
/// -> bytes
#let integrate(
  /// Polynomial expression to integrate.
  /// -> bytes | content | int | float | str
  expr,
  /// Integration variable, normally created with `var`.
  /// -> bytes | content | str
  var,
) = (_default_engine.integrate)(expr, var)

/// Integrate a polynomial and expose one contribution per expanded term.
///
/// This has the same polynomial-only restriction as `integrate`. It returns
/// `(result: bytes, steps: array)`, where `result` is the complete antiderivative
/// and every atom payload in `steps` is the antiderivative of one top-level term
/// after Symbolica expands and canonically normalizes the input. A non-sum has
/// one step; source terms may combine or cancel during expansion. Thus `steps`
/// is a deterministic term-by-term decomposition, not a general pedagogical
/// derivation trace. No integration constant is added.
///
/// ```example
/// #let x = var("x")
/// #let out = integrate-with-steps(math($3 x^2 - 4 x + 7$), x)
/// #to-typst(out.result)
/// #out.steps.map(to-typst).join[, ]
/// ```
///
/// -> dictionary
#let integrate-with-steps(
  /// Polynomial expression to integrate.
  /// -> bytes | content | int | float | str
  expr,
  /// Integration variable, normally created with `var`.
  /// -> bytes | content | str
  var,
) = (_default_engine.integrate-with-steps)(expr, var)

/// Compute a univariate series expansion around `expansion-point`.
///
/// The truncation depth is the rational number `depth / depth-denom`. With an
/// absolute depth it is measured directly in `var`; with a relative depth it is
/// measured from the lowest order encountered in the expression.
///
/// ```example
/// #let sym = init(namespace: "symbolica")
/// #let m = sym.math
/// #let v = sym.var
/// #let ser = sym.series
/// #let render = sym.to-typst
/// #render(ser(m($cos(x)/(x + 1)$), v("x"), 0, 3))
/// ```
///
/// -> bytes
#let series(
  /// Expression to expand.
  /// -> bytes | content | int | float | str
  expr,
  /// Expansion variable, normally created with `var`.
  /// -> bytes | content | str
  var,
  /// Point about which to expand.
  /// -> bytes | content | int | float | str
  expansion-point,
  /// Numerator of the requested truncation depth.
  /// -> int
  depth,
  /// Denominator of the truncation depth.
  /// -> int
  depth-denom: 1,
  /// Use an absolute depth when `true`, or a depth relative to the lowest order
  /// in the expression when `false`.
  /// -> bool
  depth-is-absolute: true,
) = (
  _default_engine.series)(expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute)

/// Build a reusable replacement rule for `replace-multiple`.
///
/// `pattern` and `rhs` may contain symbols created with `wild`. The returned
/// dictionary contains opaque atom payloads plus the matching options; pass it
/// to `replace-multiple` rather than editing it manually.
///
/// ```example
/// #let r = rule(math($f("a_")$), math($g("a_")$))
/// #to-typst(replace-multiple(math($f(x)$), (r,)))
/// ```
///
/// -> dictionary
#let rule(
  /// Pattern to match.
  /// -> bytes | content | int | float | str
  pattern,
  /// Expression substituted for each match. Wildcards captured by `pattern`
  /// are substituted in this expression.
  /// -> bytes | content | int | float | str
  rhs,
  /// Wildcards that should prefer the smallest possible match.
  /// -> array
  non-greedy-wildcards: (),
  /// Lowest expression level at which matching is allowed; the root is level
  /// zero.
  /// -> int
  min-level: 0,
  /// Highest allowed matching level, or `none` for no upper bound.
  /// -> int | none
  max-level: none,
  /// Optional `(minimum, maximum)` pair overriding `min-level` and `max-level`;
  /// the maximum may be `none`.
  /// -> array | none
  level-range: none,
  /// Count full expression-tree depth when `true`; otherwise levels increase
  /// when entering functions.
  /// -> bool
  level-is-tree-depth: false,
  /// Allow a pattern to match part of a sum, product, or other term instead of
  /// requiring the whole term.
  /// -> bool
  partial: true,
  /// Permit wildcards on `rhs` that do not occur in `pattern`. When `false`,
  /// such rules are rejected.
  /// -> bool
  allow-new-wildcards-on-rhs: false,
  /// Maximum number of substituted right-hand sides cached; use zero to
  /// disable this cache.
  /// -> int
  rhs-cache-size: 100,
) = (
  _default_engine.rule)(pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Replace subexpressions matching `pattern` with `rhs`.
///
/// By default all non-overlapping outermost matches are replaced once.
/// `repeat: true` reapplies the rule until the expression stops changing; rules
/// that cycle therefore do not terminate. Use `rule` plus `replace-multiple`
/// when several patterns must be considered together.
///
/// ```example
/// #to-typst(replace(math($f(x, y)$), math($f("a_", "b_")$), math($g("b_", "a_")$)))
/// ```
///
/// -> bytes
#let replace(
  /// Expression in which to replace matches.
  /// -> bytes | content | int | float | str
  expr,
  /// Pattern to match.
  /// -> bytes | content | int | float | str
  pattern,
  /// Replacement expression.
  /// -> bytes | content | int | float | str
  rhs,
  /// Reapply the rule until it makes no further change.
  /// -> bool
  repeat: false,
  /// Replace only the first match found during a pass.
  /// -> bool
  once: false,
  /// Visit deepest matches before outer matches.
  /// -> bool
  bottom-up: false,
  /// Replace nested matches from the deepest outward, acting on the result of
  /// each inner replacement.
  /// -> bool
  nested: false,
  /// Wildcards that should prefer the smallest possible match.
  /// -> array
  non-greedy-wildcards: (),
  /// Lowest allowed matching level; the root is level zero.
  /// -> int
  min-level: 0,
  /// Highest allowed matching level, or `none` for no upper bound.
  /// -> int | none
  max-level: none,
  /// Optional `(minimum, maximum)` pair overriding `min-level` and `max-level`.
  /// -> array | none
  level-range: none,
  /// Count full tree depth instead of function-entry depth.
  /// -> bool
  level-is-tree-depth: false,
  /// Allow matching a part of a term rather than the entire term.
  /// -> bool
  partial: true,
  /// Permit `rhs` wildcards that are absent from `pattern`.
  /// -> bool
  allow-new-wildcards-on-rhs: false,
  /// Maximum number of substituted right-hand sides cached; zero disables the
  /// cache.
  /// -> int
  rhs-cache-size: 100,
) = (
  _default_engine.replace)(expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Apply several reusable replacement rules together.
///
/// The traversal options apply to the combined rule set. With `repeat: true`,
/// the complete set is reapplied until no rule changes the expression; cyclic
/// rule sets do not terminate.
///
/// ```example
/// #let r1 = rule(math($f("a_")$), math($h("a_")$))
/// #let r2 = rule(var("x"), var("z"))
/// #to-typst(replace-multiple(math($f(x) + x$), (r1, r2)))
/// ```
///
/// -> bytes
#let replace-multiple(
  /// Expression in which to replace matches.
  /// -> bytes | content | int | float | str
  expr,
  /// Array of dictionaries returned by `rule`.
  /// -> array
  rules,
  /// Reapply the rule set until it makes no further change.
  /// -> bool
  repeat: false,
  /// Replace only the first match found during a pass.
  /// -> bool
  once: false,
  /// Visit deepest matches before outer matches.
  /// -> bool
  bottom-up: false,
  /// Replace nested matches from deepest to outermost, acting on intermediate
  /// results.
  /// -> bool
  nested: false,
) = (
  _default_engine.replace-multiple)(expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested)

/// Substitute explicit values for wildcard placeholders in a pattern.
///
/// This does not search another expression. It transforms `pattern` itself and
/// requires each key in `replacements` to be a wildcard symbol.
///
/// ```example
/// #to-typst(replace-wildcards(math($k("a_")$), ((wild("a"), math($x + 1$)),)))
/// ```
///
/// -> bytes
#let replace-wildcards(
  /// Pattern containing wildcards to substitute.
  /// -> bytes | content | int | float | str
  pattern,
  /// Array of `(wildcard, replacement)` pairs.
  /// -> array
  replacements,
) = (_default_engine.replace-wildcards)(pattern, replacements)

/// Evaluate one expression numerically with optional substitutions.
///
/// `values` maps atom keys to real numbers or complex dictionaries of the form
/// `(re: number, im: number)`. Evaluation must eliminate every unsupported
/// symbolic quantity. The result always has the exact shape
/// `(re: float, im: float)`, even when it is real.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #repr(evaluate(math($x^2 + y$), values: ((x, 2.0), (y, 3.0))))
/// ```
///
/// -> dictionary
#let evaluate(
  /// Expression to evaluate.
  /// -> bytes | content | int | float | str
  expr,
  /// Array of `(expression, value)` substitution pairs. Values may be real
  /// numbers or `(re: ..., im: ...)` dictionaries.
  /// -> array
  values: (),
) = (_default_engine.evaluate)(expr, values: values)

/// Describe one real sampling axis for `evaluate-grid`.
///
/// Both endpoints are included when `samples` is greater than one. A
/// single-sample domain evaluates only at `min`. The returned dictionary has
/// the exact shape `(min: float, max: float, samples: int)`.
///
/// ```example
/// #repr(domain(-1, 1, samples: 3))
/// ```
///
/// -> dictionary
#let domain(
  /// Finite lower endpoint; it must be less than `max`.
  /// -> int | float
  min,
  /// Finite upper endpoint.
  /// -> int | float
  max,
  /// Number of evenly spaced points. Must be positive.
  /// -> int
  samples: 200,
) = _domain(min, max, samples: samples)

/// Evaluate one or more expressions at explicit points in one batch.
///
/// A single expression or variable may be passed directly; otherwise use an
/// array. Every point must supply one real or complex value per variable. The
/// result has one row per point in input order and one `(re: float, im: float)`
/// dictionary per expression in expression order.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #let rows = evaluate-many((math($x + y$), math($x y$)), (x, y), ((1, 2), (3, 4)))
/// #repr(rows)
/// ```
///
/// -> array
#let evaluate-many(
  /// One expression or a non-empty array of expressions to evaluate.
  /// -> bytes | content | array | int | float | str
  expressions,
  /// One variable or an array defining input-column order.
  /// -> bytes | content | array | str
  variables,
  /// Array of input rows. Each row is an array ordered like `variables`; for
  /// one variable, a scalar point is also accepted.
  /// -> array
  points,
) = (
  _default_engine.evaluate-many)(expressions, variables, points)

/// Evaluate expressions over a Cartesian product of real domains in one batch.
///
/// The result is `(shape: array, points: array, values: array)`. `shape` lists
/// the sample count of every domain. The grid axes are flattened into rows with
/// the last domain varying fastest: each `points` row contains real coordinates
/// in variable order, while the corresponding `values` row contains one
/// `(re: float, im: float)` dictionary per expression.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #let grid = evaluate-grid(math($x^2 + y$), (x, y), (domain(-1, 1, samples: 3), domain(0, 1, samples: 2)))
/// shape: #repr(grid.shape); values: #repr(grid.values)
/// ```
///
/// -> dictionary
#let evaluate-grid(
  /// One expression or a non-empty array of expressions.
  /// -> bytes | content | array | int | float | str
  expressions,
  /// One variable or an array defining grid-axis order.
  /// -> bytes | content | array | str
  variables,
  /// One `domain` dictionary per variable, in the same order.
  /// -> array
  domains,
) = (
  _default_engine.evaluate-grid)(expressions, variables, domains)

/// Solve a linear system exactly for `variables`.
///
/// Each item in `system` is interpreted as an expression equal to zero. A
/// vector matrix may be supplied instead of an array. The returned array is
/// contains one atom payload per item in `variables`, in the same order. For an
/// underdetermined system, bound variables may be expressed using free
/// variables; Symbolica chooses the highest-indexed variables as free.
/// Inconsistent or nonlinear systems produce an error.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #let sol = solve-linear((math($2 x + y - 5$), math($x - y - 1$)), (x, y))
/// #sol.map(to-typst).join[, ]
/// ```
///
/// -> array
#let solve-linear(
  /// Linear expressions understood to equal zero, as an array or vector matrix.
  /// -> array | bytes | content | int | float | str
  system,
  /// Variables to solve for, in result order.
  /// -> array | content | str
  variables,
) = (_default_engine.solve-linear)(system, variables)

/// Solve a linear or supported polynomial nonlinear system exactly.
///
/// Each expression in `system` is understood to equal zero. Polynomial systems
/// are solved exactly through Symbolica's Gröbner-basis and algebraic-root
/// machinery; coefficients may contain symbolic parameters when Symbolica can
/// treat them rationally. The result is an array of solution rows. Every row
/// contains one atom payload per requested variable, in `variables` order; an
/// empty array means there are no solutions.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #let solutions = solve-system((math($x^2 - 1$), math($y - x$)), (x, y))
/// #repr(solutions.map(row => row.map(canonical)))
/// ```
///
/// -> array
#let solve-system(
  /// Expressions understood to equal zero.
  /// -> array | content | int | float | str
  system,
  /// Variables to eliminate and return, in result-column order.
  /// -> array | content | str
  variables,
) = (_default_engine.solve-system)(system, variables)

/// Find a real root of a univariate expression with Newton's method.
///
/// The expression is interpreted as equal to zero and evaluated with `f64`
/// arithmetic. Convergence is local and depends on `init`; failure to converge
/// within `max-iterations` produces an error.
///
/// ```example
/// #let x = var("x")
/// #repr(nsolve(math($x^2 - 2$), x, 1.0))
/// ```
///
/// -> float
#let nsolve(
  /// Expression understood to equal zero.
  /// -> bytes | content | int | float | str
  expr,
  /// Real solve variable, normally created with `var`.
  /// -> bytes | content | str
  var,
  /// Initial real guess.
  /// -> int | float
  init,
  /// Numerical tolerance for the Newton iteration.
  /// -> int | float
  prec: 1e-4,
  /// Maximum number of Newton iterations.
  /// -> int
  max-iterations: 1000,
) = (
  _default_engine.nsolve)(expr, var, init, prec: prec, max-iterations: max-iterations)

/// Find a common real root of a system with multivariate Newton iteration.
///
/// Every expression is interpreted as equal to zero. `variables` and `init`
/// must have matching lengths, and the returned floats follow that same order.
/// Convergence is local and is not guaranteed for an arbitrary initial guess.
///
/// ```example
/// #let x = var("x")
/// #let y = var("y")
/// #repr(nsolve-system((math($x^2 + y - 3$), math($x - y$)), (x, y), (1.0, 1.0)))
/// ```
///
/// -> array
#let nsolve-system(
  /// Expressions understood to equal zero.
  /// -> array | content | int | float | str
  system,
  /// Variables to solve for, in result order.
  /// -> array | content | str
  variables,
  /// Initial real value for every variable.
  /// -> array
  init,
  /// Numerical tolerance for the Newton iteration.
  /// -> int | float
  prec: 1e-4,
  /// Maximum number of Newton iterations.
  /// -> int
  max-iterations: 1000,
) = (
  _default_engine.nsolve-system)(system, variables, init, prec: prec, max-iterations: max-iterations)

/// Convert Typst values into an opaque Symbolica matrix payload.
///
/// Accepted forms are Typst math `mat(...)` or `vec(...)`, a non-empty
/// rectangular nested array, a flat array interpreted as a column vector, a
/// scalar interpreted as a one-by-one matrix, or an existing payload. Every
/// entry must be convertible to a rational polynomial; general transcendental
/// expressions are not valid matrix entries.
///
/// ```example
/// #to-typst(matrix($mat(1, 2; 3, 4)$))
/// ```
///
/// -> bytes
#let matrix(
  /// Matrix source value.
  /// -> bytes | content | array | int | float | str
  value,
) = (_default_engine.matrix)(value)

/// Convert a non-empty sequence into a column-vector matrix payload.
///
/// Array entries must be rational-polynomial compatible. Typst math `vec(...)`
/// is accepted directly; existing matrix bytes pass through unchanged.
///
/// ```example
/// #to-typst(vec((1, 2)))
/// ```
///
/// -> bytes
#let vec(
  /// Vector entries, Typst `vec(...)` content, or an existing matrix payload.
  /// -> array | content | bytes
  values,
) = (_default_engine.vec)(values)

/// Create an `n` by `n` identity matrix payload.
///
/// ```example
/// #to-typst(identity(2))
/// ```
///
/// -> bytes
#let identity(
  /// Positive matrix dimension.
  /// -> int
  n,
) = (_default_engine.identity)(n)

/// Create a square diagonal matrix from a non-empty entry array.
///
/// Every entry must be rational-polynomial compatible.
///
/// ```example
/// #to-typst(eye((1, 2)))
/// ```
///
/// -> bytes
#let eye(
  /// Diagonal entries in top-left to bottom-right order.
  /// -> array
  diag,
) = (_default_engine.eye)(diag)

/// Add two matrices entry by entry.
///
/// The matrices must have equal shapes. Matrix source values accepted by
/// `matrix` may be supplied directly.
///
/// ```example
/// #to-typst(matrix-add(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-add(
  /// Left matrix.
  /// -> bytes | content | array | int | float | str
  lhs,
  /// Right matrix of the same shape.
  /// -> bytes | content | array | int | float | str
  rhs,
) = (_default_engine.matrix-add)(lhs, rhs)

/// Subtract two matrices entry by entry.
///
/// The matrices must have equal shapes. Matrix source values accepted by
/// `matrix` may be supplied directly.
///
/// ```example
/// #to-typst(matrix-sub(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-sub(
  /// Left matrix.
  /// -> bytes | content | array | int | float | str
  lhs,
  /// Right matrix of the same shape.
  /// -> bytes | content | array | int | float | str
  rhs,
) = (_default_engine.matrix-sub)(lhs, rhs)

/// Multiply two matrices, or multiply a matrix by a scalar expression.
///
/// Matrix multiplication requires compatible inner dimensions. A scalar must
/// be rational-polynomial compatible. Because opaque bytes are interpreted as
/// matrix payloads in the right-hand position, pass scalar atoms as ordinary
/// values or math content rather than as preconstructed atom bytes.
///
/// ```example
/// #to-typst(matrix-mul(matrix(((1, 2), (3, 4))), identity(2)))
/// ```
///
/// -> bytes
#let matrix-mul(
  /// Left matrix.
  /// -> bytes | content | array | int | float | str
  lhs,
  /// Right matrix, or scalar Typst value/math content.
  /// -> bytes | content | array | int | float | str
  rhs,
) = (_default_engine.matrix-mul)(lhs, rhs)

/// Divide every matrix entry by a nonzero scalar expression.
///
/// The scalar must be rational-polynomial compatible.
///
/// ```example
/// #to-typst(matrix-div-scalar(matrix(((2, 4), (6, 8))), 2))
/// ```
///
/// -> bytes
#let matrix-div-scalar(
  /// Matrix dividend.
  /// -> bytes | content | array | int | float | str
  lhs,
  /// Nonzero scalar divisor.
  /// -> bytes | content | int | float | str
  rhs,
) = (_default_engine.matrix-div-scalar)(lhs, rhs)

/// Transpose a matrix, exchanging rows and columns.
///
/// ```example
/// #to-typst(transpose(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let transpose(
  /// Matrix to transpose.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.transpose)(matrix)

/// Compute the exact determinant of a square matrix.
///
/// The result is an atom payload rather than a matrix payload.
///
/// ```example
/// #to-typst(det(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let det(
  /// Square matrix.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.det)(matrix)

/// Compute the exact inverse of an invertible square matrix.
///
/// Singular and non-square matrices produce an error.
///
/// ```example
/// #to-typst(inv(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let inv(
  /// Invertible square matrix.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.inv)(matrix)

/// Solve the matrix equation `A x = b` exactly.
///
/// The row counts of `A` and `b` must agree. This strict solver reports an
/// error when the system does not have the required unique solution; use
/// `matrix-solve-any` for underdetermined systems.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vec((5, 1))
/// #to-typst(matrix-solve(A, b))
/// ```
///
/// -> bytes
#let matrix-solve(
  /// Coefficient matrix.
  /// -> bytes | content | array | int | float | str
  A,
  /// Right-hand-side matrix or column vector.
  /// -> bytes | content | array | int | float | str
  b,
) = (_default_engine.matrix-solve)(A, b)

/// Solve `A x = b` exactly, choosing one solution if underdetermined.
///
/// The row counts of `A` and `b` must agree. An inconsistent system still
/// produces an error.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vec((5, 1))
/// #to-typst(matrix-solve-any(A, b))
/// ```
///
/// -> bytes
#let matrix-solve-any(
  /// Coefficient matrix.
  /// -> bytes | content | array | int | float | str
  A,
  /// Right-hand-side matrix or column vector.
  /// -> bytes | content | array | int | float | str
  b,
) = (_default_engine.matrix-solve-any)(A, b)

/// Row-reduce a matrix exactly using Gaussian elimination.
///
/// Returns `(matrix: bytes, rank: int)`. By default every column may contain a
/// pivot. Set `max-col` to limit pivot search to the first `max-col` columns,
/// which is useful for an augmented matrix whose trailing columns are right-hand
/// sides; all columns are still transformed by the row operations.
///
/// ```example
/// #let rr = row-reduce(matrix(((1, 2), (3, 4))))
/// rank #rr.rank: #to-typst(rr.matrix)
/// ```
///
/// -> dictionary
#let row-reduce(
  /// Matrix to reduce.
  /// -> bytes | content | array | int | float | str
  matrix,
  /// Number of leading columns eligible for pivots. `none` uses every column.
  /// An explicit value must be between zero and the column count.
  /// -> int | none
  max-col: none,
) = (_default_engine.row-reduce)(matrix, max-col: max-col)

/// Horizontally concatenate two matrices as `[lhs rhs]`.
///
/// Both matrices must have the same number of rows.
///
/// ```example
/// #to-typst(augment(identity(2), matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let augment(
  /// Left block.
  /// -> bytes | content | array | int | float | str
  lhs,
  /// Right block with the same row count.
  /// -> bytes | content | array | int | float | str
  rhs,
) = (_default_engine.augment)(lhs, rhs)

/// Split a matrix into left and right column blocks.
///
/// Returns exactly `(left, right)`, both as matrix payloads. The current matrix
/// backend requires `index > 0` and `index < columns - 1`.
///
/// ```example
/// #let aug = augment(identity(2), matrix(((1, 2), (3, 4))))
/// #let parts = split-col(aug, 2)
/// #to-typst(parts.at(0)) | #to-typst(parts.at(1))
/// ```
///
/// -> array
#let split-col(
  /// Matrix to split.
  /// -> bytes | content | array | int | float | str
  matrix,
  /// Zero-based first column of the right block.
  /// -> int
  index,
) = (_default_engine.split-col)(matrix, index)

/// Divide a rational-polynomial matrix by its content.
///
/// The result is the primitive matrix whose coefficient GCD has been removed.
///
/// ```example
/// #let x = var("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(primitive-part(P))
/// ```
///
/// -> bytes
#let primitive-part(
  /// Matrix whose entries are rational polynomials.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.primitive-part)(matrix)

/// Compute the coefficient content of a rational-polynomial matrix.
///
/// The content is the common coefficient GCD and is returned as an atom
/// payload.
///
/// ```example
/// #let x = var("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(content(P))
/// ```
///
/// -> bytes
#let content(
  /// Matrix whose entries are rational polynomials.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.content)(matrix)

/// Read one matrix entry as an atom payload using zero-based indices.
///
/// ```example
/// #let A = matrix(((1, 2), (3, 4)))
/// #to-typst(matrix-at(A, 0, 1))
/// ```
///
/// -> bytes
#let matrix-at(
  /// Matrix to index.
  /// -> bytes | content | array | int | float | str
  matrix,
  /// Zero-based row index.
  /// Must be within the matrix bounds.
  /// -> int
  row,
  /// Zero-based column index.
  /// Must be within the matrix bounds.
  /// -> int
  col,
) = (_default_engine.matrix-at)(matrix, row, col)

/// Return the matrix shape as the two-element array `(rows, columns)`.
///
/// ```example
/// #repr(matrix-shape(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> array
#let matrix-shape(
  /// Matrix to inspect.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine.matrix-shape)(matrix)

/// Construct an exact sum from expression values.
///
/// Arguments are converted as by `atom`. With no arguments, the result is the
/// additive identity zero.
///
/// ```example
/// #to-typst(add("x", 1, "y"))
/// ```
///
/// -> bytes
#let add(
  /// Positional terms to add.
  /// -> arguments
  ..terms,
) = (_default_engine.add)(..terms)

/// Construct an exact product from expression values.
///
/// Arguments are converted as by `atom`. With no arguments, the result is the
/// multiplicative identity one.
///
/// ```example
/// #to-typst(mul(2, "x", "y"))
/// ```
///
/// -> bytes
#let mul(
  /// Positional factors to multiply.
  /// -> arguments
  ..factors,
) = (_default_engine.mul)(..factors)

/// Negate an expression exactly.
///
/// ```example
/// #to-typst(neg("x"))
/// ```
///
/// -> bytes
#let neg(
  /// Expression to negate.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine.neg)(expr)

/// Subtract `rhs` from `lhs` exactly.
///
/// ```example
/// #to-typst(sub("x", "y"))
/// ```
///
/// -> bytes
#let sub(
  /// Minuend.
  /// -> bytes | content | int | float | str
  lhs,
  /// Subtrahend.
  /// -> bytes | content | int | float | str
  rhs,
) = (_default_engine.sub)(lhs, rhs)

/// Construct the exact quotient `lhs / rhs`.
///
/// ```example
/// #to-typst(div(1, "x"))
/// ```
///
/// -> bytes
#let div(
  /// Numerator.
  /// -> bytes | content | int | float | str
  lhs,
  /// Denominator.
  /// -> bytes | content | int | float | str
  rhs,
) = (_default_engine.div)(lhs, rhs)

/// Construct the exact power `base ^ exp`.
///
/// ```example
/// #to-typst(pow("x", 3))
/// ```
///
/// -> bytes
#let pow(
  /// Base expression.
  /// -> bytes | content | int | float | str
  base,
  /// Exponent expression.
  /// -> bytes | content | int | float | str
  exp,
) = (_default_engine.pow)(base, exp)
