# Symbolica: constructing the algebraic context of sqrt(2) stalls

This standalone Rust program reproduces the problem without Tymbolica, Typst,
Rubi, or a Wasm runtime. It runs on native x86_64 Linux, selecting Symbolica's
`wasm` feature to use the same numeric backends as Tymbolica:
`integer-malachite` and `float-astro`.

## Reproduce

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
returns successfully and prints `OK`.

Observed: the program prints

```text
Parsed sqrt(2); constructing its algebraic context...
```

but does not reach `OK` before the timeout (exit status 124). It remains
CPU-bound. Parsing has already finished when the marker is printed.

## Pinned dependencies

- Symbolica, Numerica, and Graphica: official Symbolica `main` at
  `ba3737137c2a2ccd7bb39f0441837d38ec867e78`.
- Symbolica: `default-features = false`, `features = ["wasm"]`.
- Astro-float: Tymbolica's existing patch at
  `f92380e025deb8e1743ed93c6d5bf0783ac28717` (`astro-float` 0.9.5,
  `astro-float-num` 0.3.6).

Keep the included lockfile and use `--locked`: a fresh resolution can select
a newer registry version of `astro-float-num` instead of the patch. A separate
check with registry `astro-float` 0.9.6 and `astro-float-num` 0.3.7 still
reproduces the hang.
The native GMP/MPFR backend completes successfully.

## Confirmed cause

The Astro backend produces negative zero during the root-inclusion radius
calculation. Symbolica's `root_inclusion_disk` rejects that valid zero radius
because `radius.is_negative()` tests its sign, causing root isolation to keep
increasing precision. Correcting that zero's sign in the debugger lets the
calculation finish. This reproducer leaves the upstream code unchanged.

The same configuration also stalls when solving `x - 2^(1/2) = 0`, over either
the reals or the complexes. The solver reaches this path through
`solve_unrestricted` -> `algebraically_zero` -> `AlgebraicContext::from_atom`.
By contrast, solving the univariate polynomial `y^2 - 2 = 0` completes in
milliseconds. Tymbolica's previous bundle, pinned to Symbolica `ec19eeb`,
solves the linear case successfully; this standalone context API has not been
compared against that older revision.

The source is licensed under MIT, like Tymbolica.
