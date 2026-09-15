#let _payload(value, label) = {
  if type(value) != bytes {
    panic(label + " must be portable Symbolica Atom payload bytes")
  }
  value
}

// Keep module construction inside a pure, zero-argument function. Importing
// this package does not evaluate either asset. Typst is free to cache calls,
// but correctness does not depend on reuse: Wizer gives every instance the
// same preinitialized Rubi tables.
#let _rubi-plugin() = {
  let inflater = plugin("symbolica-inflate.wasm")
  let module = inflater.decompress(
    read("symbolica-integrate.wasm.zlib", encoding: none),
  )
  plugin(module)
}

/// Integrate a portable Symbolica Atom payload with Rubi.
///
/// The integration variable must be an Atom payload containing one symbol.
/// Rubi returns its best-effort result and does not add an integration
/// constant. Construct inputs and render the returned bytes with Symbolica or
/// another plugin that implements the shared Atom-payload protocol.
///
/// -> bytes
#let integrate(
  /// Integrand encoded as a portable Atom payload.
  /// -> bytes
  expression,
  /// Integration variable encoded as a portable Atom payload.
  /// -> bytes
  variable,
) = {
  let expression = _payload(expression, "expression")
  let variable = _payload(variable, "variable")
  let rubi = _rubi-plugin()
  rubi.integrate(expression, variable)
}

/// Integrate a portable Atom payload and return Rubi's rule trace.
///
/// The result is `(result: bytes, complete: bool, steps: array)`. Every step
/// contains `rule`, `depth`, `description`, `references`, `source`, `input`,
/// and `output`; the two expressions are portable Atom payload bytes.
///
/// -> dictionary
#let integrate-with-steps(
  /// Integrand encoded as a portable Atom payload.
  /// -> bytes
  expression,
  /// Integration variable encoded as a portable Atom payload.
  /// -> bytes
  variable,
) = {
  let expression = _payload(expression, "expression")
  let variable = _payload(variable, "variable")
  let rubi = _rubi-plugin()
  cbor(rubi.integrate_with_steps(expression, variable))
}
