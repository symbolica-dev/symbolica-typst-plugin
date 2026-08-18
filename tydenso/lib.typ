#let _decompress-bundled(path) = plugin("tydenso-inflate.wasm").decompress(
  read(path, encoding: none),
)
#let _bundled-plugin() = plugin(_decompress-bundled("tydenso.wasm.zlib"))

#let _plain-representation(value) = (
  kind: "representation",
  name: value.name,
  namespace: value.namespace,
  dimension: value.dimension,
  self-dual: value.self-dual,
  is-dual: value.is-dual,
  dual-name: value.dual-name,
)

#let _portable(value) = {
  if type(value) == dictionary and value.at("kind", default: none) == "representation" {
    return _plain-representation(value)
  }
  if type(value) == dictionary and value.at("kind", default: none) == "slot" {
    return (
      kind: "slot",
      representation: _plain-representation(value.representation),
      index: _portable(value.index),
      dual: value.dual,
    )
  }
  if type(value) == array {
    return value.map(_portable)
  }
  value
}

#let _construct(engine, value) = engine.plugin.construct(cbor.encode(_portable(value)))
#let _payload(engine, value, label: "expression") = {
  if type(value) == bytes { return value }
  if type(value) in (int, float, str, dictionary) { return _construct(engine, value) }
  panic(label + " must be an Atom payload or a Tydenso constructor value")
}

#let _slot(representation, index, dual: none) = (
  kind: "slot",
  representation: _plain-representation(representation),
  index: index,
  dual: if dual == none { representation.is-dual } else { dual },
)

#let _as-slot(representation, value, dual: none) = {
  if type(value) == dictionary and value.at("kind", default: none) == "slot" {
    value
  } else {
    _slot(representation, value, dual: dual)
  }
}

#let _call(
  engine,
  name,
  arguments,
  namespace: "spenso",
  symmetric: false,
  antisymmetric: false,
  cycle-symmetric: false,
  linear: false,
) = _construct(engine, (
  kind: "call",
  name: name,
  namespace: namespace,
  arguments: arguments.map(_portable),
  symmetric: symmetric,
  antisymmetric: antisymmetric,
  cycle-symmetric: cycle-symmetric,
  linear: linear,
))

#let _tensor(
  engine,
  name,
  rank-one: false,
  namespace: "spenso",
  symmetric: false,
  antisymmetric: false,
  cycle-symmetric: false,
  linear: false,
) = (..arguments) => _construct(engine, (
  kind: if rank-one { "vector" } else { "tensor" },
  name: name,
  namespace: namespace,
  arguments: arguments.pos().map(_portable),
  symmetric: symmetric,
  antisymmetric: antisymmetric,
  cycle-symmetric: cycle-symmetric,
  linear: linear,
))

#let _representation(
  engine,
  name,
  dimension,
  namespace: "spenso",
  self-dual: false,
  is-dual: false,
  dual-name: none,
) = {
  let descriptor = (
    kind: "representation",
    name: name,
    namespace: namespace,
    dimension: dimension,
    self-dual: self-dual,
    is-dual: is-dual,
    dual-name: if dual-name == none { name } else { dual-name },
  )
  (
    ..descriptor,
    slot: (index, dual: none) => _slot(descriptor, index, dual: dual),
    dual: () => _representation(
      engine,
      descriptor.dual-name,
      dimension,
      namespace: namespace,
      self-dual: self-dual,
      is-dual: if self-dual { false } else { not is-dual },
      dual-name: name,
    ),
    metric: (first, second) => _call(engine, "g", (
      _as-slot(descriptor, first),
      _as-slot(descriptor, second),
    )),
    identity: (first, second) => _call(engine, "id", (
      _as-slot(descriptor, first),
      _as-slot(descriptor, second),
    )),
    flat: (first, second) => _call(engine, "flat", (
      _as-slot(descriptor, first),
      _as-slot(descriptor, second),
    )),
  )
}

#let _print-settings(
  preset: "typst",
  with-dim: none,
  parens: none,
  commas: none,
  index-subscripts: none,
  symbol-scripts: none,
) = {
  assert(preset in ("typst", "compact"), message: "preset must be 'typst' or 'compact'")
  let defaults = if preset == "typst" {
    (with-dim: false, parens: true, commas: false, index-subscripts: true, symbol-scripts: true)
  } else {
    (with-dim: false, parens: true, commas: false, index-subscripts: false, symbol-scripts: false)
  }
  (
    preset: preset,
    with-dim: if with-dim == none { defaults.with-dim } else { with-dim },
    parens: if parens == none { defaults.parens } else { parens },
    commas: if commas == none { defaults.commas } else { commas },
    index-subscripts: if index-subscripts == none { defaults.index-subscripts } else { index-subscripts },
    symbol-scripts: if symbol-scripts == none { defaults.symbol-scripts } else { symbol-scripts },
  )
}

#let _to-typst-source(engine, expression, settings: _print-settings()) = str(
  engine.plugin.to_typst(cbor.encode((
    expr: _payload(engine, expression),
    settings: settings,
  ))),
)

#let _to-typst(engine, expression, settings: _print-settings(), block: false) = {
  let equation = eval(_to-typst-source(engine, expression, settings: settings), mode: "math")
  if block { math.equation(equation.body, block: true) } else { equation }
}

#let _to-string(engine, expression, settings: _print-settings(preset: "compact")) = str(
  engine.plugin.to_string(cbor.encode((
    expr: _payload(engine, expression),
    settings: settings,
  ))),
)

#let _inspect(engine, expression) = cbor(engine.plugin.inspect(_payload(engine, expression)))

/// Create an independent Tydenso API.
///
/// The returned dictionary contains tensor constructors, Spenso-aware printers,
/// CBOR inspection, and Idenso transformations. Use a custom `source` only when
/// developing another compatible plugin; ordinary documents can use the
/// imported top-level functions.
///
/// ```example
/// #let tensors = init()
/// #let V = (tensors.mink)(4)
/// #let p = (tensors.vector)("p")
/// #(tensors.to-typst)(p((tensors.slot)(V, "mu")))
/// ```
///
/// -> dictionary
#let init(
  /// WebAssembly plugin path or uncompressed module bytes. `none` selects the
  /// bundled compressed Tydenso engine.
  /// -> str | bytes | none
  source: none,
) = {
  let plugin-module = if source == none { _bundled-plugin() } else { plugin(source) }
  let engine = (plugin: plugin-module,)
  (
    plugin: plugin-module,
    construct: value => _construct(engine, value),
    symbol: (name, namespace: "spenso") => _construct(engine, (
      kind: "symbol", name: name, namespace: namespace,
    )),
    tensor: (name, namespace: "spenso", symmetric: false, antisymmetric: false, cycle-symmetric: false, linear: false) => _tensor(
      engine, name,
      namespace: namespace,
      symmetric: symmetric,
      antisymmetric: antisymmetric,
      cycle-symmetric: cycle-symmetric,
      linear: linear,
    ),
    vector: (name, namespace: "spenso", linear: false) => _tensor(
      engine, name,
      rank-one: true,
      namespace: namespace,
      linear: linear,
    ),
    representation: (name, dimension, namespace: "spenso", self-dual: false, is-dual: false, dual-name: none) => _representation(
      engine, name, dimension,
      namespace: namespace,
      self-dual: self-dual,
      is-dual: is-dual,
      dual-name: dual-name,
    ),
    mink: dimension => _representation(engine, "mink", dimension, self-dual: true),
    euc: dimension => _representation(engine, "euc", dimension, self-dual: true),
    lor: dimension => _representation(engine, "lor", dimension, dual-name: "lor"),
    bis: dimension => _representation(engine, "bis", dimension, self-dual: true),
    spf: dimension => _representation(engine, "spf", dimension, dual-name: "spf"),
    cof: dimension => _representation(engine, "cof", dimension, dual-name: "cof"),
    coad: dimension => _representation(engine, "coad", dimension, self-dual: true),
    cos: dimension => _representation(engine, "cos", dimension, dual-name: "cos"),
    slot: (representation, index, dual: none) => _slot(representation, index, dual: dual),
    metric: (representation, first, second) => _call(engine, "g", (
      _as-slot(representation, first), _as-slot(representation, second),
    )),
    identity-tensor: (representation, first, second) => _call(engine, "id", (
      _as-slot(representation, first), _as-slot(representation, second),
    )),
    flat-tensor: (representation, first, second) => _call(engine, "flat", (
      _as-slot(representation, first), _as-slot(representation, second),
    )),
    dual-representation: representation => (representation.dual)(),
    add: (..terms) => _construct(engine, (kind: "sum", terms: terms.pos().map(_portable))),
    mul: (..factors) => _construct(engine, (kind: "product", factors: factors.pos().map(_portable))),
    neg: expression => _construct(engine, (kind: "negative", expression: _portable(expression))),
    sub: (left, right) => _construct(engine, (
      kind: "sum",
      terms: (_portable(left), (kind: "negative", expression: _portable(right))),
    )),
    div: (numerator, denominator) => _construct(engine, (
      kind: "product",
      factors: (
        _portable(numerator),
        (kind: "power", base: _portable(denominator), exponent: -1),
      ),
    )),
    pow: (base, exponent) => _construct(engine, (
      kind: "power", base: _portable(base), exponent: _portable(exponent),
    )),
    print-settings: (preset: "typst", with-dim: none, parens: none, commas: none, index-subscripts: none, symbol-scripts: none) => _print-settings(
      preset: preset,
      with-dim: with-dim,
      parens: parens,
      commas: commas,
      index-subscripts: index-subscripts,
      symbol-scripts: symbol-scripts,
    ),
    to-typst-source: (expression, settings: _print-settings()) => _to-typst-source(engine, expression, settings: settings),
    to-typst: (expression, settings: _print-settings(), block: false) => _to-typst(engine, expression, settings: settings, block: block),
    to-string: (expression, settings: _print-settings(preset: "compact")) => _to-string(engine, expression, settings: settings),
    inspect: expression => _inspect(engine, expression),
    cook-function: expression => plugin-module.cook_function(_payload(engine, expression)),
    cook-indices: expression => plugin-module.cook_indices(_payload(engine, expression)),
    dirac-adjoint: expression => plugin-module.dirac_adjoint(_payload(engine, expression)),
    expand-bis: expression => plugin-module.expand_bis(_payload(engine, expression)),
    expand-color: expression => plugin-module.expand_color(_payload(engine, expression)),
    expand-metrics: expression => plugin-module.expand_metrics(_payload(engine, expression)),
    expand-mink: expression => plugin-module.expand_mink(_payload(engine, expression)),
    expand-mink-bis: expression => plugin-module.expand_mink_bis(_payload(engine, expression)),
    list-dangling: expression => cbor(plugin-module.list_dangling(_payload(engine, expression))),
    simplify-color: expression => plugin-module.simplify_color(_payload(engine, expression)),
    simplify-gamma: expression => plugin-module.simplify_gamma(_payload(engine, expression)),
    simplify-metrics: expression => plugin-module.simplify_metrics(_payload(engine, expression)),
    to-dots: expression => plugin-module.to_dots(_payload(engine, expression)),
    wrap-dummies: (expression, header) => plugin-module.wrap_dummies(
      _payload(engine, expression), _payload(engine, header, label: "header"),
    ),
    wrap-indices: (expression, header) => plugin-module.wrap_indices(
      _payload(engine, expression), _payload(engine, header, label: "header"),
    ),
  )
}

#let _default-engine() = init()

/// Construct a named tensor function.
///
/// The result is callable with any mixture of slots, representations, scalar
/// values, and compatible Atom payloads. Symmetry flags follow Spenso's
/// `TensorName` model and are registered on the underlying Symbolica symbol.
///
/// ```example
/// #let V = mink(4)
/// #let Z = tensor("Z")
/// #to-typst(Z(slot(V, "mu"), slot(V, "nu")))
/// ```
///
/// -> function
#let tensor(
  /// Tensor name.
  /// -> str
  name,
  /// Symbol namespace.
  /// -> str
  namespace: "spenso",
  /// Make the tensor symmetric under argument permutation.
  /// -> bool
  symmetric: false,
  /// Make the tensor antisymmetric under argument permutation.
  /// -> bool
  antisymmetric: false,
  /// Make the tensor symmetric under cyclic argument permutation.
  /// -> bool
  cycle-symmetric: false,
  /// Mark the tensor function as linear.
  /// -> bool
  linear: false,
) = (_default-engine().tensor)(
  name,
  namespace: namespace,
  symmetric: symmetric,
  antisymmetric: antisymmetric,
  cycle-symmetric: cycle-symmetric,
  linear: linear,
)

/// Construct a named rank-one tensor function.
///
/// Non-slot arguments are rendered as symbol subscripts in Spenso's Typst
/// mode, while the vector slot is rendered as an abstract index.
///
/// ```example
/// #let V = mink(4)
/// #let p = vector("p")
/// #to-typst(p(1, slot(V, "mu")))
/// ```
///
/// -> function
#let vector(
  /// Vector name.
  /// -> str
  name,
  /// Symbol namespace.
  /// -> str
  namespace: "spenso",
  /// Mark the vector function as linear.
  /// -> bool
  linear: false,
) = (_default-engine().vector)(
  name,
  namespace: namespace,
  linear: linear,
)

/// Construct a symbolic scalar.
///
/// -> bytes
#let symbol(
  /// Symbol name.
  /// -> str
  name,
  /// Symbol namespace.
  /// -> str
  namespace: "spenso",
) = (_default-engine().symbol)(name, namespace: namespace)

/// Describe a representation and attach its tensor-building methods.
///
/// The returned dictionary is inspectable Typst data. Its `slot`, `metric`,
/// `identity`, `flat`, and `dual` fields are functions.
///
/// -> dictionary
#let representation(
  /// Symbolic representation name.
  /// -> str
  name,
  /// Dimension, which may itself be symbolic.
  /// -> int | str | bytes
  dimension,
  /// Symbol namespace.
  /// -> str
  namespace: "spenso",
  /// Whether the representation is its own dual.
  /// -> bool
  self-dual: false,
  /// Whether this value denotes the dual orientation of a non-self-dual
  /// representation.
  /// -> bool
  is-dual: false,
  /// Name used by the representation returned from `dual`.
  /// -> str | none
  dual-name: none,
) = (_default-engine().representation)(
  name, dimension,
  namespace: namespace,
  self-dual: self-dual,
  is-dual: is-dual,
  dual-name: dual-name,
)

/// Construct a Minkowski representation.
///
/// -> dictionary
#let mink(dimension) = (_default-engine().mink)(dimension)

/// Construct a Euclidean representation.
///
/// -> dictionary
#let euc(dimension) = (_default-engine().euc)(dimension)

/// Construct a Lorentz representation.
///
/// -> dictionary
#let lor(dimension) = (_default-engine().lor)(dimension)

/// Construct a bispinor representation.
///
/// -> dictionary
#let bis(dimension) = (_default-engine().bis)(dimension)

/// Construct a spin-fundamental representation.
///
/// -> dictionary
#let spf(dimension) = (_default-engine().spf)(dimension)

/// Construct a color-fundamental representation.
///
/// -> dictionary
#let cof(dimension) = (_default-engine().cof)(dimension)

/// Construct a color-adjoint representation.
///
/// -> dictionary
#let coad(dimension) = (_default-engine().coad)(dimension)

/// Construct a color-sextet representation.
///
/// -> dictionary
#let cos(dimension) = (_default-engine().cos)(dimension)

/// Construct an indexed slot from a representation.
///
/// The result is an ordinary dictionary, so its representation, index, and
/// variance remain visible to Typst before an expression is constructed.
///
/// -> dictionary
#let slot(
  /// Representation dictionary.
  /// -> dictionary
  representation,
  /// Abstract index label.
  /// -> int | str | bytes
  index,
  /// Wrap the slot with Spenso's dual-index marker. `none` inherits the
  /// representation's `is-dual` field.
  /// -> bool | none
  dual: none,
) = (_default-engine().slot)(representation, index, dual: dual)

/// Construct the metric tensor for a representation.
///
/// Index labels are converted to slots automatically; existing slots pass
/// through unchanged.
///
/// -> bytes
#let metric(representation, first, second) = (
  _default-engine().metric
)(representation, first, second)

/// Construct the identity tensor for a representation.
///
/// -> bytes
#let identity-tensor(representation, first, second) = (
  _default-engine().identity-tensor
)(representation, first, second)

/// Construct Spenso's musical isomorphism tensor for a representation.
///
/// -> bytes
#let flat-tensor(representation, first, second) = (
  _default-engine().flat-tensor
)(representation, first, second)

/// Return the representation paired with this representation.
///
/// -> dictionary
#let dual-representation(representation) = (
  _default-engine().dual-representation
)(representation)

/// Add symbolic expressions or constructor values.
///
/// -> bytes
#let add(..terms) = (_default-engine().add)(..terms)

/// Multiply symbolic expressions or constructor values.
///
/// -> bytes
#let mul(..factors) = (_default-engine().mul)(..factors)

/// Negate a symbolic expression or constructor value.
///
/// -> bytes
#let neg(expression) = (_default-engine().neg)(expression)

/// Subtract two symbolic expressions or constructor values.
///
/// -> bytes
#let sub(left, right) = (_default-engine().sub)(left, right)

/// Divide two symbolic expressions or constructor values.
///
/// -> bytes
#let div(numerator, denominator) = (_default-engine().div)(numerator, denominator)

/// Raise a symbolic expression or constructor value to a power.
///
/// -> bytes
#let pow(base, exponent) = (_default-engine().pow)(base, exponent)

/// Create Spenso printer settings.
///
/// `"typst"` uses valid Typst math syntax with indexed tensor notation;
/// `"compact"` uses Spenso's concise Symbolica-style form. Any field may be
/// overridden without changing the others.
///
/// -> dictionary
#let print-settings(
  /// Base preset: `"typst"` or `"compact"`.
  /// -> str
  preset: "typst",
  /// Include representation dimensions in printed slots.
  /// -> bool | none
  with-dim: none,
  /// Use parentheses around tensor arguments.
  /// -> bool | none
  parens: none,
  /// Separate tensor arguments with commas.
  /// -> bool | none
  commas: none,
  /// Render indices as subscripts where supported.
  /// -> bool | none
  index-subscripts: none,
  /// Render non-slot tensor arguments as symbol scripts where supported.
  /// -> bool | none
  symbol-scripts: none,
) = (_default-engine().print-settings)(
  preset: preset,
  with-dim: with-dim,
  parens: parens,
  commas: commas,
  index-subscripts: index-subscripts,
  symbol-scripts: symbol-scripts,
)

/// Print an expression as Typst math source using Spenso's printer.
///
/// -> str
#let to-typst-source(
  /// Atom payload or constructor value.
  expression,
  /// Settings created by `print-settings`.
  /// -> dictionary
  settings: _print-settings(),
) = (_default-engine().to-typst-source)(expression, settings: settings)

/// Print and evaluate an expression as Typst math content.
///
/// -> content
#let to-typst(
  /// Atom payload or constructor value.
  expression,
  /// Settings created by `print-settings`.
  /// -> dictionary
  settings: _print-settings(),
  /// Display the result as a block equation.
  /// -> bool
  block: false,
) = (_default-engine().to-typst)(expression, settings: settings, block: block)

/// Print an expression in Spenso's compact Symbolica notation.
///
/// -> str
#let to-string(
  /// Atom payload or constructor value.
  expression,
  /// Settings created by `print-settings`.
  /// -> dictionary
  settings: _print-settings(preset: "compact"),
) = (_default-engine().to-string)(expression, settings: settings)

/// Decode an Atom payload into a recursive CBOR expression tree.
///
/// The result uses `kind` values such as `symbol`, `number`, `function`,
/// `sum`, `product`, and `power`. Function nodes include their name, arguments,
/// and symmetry flags. This view is ordinary Typst data; transforming the
/// expression still uses its lossless Atom payload.
///
/// -> dictionary
#let inspect(expression) = (_default-engine().inspect)(expression)

/// Cook a tensor function into Idenso's canonical structure.
///
/// -> bytes
#let cook-function(expression) = (_default-engine().cook-function)(expression)

/// Cook indices into Idenso's canonical structure.
///
/// -> bytes
#let cook-indices(expression) = (_default-engine().cook-indices)(expression)

/// Take the Dirac adjoint of a tensor expression.
///
/// -> bytes
#let dirac-adjoint(expression) = (_default-engine().dirac-adjoint)(expression)

/// Selectively expand bispinor structures.
///
/// -> bytes
#let expand-bis(expression) = (_default-engine().expand-bis)(expression)

/// Selectively expand color structures.
///
/// -> bytes
#let expand-color(expression) = (_default-engine().expand-color)(expression)

/// Expand all supported metric structures.
///
/// -> bytes
#let expand-metrics(expression) = (_default-engine().expand-metrics)(expression)

/// Selectively expand Minkowski structures.
///
/// -> bytes
#let expand-mink(expression) = (_default-engine().expand-mink)(expression)

/// Selectively expand combined Minkowski and bispinor structures.
///
/// -> bytes
#let expand-mink-bis(expression) = (_default-engine().expand-mink-bis)(expression)

/// Return the dangling indices as compatible Atom payloads.
///
/// -> array
#let list-dangling(expression) = (_default-engine().list-dangling)(expression)

/// Simplify color tensors.
///
/// -> bytes
#let simplify-color(expression) = (_default-engine().simplify-color)(expression)

/// Simplify gamma-matrix expressions.
///
/// -> bytes
#let simplify-gamma(expression) = (_default-engine().simplify-gamma)(expression)

/// Contract and simplify metric tensors.
///
/// -> bytes
#let simplify-metrics(expression) = (_default-engine().simplify-metrics)(expression)

/// Rewrite supported contractions as dot products.
///
/// -> bytes
#let to-dots(expression) = (_default-engine().to-dots)(expression)

/// Wrap dummy indices under the given header symbol.
///
/// -> bytes
#let wrap-dummies(expression, header) = (_default-engine().wrap-dummies)(expression, header)

/// Wrap all indices under the given header symbol.
///
/// -> bytes
#let wrap-indices(expression, header) = (_default-engine().wrap-indices)(expression, header)
