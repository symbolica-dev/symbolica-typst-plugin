# Changelog

## Unreleased

### Breaking changes

- Rename the public API, including the fields returned by `init()`:
  `math` → `parse`, `symbol` → `literal`, `function` → `function-head`,
  `terms` → `summands`, `content` → `matrix-content`, `vec` → `vector`,
  `det` → `determinant`, `cancel` → `cancel-factors`, `sub` → `subtract`,
  `div` → `divide`, `gamma` → `gamma-function`, and `zeta` → `zeta-function`.
  The old names have no aliases. Wildcard imports leave Typst's native names
  available.
- Remove `atom` and `init().atom`. Use `parse` for Typst math, Symbolica
  expression strings, numbers, or existing expression bytes. Strings require
  explicit multiplication, for example `parse("2*x + y")`. Custom `grammar`
  applies only to Typst content; `namespace` does not rename existing payloads.
- Reject trailing underscores in literal names. Wildcard names must be
  nonempty and must not end with an underscore; `wild` requires `level >= 1`.
- Update Symbolica to 3.0.1, using Atom export format 6. Stored expression bytes
  from Symbolica 3.0.0 (format 5) must be regenerated. Typst documents recreate
  their expressions when compiled.

### Added and improved

- Add `inspect` and `from-tree` for reading and rebuilding semantic expression
  trees, with matching Rust types and a versioned CBOR contract. Preserve exact
  coefficients, decorated symbols, and annotations from other plugins.
- Allow `literal` to use supported Typst math or text as the label for one
  symbol. An optional `name` lets distinct symbols share a label; composite
  labels are grouped when rendered.
- Preserve subscripts, corner attachments, and primes through parsing and
  rendering. Ordered labels stay distinct: `C_(i j)` differs from `C_(j i)`.
  Decorations remain literal notation; primes do not request differentiation,
  and superscripts in ordinary expressions still represent powers.
- Expose `gamma-function`, `polygamma`, `polylog`, `zeta-function`, and
  `bessel-j/y/i/k` directly and through `init()` engines.
- Add `logo()` for an inline Symbolica logo, with an optional `size` argument.
- Reduce integration-rule initialization time while retaining the complete
  rule set and step explanations. The runtime archive remains below 8 MB
  compressed. Load the Wasm directly, without the separate decompression plugin.
- Revise the manual and add generated equation previews to the Universe README.

### Fixed

- Keep `to-typst` output in one inline equation, including fractional products
  such as `parse($1/3 x^2$)`.
- Match Symbolica's fraction and rational-power layouts while preserving custom
  notation and expression metadata inside fractions and roots.
- Preserve decorated expressions when parsing rendered content back into math,
  including equation wrappers and explicit spacing.
- Avoid false metadata conflicts when expressions with nested labels are
  exchanged between native and Wasm runtimes.
- Report unsupported label content instead of silently treating it as text.

### Rust library

- Combine the plugin and shared payload library in `symbolica-typst-plugin`.
  Other plugins can use `payload`, `math_display`, and `typst_ast` without the
  standalone plugin's entry points or integration engine. Use the default
  `native` feature for native builds, or disable defaults and enable `wasm`
  for Wasm consumers. Enable `plugin` only for the standalone plugin.
- Use Symbolica 3.0.1 from crates.io; consumers no longer need a Git override.

## 0.1.0

Initial release on Typst Universe.

- Symbolic algebra, rational-expression transformations, differentiation,
  series, exact and numerical solving, numerical evaluation, and matrices.
- Symbolic integration with Rubi, including nested integration steps.
- Typst math parsing through Parsely, wildcard rewriting, and annotated
  expressions that can be exchanged with compatible plugins.
- A user manual, API reference, and worked examples, including batched
  evaluation, differential equations, and complex phase portraits.
- Free use within Typst, including commercial use, under the Symbolica Typst
  permission.
