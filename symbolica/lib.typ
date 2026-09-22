#import "@preview/parsely:0.1.1"
#import "render.typ" as atom-render

#let _is-call-head(head) = {
  if type(head) != content or head.func() != math.attach { return true }
  let fields = head.fields()
  let top = fields.at("t", default: none)
  if top == [] { return true }
  if type(top) == content {
    let text = top.fields().at("text", default: none)
    if type(text) == str and text.trim() == "" { return true }
  }
  ("b", "tl", "tr", "bl", "br").any(slot => fields.at(slot, default: none) != none)
}

#let _default_grammar = (
  arg: (infix: $,$, assoc: true, prec: 4),
  add: (infix: $+$, prec: 1, assoc: true),
  sub: (infix: $-$, prec: 1),
  plus: (prefix: $+$, prec: 2),
  neg: (prefix: $-$, prec: 2),
  times: (infix: $times$, prec: 2),
  dot: (infix: $dot$, prec: 2),
  factorial: (postfix: $#parsely.tight !$, prec: 3),
  semantic-metadata: (postfix: metadata, prec: 5),
  mul: (infix: $$, prec: 2.5, assoc: true),
  "()": (match: $(#parsely.slot("expr*"))$),
  union: (infix: $union$, prec: 1),
  inter: (infix: $inter$, prec: 1),
  // Calls must match before their decorated heads are consumed as attachments.
  // A bare superscript remains a power followed by multiplication.
  attached-call: (
    match: $#parsely.slot("fn") #parsely.tight #parsely.slot("tail", guard: value => {
      (type(value) == content and value.func() == math.attach and
        parsely.match($(#parsely.slot("body*"))$, value.base) != false)
    })$,
    guard: slots => _is-call-head(slots.fn),
    rewrite: node => {
      // Typst stores h'(c)^2 as h' followed by attach((c), t:2).
      // Rebuild the complete call beneath the unchanged attachment slots.
      let fields = node.slots.tail.fields()
      fields.base = node.slots.fn + fields.base
      (head: "attach", args: (), slots: fields)
    },
  ),
  op-call: (match: $op(#parsely.slot("op"))(#parsely.slot("args*"))$),
  call: (
    match: $#parsely.slot("fn") #parsely.tight (#parsely.slot("body*"))$,
    guard: slots => _is-call-head(slots.fn),
  ),
  // A power-shaped pattern matches a subset of attachment fields and would
  // discard the subscript in a_i^2. Preserve the complete native element.
  attach: (match: math.attach),
  primes: (match: math.primes),
  accent: (match: math.accent),
  cancel: (match: math.cancel),
  cases: (match: math.cases),
  class: (match: math.class),
  frac: (match: math.frac),
  limits: (match: math.limits),
  lr: (match: math.lr),
  mat: (match: math.mat),
  mid: (match: math.mid),
  overbrace: (match: math.overbrace),
  overbracket: (match: math.overbracket),
  overline: (match: math.overline),
  overparen: (match: math.overparen),
  overshell: (match: math.overshell),
  vec: (match: math.vec),
  root: (match: math.root),
  scripts: (match: math.scripts),
  stretch: (match: math.stretch),
  underbrace: (match: math.underbrace),
  underbracket: (match: math.underbracket),
  underline: (match: math.underline),
  underparen: (match: math.underparen),
  undershell: (match: math.undershell),
  op: (match: math.op),
)
#let _typst_math = math

#let _leaf(value, parse-content: none, display: false, strict: false) = {
  if type(value) == dictionary {
    value
  } else if type(value) == array {
    value.map(item => {
      if type(item) == content and parse-content != none {
        parse-content(item)
      } else {
        _leaf(item, parse-content: parse-content, display: display, strict: strict)
      }
    })
  } else if type(value) in (str, int, float, bool, bytes) or value == none {
    value
  } else if type(value) == content {
    let fields = value.fields()
    if repr(value.func()) == "styled" and "child" in fields {
      if parse-content != none { parse-content(fields.child) }
      else { _leaf(fields.child, display: display, strict: strict) }
    } else if "text" in fields {
      // Typst exposes quoted labels as text and math identifiers as symbols.
      // Numeric text also represents ordinary numeric literals, so retain the
      // existing numeric interpretation for that indistinguishable case.
      if (display and repr(value.func()) == "text" and fields.text.trim() != "" and
        fields.text.match(regex("^[0-9]+([.][0-9]+)?([eE][+-]?[0-9]+)?$")) == none) {
        (head: "text", args: (fields.text,), slots: (:))
      } else { fields.text }
    } else if "children" in fields {
      fields.children.map(_leaf).join("")
    } else if strict {
      panic("literal: unsupported standalone element " + repr(value.func()) +
        "; Atom metadata must follow its visible label")
    } else {
      repr(value)
    }
  } else if strict {
    panic("literal: unsupported value of type " + repr(type(value)))
  } else {
    repr(value)
  }
}

#let _with-semantic-metadata(grammar) = {
  let enhanced = (
    semantic-metadata: (postfix: metadata, prec: 5),
  )
  for (name, rule) in grammar {
    if name != "semantic-metadata" { enhanced.insert(name, rule) }
  }
  enhanced
}

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
  if kind == "vec" or kind == "cases" {
    return fn(..fields.remove("children"), ..fields)
  }
  if kind == "mat" {
    return fn(..fields.remove("rows"), ..fields)
  }
  if kind == "root" and "index" not in fields {
    return _typst_math.sqrt(fields.remove("radicand"), ..fields)
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
  // Inline equations inserted by `to-typst` are transparent inside math.
  // Parsely unwraps its outer equation, but not equations nested in a sequence.
  if kind == "equation" { return _trim_math(value.body) }
  // Explicit math spacing (including rendered product separators) is lexical
  // whitespace, not a symbolic operand.
  if kind == "h" { return [ ] }
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

#let _empty-literal-label(value) = {
  if value == none { return true }
  if type(value) == array { return value.all(_empty-literal-label) }
  if type(value) == content {
    let kind = repr(value.func())
    if kind == "equation" { return _empty-literal-label(value.body) }
    if kind == "sequence" { return value.children.all(_empty-literal-label) }
    if kind == "space" { return true }
    if kind in ("text", "symbol") { return value.text.trim() == "" }
  }
  false
}

// Parsely recursively parses content slots before the leaf encoder runs.
// An explicit empty text leaf keeps an empty attachment present and avoids
// Parsely's whitespace-only slot failure. Keep algebra grammar rewrites out
// of literal labels, while retaining the display grammar's call handling.
#let _literal-grammar() = _default_grammar.pairs().map(((name, rule)) => {
  let original = rule.at("rewrite", default: node => node)
  let normalize(value) = if type(value) == content and _empty-literal-label(value) {
    text("")
  } else { value }
  rule.insert("rewrite", node => {
    let node = original(node)
    if type(node) != dictionary { return node }
    node.args = node.args.map(normalize)
    node.slots = node.slots.pairs().map(((key, value)) => (key, normalize(value))).to-dict()
    node
  })
  (name, rule)
}).to-dict()

#let _ast_bytes(eqn, grammar, display: false, complete: false) = {
  let encode-tree(value, display: false) = {
    let encode-node(node, display: false) = {
      if type(node) != dictionary or "head" not in node {
        return _leaf(node, display: display, strict: complete, parse-content: encode-tree.with(display: display))
      }
      (
        head: node.head,
        args: node.args.map(child => encode-node(child, display: display)),
        slots: node.slots.pairs().map(((slot, child)) => (
          slot,
          encode-node(child, display: display or (node.head == "attach" and slot != "t")),
        )).to-dict(),
      )
    }
    if display and _empty-literal-label(value) { return none }
    let parsed = parsely.parse(_trim_math(value), _with-semantic-metadata(grammar))
    if complete and parsed.rest != none and parsed.rest != [] {
      panic("literal: unparsed content in the display label; use a named literal with notation(...) for custom rendering")
    }
    encode-node(parsed.tree, display: display)
  }
  cbor.encode(encode-tree(eqn, display: display))
}

#let _from_math(engine, eqn, grammar: none, namespace: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  engine.plugin.from_ast(_ast_bytes(eqn, grammar), _namespace_bytes(engine, namespace: namespace))
}

#let _parse(engine, input, grammar: none, namespace: none) = {
  if type(input) == content {
    return _from_math(engine, input, grammar: grammar, namespace: namespace)
  }
  if grammar != none {
    panic("parse: grammar applies only to Typst math content; strings use Symbolica syntax and numbers or Atom payloads need no grammar")
  }
  if type(input) == str {
    engine.plugin.from_string(cbor.encode(input), _namespace_bytes(engine, namespace: namespace))
  } else if type(input) == bytes {
    input
  } else if type(input) in (int, float) {
    engine.plugin.from_ast(cbor.encode(input), _namespace_bytes(engine, namespace: namespace))
  } else {
    panic("parse: expected Typst math content, a Symbolica expression string, a number, or an Atom payload; found " + repr(type(input)))
  }
}

#let _array_tree(engine, eqn, grammar: none) = {
  let grammar = if grammar == none { engine.grammar } else { grammar }
  let parsed = parsely.parse(_trim_math(eqn), _with-semantic-metadata(grammar))
  parsely.walk(parsed.tree, post: it => (
    strong(raw(it.head)),
    ..it.args,
    ..it.slots.pairs().map(((slot, it)) => {
      (text(gray, 0.8em, raw(slot)), it)
    }),
  ), leaf: _typst_math.equation)
}

#let _symbol-atom(engine, name, namespace: none, tags: ()) = engine.plugin.symbol(
  cbor.encode(name),
  _namespace_bytes(engine, namespace: namespace),
  cbor.encode(tags),
)
#let _expr_bytes(engine, expr, namespace: none) = {
  if type(expr) == bytes {
    expr
  } else if type(expr) == content {
    _from_math(engine, expr, namespace: namespace)
  } else {
    engine.plugin.from_ast(cbor.encode(expr), _namespace_bytes(engine, namespace: namespace))
  }
}
#let _atom-envelope(atom, semantic) = (
  protocol: "symbolica",
  version: 1,
  kind: "atom",
  atom: atom,
  semantic: semantic,
)
#let _annotated-visual(atom, visual, semantic) = {
  _typst_math.attach(visual) + metadata(_atom-envelope(atom, semantic))
}
#let _annotated-atom(engine, atom, semantic) = {
  // Constructors use the same generic, document-side renderer as `to-typst`.
  // The outer annotation intentionally represents the complete constructor
  // result, so nested generic annotations are disabled here.
  let rendered = cbor(engine.plugin.render_tree(atom))
  let visual = if rendered.kind == "matrix-render-source" {
    eval(rendered.source, mode: "math", scope: (:)).body
  } else {
    atom-render.render-tree(rendered, notation: engine.notation)
  }
  _annotated-visual(atom, visual, semantic)
}
#let _validate-tags(tags) = {
  if type(tags) != array or not tags.all(tag => type(tag) == str) {
    panic("tags must be an array of strings")
  }
  for (index, tag) in tags.enumerate() {
    let parts = tag.split("::")
    if parts.len() < 2 or parts.any(part => part == "") {
      panic("tags must use canonical namespaced names such as model::positive")
    }
    if tag in tags.slice(0, index) {
      panic("tags must not contain duplicates")
    }
  }
}
// Portable literal labels accept only fields represented by MathDisplay.
// Validate the original content before Parsely can unwrap styles or discard
// fields. Equation block layout and ordinary whitespace are presentation only.
#let _literal-fields = (
  equation: ("body", "block"), sequence: ("children",),
  symbol: ("text",), text: ("text",), space: (),
  attach: ("base", "t", "b", "tl", "tr", "bl", "br"),
  primes: ("count",), frac: ("num", "denom"), root: ("index", "radicand"),
  lr: ("body",), vec: ("children",), mat: ("rows",), cases: ("children",),
  accent: ("base", "accent"), op: ("text",), class: ("class", "body"),
  cancel: ("body",), limits: ("body",), scripts: ("body",),
  stretch: ("body",), mid: ("body",), overline: ("body",), underline: ("body",),
  overbrace: ("body", "annotation"), underbrace: ("body", "annotation"),
  overbracket: ("body", "annotation"), underbracket: ("body", "annotation"),
  overparen: ("body", "annotation"), underparen: ("body", "annotation"),
  overshell: ("body", "annotation"), undershell: ("body", "annotation"),
)
#let _validate-literal-content(value, location: "literal label") = {
  let unsupported(detail) = panic("literal: " + detail + " in " + location +
    "; use a named literal with notation(...) for custom rendering")
  if type(value) == array {
    for (index, item) in value.enumerate() {
      _validate-literal-content(item, location: location + " item " + str(index + 1))
    }
  } else if type(value) == content {
    let kind = repr(value.func())
    let fields = value.fields()
    if kind == "metadata" {
      let envelope = fields.at("value", default: none)
      if (type(envelope) != dictionary or envelope.at("protocol", default: none) != "symbolica" or
        envelope.at("kind", default: none) != "atom") {
        unsupported("unsupported metadata")
      }
      return
    }
    if kind not in _literal-fields { unsupported("unsupported element " + kind) }
    let allowed = _literal-fields.at(kind)
    for (key, field) in fields {
      if key not in allowed { unsupported("unsupported field " + kind + "." + key) }
      let child-location = key + " of " + kind
      if kind == "attach" and key != "base" {
        let slot = (t: "top", b: "bottom", tl: "top-left", tr: "top-right", bl: "bottom-left", br: "bottom-right").at(key)
        let base = fields.base
        let label = base.fields().at("text", default: "attach")
        child-location = slot + " attachment of " + label
      }
      _validate-literal-content(field, location: child-location + " in " + location)
    }
  } else if value != none and type(value) not in (str, int, float, bool) {
    unsupported("unsupported value of type " + repr(type(value)))
  }
}
#let _ordinary-name(name, constructor) = {
  if type(name) != str or name == "" {
    panic(constructor + ": name must be a nonempty string")
  }
  if name != name.trim() {
    panic(constructor + ": name must not contain surrounding whitespace")
  }
  if name.ends-with("_") {
    panic(constructor + ": names must not end with an underscore; use wild(...) for pattern placeholders")
  }
}
#let _symbol(engine, input, name: none, namespace: none, tags: ()) = {
  _validate-tags(tags)
  let namespace = _namespace(engine, namespace)
  let atom = if type(input) == str {
    if name != none { panic("literal: name is only available for content labels") }
    _ordinary-name(input, "literal")
    _symbol-atom(engine, input, namespace: namespace, tags: tags)
  } else if type(input) == content {
    if name != none { _ordinary-name(name, "literal") }
    _validate-literal-content(input)
    if _empty-literal-label(input) { panic("literal: label must contain visible content") }
    engine.plugin.literal_from_ast(
      _ast_bytes(input, _literal-grammar(), display: true, complete: true),
      cbor.encode(namespace), cbor.encode(name), cbor.encode(tags),
    )
  } else {
    panic("literal: expected a symbol-name string or portable Typst math/text content")
  }
  _annotated-atom(engine, atom, (
    kind: "symbol",
    name: if type(input) == str { input } else { name },
    namespace: namespace,
    tags: tags,
  ))
}
#let _function-atom(engine, name, arguments, namespace: none, tags: ()) = {
  let namespace = _namespace(engine, namespace)
  let head = _symbol-atom(engine, name, namespace: namespace, tags: tags)
  let tree = (
    head: "call",
    args: (),
    slots: (
      fn: head,
      body: (
        head: "arg",
        args: arguments.map(argument => _expr_bytes(engine, argument)),
        slots: (:),
      ),
    ),
  )
  engine.plugin.from_ast(cbor.encode(tree), cbor.encode(namespace))
}
#let _function(engine, name, namespace: none, tags: ()) = {
  if type(name) != str { panic("function-head: name must be a string") }
  _validate-tags(tags)

  let namespace = _namespace(engine, namespace)
  (..arguments) => {
    if arguments.named().len() > 0 {
      panic("symbolic function calls accept only positional arguments")
    }
    let arguments = arguments.pos()
    let atom = _function-atom(engine, name, arguments, namespace: namespace, tags: tags)
    _annotated-atom(engine, atom, (
      kind: "function-call",
      head: (
        kind: "function",
        name: name,
        namespace: namespace,
        tags: tags,
      ),
      arguments: arguments.map(argument => _expr_bytes(engine, argument)),
    ))
  }
}
#let _wild(engine, name, level: 1, namespace: none) = {
  _ordinary-name(name, "wild")
  if type(level) != int or level < 1 {
    panic("wild: level must be a positive integer; use literal(...) for ordinary symbols")
  }
  let suffix = ""
  for _ in range(level) { suffix += "_" }
  _symbol-atom(engine, name + suffix, namespace: namespace)
}

#let _atom_array(engine, values, namespace: none) = cbor.encode(values.map(value => _expr_bytes(engine, value, namespace: namespace)))
#let _payload_bytes(engine, value, namespace: none) = if type(value) == bytes { value } else { _expr_bytes(engine, value, namespace: namespace) }
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
  (_expr_bytes(engine, value),)
}

#let _canonical(engine, expr, namespaces: false) = str(engine.plugin.canonical(_payload_bytes(engine, expr), cbor.encode(namespaces)))
#let _to_typst_source(engine, expr) = str(engine.plugin.to_typst(_payload_bytes(engine, expr)))
#let _render-semantic(node) = {
  let semantic = (
    kind: "render-node",
    node-kind: node.at("kind", default: none),
  )
  let descriptor = node.at("symbol", default: none)
  if type(descriptor) == dictionary {
    semantic.insert("symbol", descriptor)
  }
  semantic
}
#let _to_typst(engine, expr, notation: none, block: false) = {
  let payload = _payload_bytes(engine, expr)
  let rendered = cbor(engine.plugin.render_tree(payload))
  let display-notation = if notation == none {
    engine.notation
  } else {
    atom-render.merge-notation(engine.notation, notation)
  }
  let body = if rendered.kind == "matrix-render-source" {
    eval(rendered.source, mode: "math", scope: (:)).body
  } else {
    atom-render.render-tree(
      rendered,
      notation: display-notation,
      annotate: (atom, visual, node) => _annotated-visual(
        atom,
        visual,
        _render-semantic(node),
      ),
    )
  }
  _typst_math.equation(body, block: block)
}
#let _to_latex(engine, expr) = str(engine.plugin.to_latex(_payload_bytes(engine, expr)))

#let _simplify(engine, expr) = engine.plugin.simplify_expr(_expr_bytes(engine, expr))
#let _expand(engine, expr) = engine.plugin.expand(_expr_bytes(engine, expr))
#let _together(engine, expr) = engine.plugin.together(_expr_bytes(engine, expr))
#let _cancel(engine, expr) = engine.plugin.cancel(_expr_bytes(engine, expr))
#let _apart(engine, expr, var) = engine.plugin.apart(cbor.encode((
  expr: _expr_bytes(engine, expr),
  var: _expr_bytes(engine, var),
)))
#let _collect(engine, expr, variables) = engine.plugin.collect(cbor.encode((
  expr: _expr_bytes(engine, expr),
  variables: _expr_array(engine, variables),
)))
#let _coefficient(engine, expr, monomial) = engine.plugin.coefficient(
  _expr_bytes(engine, expr),
  _expr_bytes(engine, monomial),
)
#let _coefficient_list(engine, expr, variables) = cbor(engine.plugin.coefficient_list(cbor.encode((
  expr: _expr_bytes(engine, expr),
  variables: _expr_array(engine, variables),
))))
#let _terms(engine, expr) = cbor(engine.plugin.terms(_expr_bytes(engine, expr)))
#let _indeterminates(engine, expr, enter-functions: true) = cbor(engine.plugin.indeterminates(cbor.encode((
  expr: _expr_bytes(engine, expr),
  enter-functions: enter-functions,
))))
#let _contains(engine, expr, subexpression) = cbor(engine.plugin.contains(
  _expr_bytes(engine, expr),
  _expr_bytes(engine, subexpression),
))
#let _is_constant(engine, expr) = cbor(engine.plugin.is_constant(_expr_bytes(engine, expr)))
#let _to_float(engine, expr, decimal-prec: 16) = engine.plugin.to_float(cbor.encode((
  expr: _expr_bytes(engine, expr),
  decimal-prec: decimal-prec,
)))
#let _factor(engine, expr, complex: false, square-free: false) = {
  assert(not (complex and square-free), message: "complex and square-free factorization cannot be combined")
  engine.plugin.factor(cbor.encode((
    expr: _expr_bytes(engine, expr),
    complex: complex,
    square-free: square-free,
  )))
}
#let _derivative(engine, expr, var) = engine.plugin.derivative(_expr_bytes(engine, expr), _expr_bytes(engine, var))
#let _integrate(engine, expr, var, steps: false) = {
  let expression = _expr_bytes(engine, expr)
  let variable = _expr_bytes(engine, var)
  // Only integration prepares the rule tables. Typst caches the transition
  // and restores its memory snapshot for additional execution instances.
  let prepared = plugin.transition(engine.plugin.initialize)
  if steps {
    cbor(prepared.integrate_with_steps(expression, variable))
  } else {
    prepared.integrate(expression, variable)
  }
}
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

#let _solve(engine, system, variables, domain: "complex") = cbor(engine.plugin.solve(cbor.encode((
  system: _expr_array(engine, system),
  variables: _expr_array(engine, variables),
  domain: domain,
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
#let _matrix_is_zero(engine, matrix) = cbor(engine.plugin.matrix_is_zero(_matrix(engine, matrix)))
#let _matrix_is_diagonal(engine, matrix) = cbor(engine.plugin.matrix_is_diagonal(_matrix(engine, matrix)))
#let _matrix_derivative(engine, matrix, var) = engine.plugin.matrix_derivative(
  _matrix(engine, matrix),
  _expr_bytes(engine, var),
)

#let _add(engine, ..terms) = engine.plugin.add(_atom_array(engine, terms.pos()))
#let _mul(engine, ..factors) = engine.plugin.mul(_atom_array(engine, factors.pos()))
#let _neg(engine, expr) = engine.plugin.neg(_expr_bytes(engine, expr))
#let _sub(engine, lhs, rhs) = engine.plugin.sub(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _div(engine, lhs, rhs) = engine.plugin.div(_expr_bytes(engine, lhs), _expr_bytes(engine, rhs))
#let _pow(engine, base, exp) = engine.plugin.power(_expr_bytes(engine, base), _expr_bytes(engine, exp))

#let _bundled_plugin() = plugin("symbolica.wasm")

/// Create an independent set of Symbolica functions.
///
/// The returned dictionary exposes Symbolica's parsing, algebra, evaluation,
/// solving, integration, and matrix operations. Use `init` when you want to select a symbol
/// namespace, plugin location, or parser grammar; ordinary calculations can
/// use the imported top-level functions directly. Built-in special-function
/// constructors always use Symbolica's function namespace; their arguments
/// still use this engine's namespace and notation.
///
/// ```example
/// #let engine = init(namespace: "physics")
/// #let literal = engine.literal
/// #let render = engine.canonical
/// #raw(render(literal("x"), namespaces: true))
/// ```
///
/// -> dictionary
#let init(
  /// Default namespace for symbols parsed from Typst math or strings. A
  /// per-call `namespace` passed to `parse` or `literal` takes precedence.
  /// -> str
  namespace: "typst",
  /// WebAssembly plugin path or bytes passed to Typst's `plugin` constructor.
  /// `none` selects the bundled engine. A custom source must be an uncompressed
  /// Wasm module; relative paths are resolved by Typst from this source file.
  /// -> str | bytes | none
  source: none,
  /// Typst content grammar used by `parse` and `array-tree` unless they receive
  /// an explicit override. Expression strings always use Symbolica syntax.
  /// -> dictionary
  grammar: _default_grammar,
  /// Default document-side notation used by `to-typst`. Build one with
  /// `notation`; a per-call override still takes precedence.
  /// -> dictionary
  notation: atom-render.notation(),
) = {
  let plugin-module = if source != none {
    plugin(source)
  } else {
    _bundled_plugin()
  }
  let engine = (
    plugin: plugin-module,
    grammar: grammar,
    namespace: namespace,
    notation: notation,
  )

  let api = (
    parse: (input, grammar: none, namespace: none) => _parse(engine, input, grammar: grammar, namespace: namespace),
    literal: (input, name: none, namespace: none, tags: ()) => _symbol(engine, input, name: name, namespace: namespace, tags: tags),
    function-head: (name, namespace: none, tags: ()) => _function(engine, name, namespace: namespace, tags: tags),
    gamma-function: (z) => (_function(engine, "gamma", namespace: "symbolica"))(z),
    polygamma: (n, z) => (_function(engine, "polygamma", namespace: "symbolica"))(n, z),
    polylog: (s, z) => (_function(engine, "polylog", namespace: "symbolica"))(s, z),
    zeta-function: (s) => (_function(engine, "zeta", namespace: "symbolica"))(s),
    bessel-j: (nu, z) => (_function(engine, "bessel_j", namespace: "symbolica"))(nu, z),
    bessel-y: (nu, z) => (_function(engine, "bessel_y", namespace: "symbolica"))(nu, z),
    bessel-i: (nu, z) => (_function(engine, "bessel_i", namespace: "symbolica"))(nu, z),
    bessel-k: (nu, z) => (_function(engine, "bessel_k", namespace: "symbolica"))(nu, z),
    wild: (name, level: 1, namespace: none) => _wild(engine, name, level: level, namespace: namespace),
    array-tree: (eqn, grammar: none) => _array_tree(engine, eqn, grammar: grammar),
    canonical: (expr, namespaces: false) => _canonical(engine, expr, namespaces: namespaces),
    to-typst-source: expr => _to_typst_source(engine, expr),
    to-typst: (expr, notation: none, block: false) => _to_typst(
      engine, expr, notation: notation, block: block,
    ),
    to-latex: expr => _to_latex(engine, expr),
    simplify: expr => _simplify(engine, expr),
    expand: expr => _expand(engine, expr),
    together: expr => _together(engine, expr),
    cancel-factors: expr => _cancel(engine, expr),
    apart: (expr, var) => _apart(engine, expr, var),
    collect: (expr, variables) => _collect(engine, expr, variables),
    coefficient: (expr, monomial) => _coefficient(engine, expr, monomial),
    coefficient-list: (expr, variables) => _coefficient_list(engine, expr, variables),
    summands: expr => _terms(engine, expr),
    indeterminates: (expr, enter-functions: true) => _indeterminates(engine, expr, enter-functions: enter-functions),
    contains: (expr, subexpression) => _contains(engine, expr, subexpression),
    is-constant: expr => _is_constant(engine, expr),
    to-float: (expr, decimal-prec: 16) => _to_float(engine, expr, decimal-prec: decimal-prec),
    factor: (expr, complex: false, square-free: false) => _factor(engine, expr, complex: complex, square-free: square-free),
    derivative: (expr, var) => _derivative(engine, expr, var),
    integrate: (expr, var) => _integrate(engine, expr, var),
    integrate-with-steps: (expr, var) => _integrate(engine, expr, var, steps: true),
    series: (expr, var, expansion-point, depth, depth-denom: 1, depth-is-absolute: true) => _series(engine, expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute),
    rule: (pattern, rhs, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _rule(engine, pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace: (expr, pattern, rhs, repeat: false, once: false, bottom-up: false, nested: false, non-greedy-wildcards: (), min-level: 0, max-level: none, level-range: none, level-is-tree-depth: false, partial: true, allow-new-wildcards-on-rhs: false, rhs-cache-size: 100) => _replace(engine, expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size),
    replace-multiple: (expr, rules, repeat: false, once: false, bottom-up: false, nested: false) => _replace_multiple(engine, expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested),
    replace-wildcards: (pattern, replacements) => _replace_wildcards(engine, pattern, replacements),
    evaluate: (expr, values: ()) => _evaluate(engine, expr, values: values),
    domain: _domain,
    evaluate-many: (expressions, variables, points) => _evaluate_many(engine, expressions, variables, points),
    evaluate-grid: (expressions, variables, domains) => _evaluate_grid(engine, expressions, variables, domains),
    solve: (system, variables, domain: "complex") => _solve(engine, system, variables, domain: domain),
    nsolve: (expr, var, init, prec: 1e-4, max-iterations: 1000) => _nsolve(engine, expr, var, init, prec: prec, max-iterations: max-iterations),
    nsolve-system: (system, variables, init, prec: 1e-4, max-iterations: 1000) => _nsolve_system(engine, system, variables, init, prec: prec, max-iterations: max-iterations),
    matrix: value => _matrix(engine, value),
    vector: values => _vec(engine, values),
    identity: n => _identity(engine, n),
    eye: diag => _eye(engine, diag),
    matrix-add: (lhs, rhs) => _matrix_add(engine, lhs, rhs),
    matrix-sub: (lhs, rhs) => _matrix_sub(engine, lhs, rhs),
    matrix-mul: (lhs, rhs) => _matrix_mul(engine, lhs, rhs),
    matrix-div-scalar: (lhs, rhs) => _matrix_div_scalar(engine, lhs, rhs),
    transpose: matrix => _transpose(engine, matrix),
    determinant: matrix => _det(engine, matrix),
    inv: matrix => _inv(engine, matrix),
    matrix-solve: (A, b) => _matrix_solve(engine, A, b),
    matrix-solve-any: (A, b) => _matrix_solve_any(engine, A, b),
    row-reduce: (matrix, max-col: none) => _row_reduce(engine, matrix, max-col: max-col),
    augment: (lhs, rhs) => _augment(engine, lhs, rhs),
    split-col: (matrix, index) => _split_col(engine, matrix, index),
    primitive-part: matrix => _primitive_part(engine, matrix),
    matrix-content: matrix => _content(engine, matrix),
    matrix-at: (matrix, row, col) => _matrix_at(engine, matrix, row, col),
    matrix-shape: matrix => _matrix_shape(engine, matrix),
    matrix-is-zero: matrix => _matrix_is_zero(engine, matrix),
    matrix-is-diagonal: matrix => _matrix_is_diagonal(engine, matrix),
    matrix-derivative: (matrix, var) => _matrix_derivative(engine, matrix, var),
    add: (..terms) => _add(engine, ..terms),
    mul: (..factors) => _mul(engine, ..factors),
    neg: expr => _neg(engine, expr),
    subtract: (lhs, rhs) => _sub(engine, lhs, rhs),
    divide: (lhs, rhs) => _div(engine, lhs, rhs),
    pow: (base, exp) => _pow(engine, base, exp),
  )
  api
}

#let _default_engine() = init()

/// Draw the Symbolica logo inline with text.
///
/// The default size follows the surrounding font size. The logo is a vector
/// image and stays sharp when enlarged.
///
/// ```example
/// Calculated with #logo() Symbolica.\
/// #logo(size: 2em)
/// ```
///
/// -> content
#let logo(
  /// Width and height of the logo.
  /// -> length
  size: 1em,
) = box(
  baseline: 15%,
  image("assets/symbolica-logo.svg", width: size, height: size, alt: "Symbolica logo"),
)

/// Build a document-side Atom notation layer.
///
/// Exact `heads` entries change one namespaced symbol. Exact `calls` entries
/// receive the complete function node and its arguments. `tags` and `classes`
/// provide broader fallbacks. Renderer closures receive a context dictionary;
/// exact Atom metadata is attached by `to-typst` after a renderer returns.
///
/// -> dictionary
#let notation(
  heads: (:),
  calls: (:),
  tags: (:),
  classes: (:),
  fallback-head: none,
  fallback-call: none,
  fallback-node: none,
  fallback: none,
) = atom-render.notation(
  heads: heads,
  calls: calls,
  tags: tags,
  classes: classes,
  fallback-head: fallback-head,
  fallback-call: fallback-call,
  fallback-node: fallback-node,
  fallback: fallback,
)

/// Merge notation layers from least to most specific.
///
/// -> dictionary
#let merge-notation(..layers) = atom-render.merge-notation(..layers)

/// Convert math content, an expression string, or a number into an Atom payload.
///
/// Arithmetic, fractions, powers, roots, absolute values, calls, and common
/// Typst math structures are translated through the configured grammar. Matrix-valued
/// `mat(...)` and `vec(...)` content must instead be passed to `matrix` or
/// `vector`. Keep the returned bytes opaque and use this module's functions to
/// inspect or transform them. Decimal literals remain floating-point
/// coefficients; write a fraction when you need exact input.
/// Integers become exact numbers, floats keep their floating-point value, and
/// existing Atom payload bytes pass through unchanged. A namespace override
/// does not rename symbols in an existing payload. Matrix payloads belong in
/// the matrix APIs and are not scalar expressions.
///
/// Strings use Symbolica expression syntax, for example `"x^2/(1+x)"` or
/// `"f(x, y)"`. They are passed directly to Symbolica without evaluating Typst
/// source. Symbolica recognizes built-in names such as `sin`, `gamma`, and
/// `pi`; other unqualified names use the requested namespace.
/// A `grammar` override applies only to Typst content and is rejected
/// for other inputs. The engine's configured grammar also affects only content.
/// String arguments passed directly to algebra operations accept single
/// leaves: numbers or valid symbol names. They do not parse arithmetic
/// expressions, and invalid symbol names are rejected. Use `parse("x+1")` to
/// read an expression from a string.
///
/// Subscripts, primes, and corner attachments identify decorated symbols. Their
/// contents retain display order: `C_(i j)` differs from `C_(j i)`. Superscripts
/// remain algebraic powers, including `a_0^2`. Decoration is opaque to algebra:
/// replacing `i` does not modify the label in `C_(i j)`, and `h'(c)` names a
/// decorated function without implying a derivative. Use `h_(i)(c)` to keep a
/// subscript separate from the function arguments.
///
/// ```example
/// #let expr = parse($x + 1$)
/// #to-typst(expr)
/// #to-typst(parse("x^2/(1+x)"))
/// ```
///
/// -> bytes
#let parse(
  /// Math content (`$...$`), a Symbolica expression string, a number, or an
  /// existing Atom payload.
  /// -> bytes | content | int | float | str
  input,
  /// Typst content grammar override. `none` uses the engine grammar. An explicit
  /// grammar is rejected for strings, numbers, and existing payloads.
  /// -> dictionary | none
  grammar: none,
  /// Namespace for parsed symbols. `none` uses the engine namespace (`"typst"`
  /// for the top-level function).
  /// -> str | none
  namespace: none,
) = (_default_engine().parse)(input, grammar: grammar, namespace: namespace)

/// Construct an ordinary Symbolica symbol with an optional portable label.
///
/// A string supplies the symbol name. Math or text content supplies an opaque
/// label stored in Symbolica symbol data: `literal($x+y$)` is one variable,
/// while `parse($x+y$)` is a sum. Replacing `x` does not change that label.
/// Labels retain ordered attachments and are grouped when needed for display.
///
/// Equivalent labels in one namespace share an identity. `literal($x$)` equals
/// `literal("x")`, and `literal($a_0$)` equals `parse($a_0$)`. With `name`, the
/// supplied name determines identity; different names may share one label.
/// Declare a named label before using that name elsewhere. Conflicting labels
/// or tags for the same name are rejected.
///
/// The returned content carries exact Atom metadata, so interpolation and
/// reparsing `to-typst` retain the whole symbol. Plain `to-typst-source` output
/// preserves its appearance but carries no semantic metadata.
///
/// Portable labels support math elements and plain text. Unsupported styling,
/// layout, contextual content, or element fields produce an error. Use a named
/// string literal and a document-side `notation` renderer for custom content.
/// Names ending with `_` are reserved for `wild` and are rejected here.
///
/// ```example
/// #let label = literal($x+y$)
/// #to-typst(pow(label, 2))
/// #let x = literal("x", namespace: "model", tags: ("model::positive",))
/// #to-typst(parse($#x^2 + 1$))
/// ```
///
/// -> content
#let literal(
  /// Symbol-name string or opaque mathematical/text label.
  /// -> str | content
  input,
  /// Explicit identity for a content label; `none` derives identity from it.
  /// -> str | none
  name: none,
  /// Namespace override. `none` uses the engine namespace.
  /// -> str | none
  namespace: none,
  /// Portable semantic labels retained in Symbolica symbol data.
  /// -> array
  tags: (),
) = (_default_engine().literal)(input, name: name, namespace: namespace, tags: tags)

/// Construct a callable Symbolica function with semantic Typst metadata.
///
/// Calling the returned Typst function constructs the exact Symbolica call,
/// prints that Atom, and attaches one metadata envelope to the complete call.
/// Interpolate callable bindings inside math, for example `#f(x)`.
///
/// Use `namespace: "symbolica"` for built-in special functions, even when
/// Typst has no built-in function of that name. For example,
/// `function-head("polylog", namespace: "symbolica")(2, x)` constructs a polylogarithm.
/// Direct constructors are also available: `gamma-function`, `polygamma`, `polylog`,
/// `zeta-function`, `bessel-j`, `bessel-y`, `bessel-i`, and `bessel-k`. Unknown names
/// create ordinary symbolic functions; they do not acquire a numerical implementation.
///
/// ```example
/// #let f = function-head(
///   "f", namespace: "model", tags: ("model::smooth",),
/// )
/// #to-typst(parse($#f(literal("x")) + 1$))
/// ```
///
/// -> function
#let function-head(
  /// Function-head name.
  /// -> str
  name,
  /// Namespace override. `none` uses the engine namespace.
  /// -> str | none
  namespace: none,
  /// Portable semantic labels retained in the attached metadata.
  /// -> array
  tags: (),
) = (_default_engine().function-head)(name, namespace: namespace, tags: tags)

/// Construct the built-in gamma function.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::gamma`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #to-typst(gamma-function(5))
/// ```
///
/// -> content
#let gamma-function(
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().gamma-function)(z)

/// Construct the built-in polygamma function of order `n`.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::polygamma`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #to-typst(polygamma(1, 1))
/// ```
///
/// -> content
#let polygamma(
  /// Nonnegative integer order; zero gives the digamma function.
  /// -> bytes | content | int | float | str
  n,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().polygamma)(n, z)

/// Construct the built-in polylogarithm of order `s`.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::polylog`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #to-typst(polylog(2, literal("x")))
/// ```
///
/// -> content
#let polylog(
  /// Polylogarithm order.
  /// -> bytes | content | int | float | str
  s,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().polylog)(s, z)

/// Construct the built-in Riemann zeta function.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::zeta`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #to-typst(zeta-function(2))
/// ```
///
/// -> content
#let zeta-function(
  /// Function argument.
  /// -> bytes | content | int | float | str
  s,
) = (_default_engine().zeta-function)(s)

/// Construct the built-in Bessel function of the first kind.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::bessel_j`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #let value = evaluate(bessel-j(0, 1))
/// $ J_0(1) approx #calc.round(value.re, digits: 8) $
/// ```
///
/// -> content
#let bessel-j(
  /// Bessel function order.
  /// -> bytes | content | int | float | str
  nu,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().bessel-j)(nu, z)

/// Construct the built-in Bessel function of the second kind.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::bessel_y`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #let value = evaluate(bessel-y(0, 1))
/// $ Y_0(1) approx #calc.round(value.re, digits: 8) $
/// ```
///
/// -> content
#let bessel-y(
  /// Bessel function order.
  /// -> bytes | content | int | float | str
  nu,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().bessel-y)(nu, z)

/// Construct the built-in modified Bessel function of the first kind.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::bessel_i`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #let value = evaluate(bessel-i(0, 1))
/// $ I_0(1) approx #calc.round(value.re, digits: 8) $
/// ```
///
/// -> content
#let bessel-i(
  /// Bessel function order.
  /// -> bytes | content | int | float | str
  nu,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().bessel-i)(nu, z)

/// Construct the built-in modified Bessel function of the second kind.
///
/// Returns exact annotated math content, usable directly inside equations or
/// as input to algebra and evaluation. The head is always `symbolica::bessel_k`;
/// arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. Use `evaluate` for a numerical value,
/// or `to-float` for an approximate symbolic expression.
///
/// ```example
/// #let value = evaluate(bessel-k(0, 1))
/// $ K_0(1) approx #calc.round(value.re, digits: 8) $
/// ```
///
/// -> content
#let bessel-k(
  /// Bessel function order.
  /// -> bytes | content | int | float | str
  nu,
  /// Function argument.
  /// -> bytes | content | int | float | str
  z,
) = (_default_engine().bessel-k)(nu, z)

/// Construct a Symbolica pattern wildcard.
///
/// A wildcard captures part of a matching expression. Repeating the same
/// wildcard in a pattern requires equal captures. Its Symbolica name is `name`
/// followed by `level` underscores; use `literal` for an algebraic variable.
///
/// ```example
/// #let a = to-typst(wild("a"))
/// #let pattern = parse($f(#a, #a)$)
/// #to-typst(replace(parse($f(x, x) + f(x, y)$), pattern, 1))
/// ```
///
/// -> bytes
#let wild(
  /// Base name of the wildcard, without trailing underscores.
  /// -> str
  name,
  /// Positive number of underscore levels to append. `1` creates a conventional
  /// single wildcard such as `a_`. Zero is rejected; use `literal` for a symbol.
  /// -> int
  level: 1,
  /// Namespace override. `none` uses the engine namespace.
  /// -> str | none
  namespace: none,
) = (_default_engine().wild)(name, level: level, namespace: namespace)

/// Render the parse tree for a Typst math expression.
///
/// This is a diagnostic view of the tree consumed by `parse`; it does not create
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
) = (_default_engine().array-tree)(eqn, grammar: grammar)

/// Render an atom or matrix payload as Symbolica source text.
///
/// ```example
/// #raw(canonical(parse($x + 1$)))
/// ```
///
/// -> str
#let canonical(
  /// Atom or matrix payload. Other supported expression values are first
  /// converted to expressions; strings represent numbers or symbol names.
  /// -> bytes | content | int | float | str
  expr,
  /// Include symbol namespaces in the output.
  /// -> bool
  namespaces: false,
) = (_default_engine().canonical)(expr, namespaces: namespaces)

/// Render an atom or matrix payload as Typst math source.
///
/// ```example
/// #raw(to-typst-source(parse($x + 1$)))
/// ```
///
/// -> str
#let to-typst-source(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().to-typst-source)(expr)

/// Render an atom or matrix payload as evaluated Typst math content.
///
/// Symbolica exports a generic algebra tree; Typst constructs its visual math
/// directly from that tree. Ordinary calls retain an exact annotated head and
/// visible arguments. A custom full-call renderer receives the complete node,
/// and the framework attaches the exact call metadata after it returns.
/// Matrix output remains display-only.
///
/// ```example
/// #to-typst(parse($x + 1$))
/// ```
///
/// -> content
#let to-typst(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
  /// Per-call document notation layered over the engine's default. `none`
  /// uses that default unchanged.
  /// -> dictionary | none
  notation: none,
  /// Render as a block equation instead of inline math.
  /// -> bool
  block: false,
) = (_default_engine().to-typst)(expr, notation: notation, block: block)

/// Render an atom or matrix payload as LaTeX source.
///
/// ```example
/// #raw(to-latex(parse($x^2 + 1$)))
/// ```
///
/// -> str
#let to-latex(
  /// Atom or matrix payload, or a supported expression value.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().to-latex)(expr)

/// Validate and normalize an expression payload.
///
/// Parsing and algebra operations already combine equal terms and remove
/// factors of one. `simplify` does not search for a shorter equivalent formula:
/// use `expand`, `factor`, or `cancel-factors` for those transformations.
///
/// ```example
/// #let expr = parse($(x + 1)^2$)
/// #to-typst(simplify(expr))
/// ```
///
/// -> bytes
#let simplify(
  /// Atom payload or supported expression value to normalize.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().simplify)(expr)

/// Multiply out products and nonnegative integer powers of sums.
///
/// Use expansion to compare polynomial expressions: expand their difference
/// and check whether the result is zero.
///
/// ```example
/// #let factored = parse($(x - 1)(x + 1)$)
/// #let expanded = parse($x^2 - 1$)
/// #to-typst(expand(subtract(factored, expanded)))
/// ```
///
/// -> bytes
#let expand(
  /// Atom payload or supported expression value to expand.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().expand)(expr)

/// Write a polynomial as a product of exact factors.
///
/// By default, factors have rational coefficients: $x^2+1$ stays irreducible.
/// `complex: true` also permits rational multiples of the imaginary unit.
///
/// ```example
/// #let expr = parse($x^4 - 1$)
/// #to-typst(factor(expr))
/// ```
///
/// -> bytes
#let factor(
  /// Atom payload or supported expression value to factor.
  /// -> bytes | content | int | float | str
  expr,
  /// Factor over the complex rationals instead of the rationals.
  /// Cannot be combined with `square-free`.
  /// -> bool
  complex: false,
  /// Return a square-free factorization, preserving multiplicities without
  /// fully splitting every factor. Complex input is handled over the complex
  /// rationals.
  /// Cannot be combined with `complex`.
  /// -> bool
  square-free: false,
) = (_default_engine().factor)(expr, complex: complex, square-free: square-free)

/// Write a rational expression over a common denominator.
///
/// ```example
/// #to-typst(together(parse($1/x + 1/y$)))
/// ```
///
/// -> bytes
#let together(
  /// Expression whose rational terms should be combined.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().together)(expr)

/// Cancel common factors between numerators and denominators.
///
/// Parts of the expression without a cancellation are unchanged. Canceling
/// a factor can hide a point excluded by the original formula.
///
/// ```example
/// #to-typst(cancel-factors(parse($(x^2 - 1)/(x - 1)$)))
/// ```
///
/// -> bytes
#let cancel-factors(
  /// Rational expression in which to cancel common factors.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().cancel-factors)(expr)

/// Decompose a rational expression into partial fractions.
///
/// Denominators are decomposed in the given indeterminate.
///
/// ```example
/// #let x = literal("x")
/// #to-typst(apart(parse($(2 x + 3)/((x + 1)(x + 2))$), x))
/// ```
///
/// -> bytes
#let apart(
  /// Rational expression to decompose.
  /// -> bytes | content | int | float | str
  expr,
  /// Indeterminate for the decomposition.
  /// -> bytes | content | str
  var,
) = (_default_engine().apart)(expr, var)

/// Group terms with the same powers of selected variables or functions.
///
/// Other symbols remain in the coefficients. For example, collecting in $x$
/// writes $a x+b x$ as $(a+b)x$ without assigning values to $a$ or $b$.
///
/// ```example
/// #let x = literal("x")
/// #let expr = parse($a x + b x + a x^2 + b x^2$)
/// #to-typst(collect(expr, x))
/// ```
///
/// -> bytes
#let collect(
  /// Expression whose terms should be collected.
  /// -> bytes | content | int | float | str
  expr,
  /// One variable or function, or an array of them.
  /// -> bytes | content | str | array
  variables,
) = (_default_engine().collect)(expr, variables)

/// Extract the coefficient of a literal monomial or subexpression.
///
/// The result may depend on other symbols. Expand first when the desired
/// monomial is still inside a product or power.
///
/// ```example
/// #let expr = expand(parse($(x + y)^3$))
/// #to-typst(coefficient(expr, parse($x^2$)))
/// ```
///
/// -> bytes
#let coefficient(
  /// Expression to inspect.
  /// -> bytes | content | int | float | str
  expr,
  /// Literal monomial or subexpression whose coefficient is wanted.
  /// -> bytes | content | int | float | str
  monomial,
) = (_default_engine().coefficient)(expr, monomial)

/// Return collected key–coefficient pairs for one or more indeterminates.
///
/// Each result is `(key, coefficient)`, both as atom payloads. A key of `1`
/// carries terms not polynomially collected in the requested variables.
/// A coefficient that vanishes only through a deeper identity may remain.
///
/// ```example
/// #let x = literal("x")
/// #let pairs = coefficient-list(parse($x^2 + 5 x + 7$), x)
/// #pairs.map(
///   pair => [#to-typst(pair.at(0)): #to-typst(pair.at(1))],
/// ).join[, ]
/// ```
///
/// -> array
#let coefficient-list(
  /// Expression to inspect.
  /// -> bytes | content | int | float | str
  expr,
  /// One variable or function, or an array of them.
  /// -> bytes | content | str | array
  variables,
) = (_default_engine().coefficient-list)(expr, variables)

/// Return the top-level additive terms of an expression.
///
/// This does not expand or recurse. A value that is not a sum is returned as
/// a one-element array.
///
/// ```example
/// #summands(parse($x^2 + 2 x + 1$)).map(to-typst).join[, ]
/// ```
///
/// -> array
#let summands(
  /// Expression to split into top-level summands.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().summands)(expr)

/// Return the variables and function expressions that act as indeterminates.
///
/// Results are sorted in Symbolica's internal order.
///
/// ```example
/// #indeterminates(parse($f(x) + y$)).map(to-typst).join[, ]
/// ```
///
/// -> array
#let indeterminates(
  /// Expression to inspect.
  /// -> bytes | content | int | float | str
  expr,
  /// Traverse function arguments as well as collecting the function itself.
  /// -> bool
  enter-functions: true,
) = (_default_engine().indeterminates)(expr, enter-functions: enter-functions)

/// Test whether an expression literally contains another expression.
///
/// The test follows the stored expression structure: for example, `x*y*z`
/// contains `x`, but does not contain the regrouped product `x*y` as a node.
///
/// ```example
/// #contains(parse($x y z$), literal("x"))
/// ```
///
/// -> bool
#let contains(
  /// Expression to search.
  /// -> bytes | content | int | float | str
  expr,
  /// Literal subexpression to look for.
  /// -> bytes | content | int | float | str
  subexpression,
) = (_default_engine().contains)(expr, subexpression)

/// Test whether an expression has no user-defined variables or functions.
///
/// Supported built-in functions such as `sin` and `cos` remain constant when
/// all of their arguments are constant.
///
/// ```example
/// #is-constant(parse($cos(2) + 1/3$))
/// ```
///
/// -> bool
#let is-constant(
  /// Expression to test.
  /// -> bytes | content | int | float | str
  expr,
) = (_default_engine().is-constant)(expr)

/// Approximate numerical coefficients and built-in functions as decimals.
///
/// Variables remain symbolic. The WebAssembly build supports between 1 and 16
/// significant decimal digits. Use `evaluate` on the original expression when
/// you need a numerical value rather than a symbolic display form. Expressions
/// containing built-in calls with very large numeric arguments remain exact.
///
/// For decimal output, use `canonical` or `to-typst-source`. The structured
/// `to-typst` renderer can currently omit digits or misplace the decimal point.
///
/// ```example
/// #let (parse, to-float) = init(namespace: "symbolica")
/// #let expr = parse($x + pi$)
/// #let approximate = to-float(expr, decimal-prec: 6)
/// #raw(canonical(approximate))
/// ```
///
/// -> bytes
#let to-float(
  /// Expression to approximate.
  /// -> bytes | content | int | float | str
  expr,
  /// Number of significant decimal digits to retain, from 1 through 16.
  /// -> int
  decimal-prec: 16,
) = (_default_engine().to-float)(expr, decimal-prec: decimal-prec)

/// Differentiate with respect to one variable, holding other symbols constant.
///
/// Apply `derivative` again for a higher derivative or a mixed partial derivative.
///
/// ```example
/// #let x = literal("x")
/// #let f = parse($x^3 y + y^2$)
/// $ (partial f)/(partial x) = #to-typst(derivative(f, x)) $
/// ```
///
/// -> bytes
#let derivative(
  /// Expression to differentiate.
  /// -> bytes | content | int | float | str
  expr,
  /// Symbolica indeterminate, normally created with `literal`.
  /// -> bytes | content | str
  var,
) = (_default_engine().derivative)(expr, var)

/// Compute a univariate series expansion around `expansion-point`.
///
/// The truncation depth is the rational number `depth / depth-denom`. With an
/// absolute depth it is measured directly in `var`; with a relative depth it is
/// measured from the lowest order encountered in the expression.
///
/// ```example
/// #let x = literal("x")
/// #let approximation = series(parse($sin(x)$), x, 0, 5)
/// #to-typst(approximation)
/// ```
///
/// -> bytes
#let series(
  /// Expression to expand.
  /// -> bytes | content | int | float | str
  expr,
  /// Expansion variable, normally created with `literal`.
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
  _default_engine().series)(expr, var, expansion-point, depth, depth-denom: depth-denom, depth-is-absolute: depth-is-absolute)

/// Build a reusable replacement rule for `replace-multiple`.
///
/// Captures from `pattern` are substituted into `rhs`. Store rules in an array
/// to apply the same definitions to several expressions.
///
/// ```example
/// #let square = rule(parse("f(a_)"), parse("a_^2"))
/// #let shift = rule(parse("g(a_)"), parse("a_+1"))
/// #to-typst(replace-multiple(parse($f(x) + g(y)$), (square, shift)))
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
  _default_engine().rule)(pattern, rhs, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Replace subexpressions matching `pattern` with `rhs`.
///
/// A literal pattern replaces that exact expression, including occurrences
/// inside function arguments. Add wildcards when the rule should also match
/// related expressions. Matching uses the stored structure: expand first to
/// find terms hidden inside a power such as $(x+1)^2$.
///
/// By default, non-overlapping outermost matches are replaced once.
/// `repeat: true` reapplies the rule until nothing changes; cyclic rules do not
/// terminate. Use `replace-multiple` to apply several rules together.
///
/// ```example
/// #let x = literal("x")
/// #to-typst(replace(parse($x^2 + f(x)$), x, parse($y + 1$)))
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
  _default_engine().replace)(expr, pattern, rhs, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested, non-greedy-wildcards: non-greedy-wildcards, min-level: min-level, max-level: max-level, level-range: level-range, level-is-tree-depth: level-is-tree-depth, partial: partial, allow-new-wildcards-on-rhs: allow-new-wildcards-on-rhs, rhs-cache-size: rhs-cache-size)

/// Apply several reusable replacement rules together.
///
/// The traversal options apply to the combined rule set. With `repeat: true`,
/// the complete set is reapplied until no rule changes the expression; cyclic
/// rule sets do not terminate.
///
/// ```example
/// #let r1 = rule(parse($f("a_")$), parse($h("a_")$))
/// #let r2 = rule(literal("x"), literal("z"))
/// #to-typst(replace-multiple(parse($f(x) + x$), (r1, r2)))
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
  _default_engine().replace-multiple)(expr, rules, repeat: repeat, once: once, bottom-up: bottom-up, nested: nested)

/// Substitute explicit values for wildcard placeholders in a pattern.
///
/// This does not search another expression. It transforms `pattern` itself and
/// requires each key in `replacements` to be a wildcard symbol.
///
/// ```example
/// #to-typst(replace-wildcards(
///   parse($k("a_")$),
///   ((wild("a"), parse($x + 1$)),),
/// ))
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
) = (_default_engine().replace-wildcards)(pattern, replacements)

/// Evaluate one expression numerically with optional substitutions.
///
/// Supply a value for each unresolved variable or function call. Values may
/// be real numbers or complex dictionaries `(re: number, im: number)`.
/// A function-call key assigns a value to that call, not a definition for the
/// function. Use `replace` to substitute a symbolic definition first.
///
/// The result is always `(re: float, im: float)`. Read `.re` for a known real
/// result; use `to-float` instead if some variables should remain symbolic.
///
/// ```example
/// #let x = literal("x")
/// #let value = evaluate(parse($x^2 + f(2)$),
///   values: ((x, 3.0), (parse($f(2)$), 4.0)))
/// #repr(value)
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
) = (_default_engine().evaluate)(expr, values: values)

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
/// #let x = literal("x")
/// #let f = parse($x^3 - x$)
/// #let slope = derivative(f, x)
/// #let rows = evaluate-many((f, slope), x, (0, 1, 2))
/// #repr(rows.map(row => row.map(value => value.re)))
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
  _default_engine().evaluate-many)(expressions, variables, points)

/// Evaluate expressions over a Cartesian product of real domains in one batch.
///
/// The result is `(shape: array, points: array, values: array)`. `shape` lists
/// the sample count of every domain. The grid axes are flattened into rows with
/// the last domain varying fastest: each `points` row contains real coordinates
/// in variable order, while the corresponding `values` row contains one
/// `(re: float, im: float)` dictionary per expression.
///
/// ```example
/// #let x = literal("x")
/// #let y = literal("y")
/// #let grid = evaluate-grid(
///   parse($x^2 + y$), (x, y),
///   (domain(-1, 1, samples: 3), domain(0, 1, samples: 2)),
/// )
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
  _default_engine().evaluate-grid)(expressions, variables, domains)

/// Solve a linear or supported polynomial nonlinear system exactly.
///
/// Each expression in `system` is understood to equal zero. Polynomial systems
/// are solved exactly using Gröbner bases and algebraic roots;
/// coefficients may contain symbolic parameters when Symbolica can
/// treat them rationally. `domain` may be `"complex"`, `"real"`, `"rational"`,
/// or `"integer"`.
/// The domain also supplies local assumptions for unrestricted external
/// parameters: for example, `"real"` treats them as real while solving, without
/// changing their global symbol properties.
///
/// The result is a dictionary with `branches`, the requested `variables`, inferred
/// external `parameters`, and `domain`. `coverage` is `"complete"` or `"generic"`:
/// generic results cover only parameter values where every expression in
/// `coverage-guard` is nonzero. Other parameter values may have additional
/// solutions. An empty `branches` array proves there are no solutions only when
/// coverage is complete.
///
/// Each branch has a `values` array in the requested variable order. Free
/// variables appear as themselves in that array and are also listed in
/// `free-variables`. `conditions` retain restrictions: `"zero"`, `"nonzero"`, and
/// `"positive"` carry an `expression`; `"domain-membership"` carries a `variable`,
/// `value`, and `domain`. `conditional` reports whether these restrictions remain;
/// `point` means there are neither free variables nor branch conditions. The
/// result's coverage guard still applies even when a branch is a point.
///
/// Branches also provide `domain`, `dimension`, and `codimension` (the number of
/// requested variables minus the dimension). The result's `dimension` is the
/// largest branch dimension, or `-1` for the complete empty set. Dimensions are
/// `none` when Symbolica cannot establish them; they are not inferred by counting
/// free variables.
///
/// ```example
/// #let x = literal("x")
/// #let y = literal("y")
/// #let solutions = solve(
///   ($x^2 + y^2 - 5$, $y - 2x$).map(parse),
///   (x, y), domain: "real",
/// )
/// #for branch in solutions.branches {
///   let (x, y) = branch.values.map(to-typst)
///   $ x = #x, quad y = #y $
/// }
/// ```
///
/// -> dictionary
#let solve(
  /// Expressions understood to equal zero.
  /// -> array | content | int | float | str
  system,
  /// Variables to solve for, in `values` order.
  /// -> array | content | str
  variables,
  /// Exact solution domain: `"complex"`, `"real"`, `"rational"`, or `"integer"`.
  /// -> str
  domain: "complex",
) = (_default_engine().solve)(system, variables, domain: domain)

/// Find a real root of a univariate expression with Newton's method.
///
/// The expression is interpreted as equal to zero and evaluated with `f64`
/// arithmetic. Convergence is local and depends on `init`; failure to converge
/// within `max-iterations` produces an error.
///
/// ```example
/// #let x = literal("x")
/// #repr(nsolve(parse($x^2 - 2$), x, 1.0))
/// ```
///
/// -> float
#let nsolve(
  /// Expression understood to equal zero.
  /// -> bytes | content | int | float | str
  expr,
  /// Real solve variable, normally created with `literal`.
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
  _default_engine().nsolve)(expr, var, init, prec: prec, max-iterations: max-iterations)

/// Find a common real root of a system with multivariate Newton iteration.
///
/// Every expression is interpreted as equal to zero. `variables` and `init`
/// must have matching lengths, and the returned floats follow that same order.
/// Convergence is local and is not guaranteed for an arbitrary initial guess.
///
/// ```example
/// #let x = literal("x")
/// #let y = literal("y")
/// #repr(nsolve-system(
///   (parse($x^2 + y - 3$), parse($x - y$)),
///   (x, y), (1.0, 1.0),
/// ))
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
  _default_engine().nsolve-system)(system, variables, init, prec: prec, max-iterations: max-iterations)

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
) = (_default_engine().matrix)(value)

/// Convert a non-empty sequence into a column-vector matrix payload.
///
/// Array entries must be rational-polynomial compatible. Typst math `vec(...)`
/// is accepted directly; existing matrix bytes pass through unchanged.
///
/// ```example
/// #to-typst(vector((1, 2)))
/// ```
///
/// -> bytes
#let vector(
  /// Vector entries, Typst `vec(...)` content, or an existing matrix payload.
  /// -> array | content | bytes
  values,
) = (_default_engine().vector)(values)

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
) = (_default_engine().identity)(n)

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
) = (_default_engine().eye)(diag)

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
) = (_default_engine().matrix-add)(lhs, rhs)

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
) = (_default_engine().matrix-sub)(lhs, rhs)

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
) = (_default_engine().matrix-mul)(lhs, rhs)

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
) = (_default_engine().matrix-div-scalar)(lhs, rhs)

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
) = (_default_engine().transpose)(matrix)

/// Compute the exact determinant of a square matrix.
///
/// The result is an atom payload rather than a matrix payload.
///
/// ```example
/// #to-typst(determinant(matrix(((1, 2), (3, 4)))))
/// ```
///
/// -> bytes
#let determinant(
  /// Square matrix.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine().determinant)(matrix)

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
) = (_default_engine().inv)(matrix)

/// Solve the matrix equation `A x = b` exactly.
///
/// The row counts of `A` and `b` must agree. This strict solver reports an
/// error when the system does not have the required unique solution; use
/// `matrix-solve-any` for underdetermined systems.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vector((5, 1))
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
) = (_default_engine().matrix-solve)(A, b)

/// Solve `A x = b` exactly, choosing one solution if underdetermined.
///
/// The row counts of `A` and `b` must agree. An inconsistent system still
/// produces an error.
///
/// ```example
/// #let A = matrix($mat(2, 1; 1, -1)$)
/// #let b = vector((5, 1))
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
) = (_default_engine().matrix-solve-any)(A, b)

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
) = (_default_engine().row-reduce)(matrix, max-col: max-col)

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
) = (_default_engine().augment)(lhs, rhs)

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
) = (_default_engine().split-col)(matrix, index)

/// Divide a rational-polynomial matrix by its content.
///
/// The result is the primitive matrix whose coefficient GCD has been removed.
///
/// ```example
/// #let x = literal("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(primitive-part(P))
/// ```
///
/// -> bytes
#let primitive-part(
  /// Matrix whose entries are rational polynomials.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine().primitive-part)(matrix)

/// Compute the coefficient content of a rational-polynomial matrix.
///
/// The content is the common coefficient GCD and is returned as an atom
/// payload.
///
/// ```example
/// #let x = literal("x")
/// #let P = matrix(((mul(2, x), mul(4, x)), (mul(6, x), mul(8, x))))
/// #to-typst(matrix-content(P))
/// ```
///
/// -> bytes
#let matrix-content(
  /// Matrix whose entries are rational polynomials.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine().matrix-content)(matrix)

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
) = (_default_engine().matrix-at)(matrix, row, col)

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
) = (_default_engine().matrix-shape)(matrix)

/// Test exactly whether every matrix entry is zero.
///
/// This is an exact symbolic predicate, not a floating-point tolerance test.
///
/// ```example
/// #matrix-is-zero(matrix(((0, 0), (0, 0))))
/// ```
///
/// -> bool
#let matrix-is-zero(
  /// Matrix to inspect.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine().matrix-is-zero)(matrix)

/// Test exactly whether every off-diagonal matrix entry is zero.
///
/// Rectangular matrices are accepted; entries outside the main diagonal must
/// be zero.
///
/// ```example
/// #matrix-is-diagonal(matrix(((1, 0), (0, 2))))
/// ```
///
/// -> bool
#let matrix-is-diagonal(
  /// Matrix to inspect.
  /// -> bytes | content | array | int | float | str
  matrix,
) = (_default_engine().matrix-is-diagonal)(matrix)

/// Differentiate every matrix entry with respect to an indeterminate.
///
/// ```example
/// #let x = literal("x")
/// #to-typst(matrix-derivative(matrix(((pow(x, 2), x), (1, 0))), x))
/// ```
///
/// -> bytes
#let matrix-derivative(
  /// Rational-polynomial matrix to differentiate.
  /// -> bytes | content | array | int | float | str
  matrix,
  /// Indeterminate with respect to which each entry is differentiated.
  /// -> bytes | content | str
  var,
) = (_default_engine().matrix-derivative)(matrix, var)

/// Construct an exact sum from expression values.
///
/// Arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. With no arguments, the result is
/// the additive identity zero.
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
) = (_default_engine().add)(..terms)

/// Construct an exact product from expression values.
///
/// Arguments accept numbers, math content, expression payloads, or single-name
/// strings. Use `parse` for expression strings. With no arguments, the result is
/// the multiplicative identity one.
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
) = (_default_engine().mul)(..factors)

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
) = (_default_engine().neg)(expr)

/// Subtract `rhs` from `lhs` exactly.
///
/// ```example
/// #to-typst(subtract("x", "y"))
/// ```
///
/// -> bytes
#let subtract(
  /// Minuend.
  /// -> bytes | content | int | float | str
  lhs,
  /// Subtrahend.
  /// -> bytes | content | int | float | str
  rhs,
) = (_default_engine().subtract)(lhs, rhs)

/// Construct the exact quotient `lhs / rhs`.
///
/// ```example
/// #to-typst(divide(1, "x"))
/// ```
///
/// -> bytes
#let divide(
  /// Numerator.
  /// -> bytes | content | int | float | str
  lhs,
  /// Denominator.
  /// -> bytes | content | int | float | str
  rhs,
) = (_default_engine().divide)(lhs, rhs)

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
) = (_default_engine().pow)(base, exp)

/// Integrate an expression using the Rubi rules.
///
/// The variable must represent one symbol. Returns a best-effort antiderivative
/// without an integration constant; unsupported parts remain unevaluated.
///
/// The first call to `integrate` or `integrate-with-steps` compiles and
/// initializes the integration rule sets, which may take about 10 seconds.
/// Both functions share the cached rules, so later calls and ordinary edits
/// reuse them. Restarting the compiler or clearing its cache repeats this setup.
///
/// ```example
/// #to-typst(integrate(parse($x^2$), literal("x")))
/// ```
///
/// -> bytes
#let integrate(
  /// Integrand as an Atom payload or supported expression.
  /// -> bytes | content | int | float | str
  expression,
  /// Integration variable as a symbol or Atom payload.
  /// -> bytes | content | str
  variable,
) = (_default_engine().integrate)(expression, variable)

/// Integrate an expression and return the nested Rubi rule trace.
///
/// The result contains `result` (Atom payload), `complete` (boolean), and
/// `steps` (array). Each step contains `rule`, `depth`, `description`,
/// `references`, `source`, `input`, and `output`. Render the input and output
/// Atom payloads with `to-typst`. No integration constant is added.
///
/// The first call to `integrate` or `integrate-with-steps` compiles and
/// initializes the integration rule sets, which may take about 10 seconds.
/// Both functions share the cached rules, so later calls and ordinary edits
/// reuse them. Restarting the compiler or clearing its cache repeats this setup.
///
/// -> dictionary
#let integrate-with-steps(
  /// Integrand as an Atom payload or supported expression.
  /// -> bytes | content | int | float | str
  expression,
  /// Integration variable as a symbol or Atom payload.
  /// -> bytes | content | str
  variable,
) = (_default_engine().integrate-with-steps)(expression, variable)
