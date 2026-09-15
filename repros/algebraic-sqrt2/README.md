# Symbolica: algebraic sqrt(2) regression check

This standalone Rust program checks a former hang without Tymbolica, Typst,
Rubi, or a Wasm runtime. Symbolica `4f132318` fixes the regression present in
`ba373713`. The program runs on native x86_64 Linux, selecting Symbolica's
`wasm` feature to use Tymbolica's numeric backends: `integer-malachite` and
`float-astro`.

## Run the check

With a recent stable Rust toolchain, from this directory:

```sh
cargo build --locked
timeout 15s ./target/debug/symbolica-algebraic-sqrt2-mre
echo $?
```

Build before applying the timeout, so compilation time is not counted. If
`CARGO_TARGET_DIR` is set, adjust the binary path accordingly. `timeout` is the
GNU coreutils command; without it, run the binary and interrupt it manually.

Expected: constructing the algebraic context of the exact number `sqrt(2)`
returns successfully, prints `OK`, and exits with status 0.

## Pinned dependencies

- Symbolica, Numerica, and Graphica: version `3.0.0` from crates.io.
- Symbolica: `default-features = false`, `features = ["wasm"]`.
- Astro-float: Tymbolica's existing patch at
  `f92380e025deb8e1743ed93c6d5bf0783ac28717` (`astro-float` 0.9.5,
  `astro-float-num` 0.3.6).

Keep the included lockfile and use `--locked`: a fresh resolution can select
a newer registry version of `astro-float-num` instead of the patch.

## Historical regression

With Symbolica `ba3737137c2a2ccd7bb39f0441837d38ec867e78`, the program printed

```text
Parsed sqrt(2); constructing its algebraic context...
```

but did not reach `OK` before the timeout (exit status 124). It remained
CPU-bound after parsing had finished. The
[original reproducer](https://github.com/lcnbr/tymbolica/tree/f8b08dbe/repros/algebraic-sqrt2)
retains that dependency configuration.

The Astro backend produced negative zero during the root-inclusion radius
calculation. Symbolica's `root_inclusion_disk` rejected that valid zero radius
because `radius.is_negative()` tested its sign, causing root isolation to keep
increasing precision. Correcting that zero's sign in the debugger let the
calculation finish. No local arithmetic patch was applied.

The same configuration also stalled when solving `x - 2^(1/2) = 0`, over either
the reals or the complexes. The solver reached this path through
`solve_unrestricted` -> `algebraically_zero` -> `AlgebraicContext::from_atom`.
By contrast, solving the univariate polynomial `y^2 - 2 = 0` completed in
milliseconds, as did the native GMP/MPFR backend. Registry `astro-float` 0.9.6
and `astro-float-num` 0.3.7 still reproduced the hang on `ba373713`.

With `4f132318` and those same registry Astro versions, the algebraic context,
the linear solve, and both branches of the coupled system `x - y = 0`,
`y^2 - 2 = 0` complete in milliseconds. Both real and complex solves were
checked, without numerical substitution or a local solver workaround.

The source is licensed under MIT, like Tymbolica.
