# Changelog

All notable user-visible changes are recorded here. The Symbolica Typst plugin
currently uses a local `0.1.0` package while its initial public surface is being
prepared.

## Unreleased

- Updated core and integration to the Symbolica 3.0.0 release from crates.io.
- Renamed the official Typst packages to `symbolica` and `symbolica-integrate`,
  and the Rust crates to `symbolica-typst-plugin`,
  `symbolica-typst-integrate-plugin`, `symbolica-typst-inflate-plugin`, and
  `symbolica-typst-atom-payload`. Update package imports and Rust dependencies
  to the new names. Payloads now use the `symbolica` protocol identifier and
  `SYMATOM` envelope prefix.
- The official plugins are free to use for any use within Typst, including
  academic and commercial work. No Symbolica license or license key is needed.
- Includes the upstream fix for the exact algebraic solver hang present in
  `ba373713`, including `x - sqrt(2) = 0` with the Wasm numeric backend. The
  standalone regression check and historical diagnosis remain in
  `repros/algebraic-sqrt2`.
- Breaking: `solve` now returns a solution-set dictionary. Read solutions from
  `.branches`; `coverage` and `coverage-guard` preserve the limits of results with
  symbolic parameters. Branches expose `point` and optional `codimension` instead
  of `rank`, `parametric`, and `indeterminate`; unknown dimensions remain `none`.
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
- Added the separate `symbolica-integrate` package around the MIT-licensed
  `symbolica-integrate` engine. Its focused plugin exposes only `integrate` and
  `integrate-with-steps`; the latter returns Rubi's genuine nested
  transformations, metadata, expressions, and completion status.
- Preinitialize Rubi's immutable rule tables with Wizer at build time. The
  lightweight Symbolica core and focused Rubi extension are stored as
  independent DEFLATE-compressed assets below the Typst web app's 10 MiB limit.
- Consolidated the Parsely-to-Atom bridge, native Atom export, portable
  attachments, and recursive render tree in the reusable
  `symbolica-typst-atom-payload` crate. Symbolica and Rubi preserve unknown attachment
  schemas without linking their owners.
- Moved Tydenso's plugin, Typst package, manual, examples, and tensor-specific
  attachment codecs into GammaLoop, where Spenso, Idenso, and Spynso are
  maintained. GammaLoop consumes the shared payload crate as a pinned Git
  dependency; Symbolica has no reverse dependency on GammaLoop.
- Added recursive CBOR inspection of Symbolica Atom internals while retaining
  the native Atom export as the lossless cross-plugin payload.
- Documented every public parameter, default, return shape, and current
  capability boundary in the generated reference.
- Declared the original plugin source code under the MIT License while keeping
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
