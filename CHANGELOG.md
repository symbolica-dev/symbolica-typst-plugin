# Changelog

All notable user-visible changes are recorded here. Tymbolica currently uses a
local `0.1.0` package while its initial public surface is being prepared.

## Unreleased

- Reorganized the manual around installation, concepts, task-oriented worked
  guides, result contracts, troubleshooting, and a grouped API reference.
- Added a pendulum-calibration workflow, exact and numerical nonlinear solving,
  verified polynomial integration, wildcard rewriting, exact interpolation,
  and batched gradient evaluation examples.
- Changed `integrate-with-steps` to return one antiderivative contribution per
  canonically expanded polynomial term. The aggregate `result` remains the
  exact sum of those contributions.
- Documented every public parameter, default, return shape, and current
  capability boundary in the generated reference.
- Declared Tymbolica's original source code under the MIT License while keeping
  Symbolica's separate upstream terms explicit.
- Added release checks for the root package manifest and documented `@local`
  import layout.
- Reworked the manual's prose around mathematical questions, removed repeated
  implementation detail, and moved tool credits to acknowledgements.

## 0.1.0

- Initial local Typst package and WebAssembly bridge for Symbolica expressions,
  rewriting, exact and numerical solving, evaluation, and matrices.
