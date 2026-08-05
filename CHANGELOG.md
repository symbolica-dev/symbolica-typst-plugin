# Changelog

All notable user-visible changes are recorded here. Tymbolica currently uses a
local `0.1.0` package while its initial public surface is being prepared.

## Unreleased

- Reorganized the manual around installation, concepts, task-oriented worked
  guides, troubleshooting, and a grouped API reference.
- Added a pendulum-calibration workflow, exact and numerical nonlinear solving,
  verified polynomial integration, wildcard rewriting, exact interpolation,
  and batched gradient evaluation examples.
- Replaced polynomial-only integration with the MIT-licensed
  `symbolica-integrate` Rubi engine. `integrate-with-steps` now returns Rubi's
  actual nested transformations, including rule metadata, input and output
  expressions, and whether the best-effort result is complete.
- Split the WebAssembly engine into a compact core plugin and a full plugin
  containing Rubi. The imported top-level API and plain `init()` use core;
  integration requires `init(profile: "full")`, with parsing, transformation,
  and rendering kept within that full API because plugin expression bytes are
  not portable. Both plugins remain part of the package, so the split reduces
  loaded and instantiated WebAssembly rather than download size.
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
