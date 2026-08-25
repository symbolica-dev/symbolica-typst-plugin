# Changelog

All notable user-visible changes are recorded here. Tymbolica currently uses a
local `0.1.0` package while its initial public surface is being prepared.

## Unreleased

- Replaced the `var` alias with `symbol` and added a distinct callable
  `function` constructor. Both use one versioned metadata envelope containing
  authoritative native Atom bytes plus inspectable namespace and tag data;
  Parsely consumes the annotation before implicit multiplication, while
  unrelated package metadata remains transparent.
- Added independently adapted batched-evaluation, Lotka–Volterra, and complex
  phase-portrait examples inspired by TimeTravelPenguin's `symbolic-eval`.
- Reorganized the manual around installation, concepts, task-oriented worked
  guides, troubleshooting, and a grouped API reference.
- Added a pendulum-calibration workflow, exact and numerical nonlinear solving,
  verified polynomial integration, wildcard rewriting, exact interpolation,
  and batched gradient evaluation examples.
- Added the separate `tymbolica-rubi` package around the MIT-licensed
  `symbolica-integrate` engine. Its focused plugin exposes only `integrate` and
  `integrate-with-steps`; the latter returns Rubi's genuine nested
  transformations, metadata, expressions, and completion status.
- Preinitialize Rubi's immutable rule tables with Wizer at build time. The
  lightweight Tymbolica core, focused Rubi extension, and Tydenso are stored as
  independent DEFLATE-compressed assets below the Typst web app's 10 MiB limit.
- Split Tydenso into its own Typst package with a separate manual and examples.
  Its scope-free API constructs Spenso representations, slots, tensor names,
  symmetry attributes, and annotated math calls; the structural constructors
  remain available for generated expressions.
- Added one shared Parsely-to-Atom Rust bridge to both plugins. Tydenso's
  `math` accepts annotated tensor calls, and annotated values move between
  Tydenso and Tymbolica through their exact native Atom payload.
- Added portable attachments to the shared Atom payload. Tymbolica and Rubi
  preserve them opaquely without linking Spenso or Idenso; Tydenso alone uses
  them to restore custom representation classes and index palettes before
  importing an expression in another plugin runtime.
- Added Tydenso's own Spenso-aware Typst and compact printers, configurable
  through the real `SpensoPrintSettings` fields. Bra-ket delimiters and
  contraction dots use compact mathematical spacing, and nested chain factors
  no longer acquire redundant parentheses.
- Render tensor and vector indices with Typst `attach`, including Physica-style
  hidden alignment columns. Built-in base orientations use the upper row except
  for bispinors; dualizable representations retain an explicit lower
  orientation. Built-in metrics use the same Typst-specific notation instead
  of exposing their internal representation slots.
- Give every built-in Spenso and Idenso representation a conventional cyclic
  palette for integer slots. The palette is canonical representation metadata,
  survives plugin round trips, and wraps with numbered subscripts; explicitly
  named slots remain unchanged.
- Preserve structured Typst math in tensor and vector arguments as one portable
  display-defined symbol; wrapping the same content with `math` continues to
  create real Symbolica algebra. Semantic string identifiers such as `"mu"`
  use Symbolica's native quoted Typst output and remain visibly distinct from
  notation such as `$mu$`.
- Attach one authoritative Atom payload to evaluated `to-typst` output. Live
  Typst content can therefore be interpolated back into `math` without
  reverse-parsing Symbolica or Spenso's displayed notation; source strings,
  copied visual text, and the separate matrix payload remain display-only.
- Added recursive CBOR inspection of Symbolica Atom internals while retaining
  the native Atom export as the lossless cross-plugin payload.
- Documented every public parameter, default, return shape, and current
  capability boundary in the generated reference.
- Declared Tymbolica's original source code under the MIT License while keeping
  Symbolica's separate upstream terms explicit.
- Added release checks for the root package manifest and documented `@local`
  import layout.
- Reworked the manual's prose around mathematical questions, removed repeated
  implementation detail, and moved tool credits to acknowledgements.
- Added rational-expression transforms (`together`, `cancel`, and `apart`),
  collection and coefficient tools, term and indeterminate inspection,
  structural predicates, decimal approximation, and complex or square-free
  factorization options.
- Added exact matrix zero and diagonal predicates plus entrywise matrix
  differentiation.
- Preserve decimal literals as floating-point coefficients instead of silently
  rewriting them as rationals. Exact rational analytic inputs such as
  `cos(1/2)` remain approximable with `to-float`.

## 0.1.0

- Initial local Typst package and WebAssembly bridge for Symbolica expressions,
  rewriting, exact and numerical solving, evaluation, and matrices.
