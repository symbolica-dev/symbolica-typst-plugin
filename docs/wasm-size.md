# Wasm size investigation

Measurements from 2026-09-15, starting at commit `c7d6848`, with Symbolica
3.0.0, symbolica-integrate 2.0.1, Binaryen 130, Wizer 11.0.3, and Typst
0.15.1. MiB means 1,048,576 bytes. Compressed sizes use this repository's
`symbolica-typst-compress` tool (miniz_oxide, level 10), not an estimated ratio.
New Rust builds use Cargo's default release codegen-unit count, not one unit.

## Measured results

| Engine / experiment | Raw Wasm bytes | Compressed bytes | Result |
| --- | ---: | ---: | --- |
| Core, original | 11,068,132 | 4,161,063 | Baseline |
| Core, remove unrelated C API exports | 8,178,996 | 3,080,353 | Adopted: 25.97% smaller compressed |
| Integration, original preinitialized bundle | 37,851,832 | 8,193,846 | Baseline; two public function exports |
| Integration, optimize again after Wizer | 37,504,691 | 8,498,836 | Rejected: compressed size increases |
| Same post-Wizer module, remove `integrate_with_steps` export | 37,489,576 | 8,493,931 | Only 15,115 raw bytes / 4,905 compressed bytes saved |
| Integration, refreshed default-16-unit preinitialized build | 38,038,007 | 8,065,811 | Current distributed bundle |
| Integration, runtime initialization prototype | 23,230,463 | 5,940,748 | Works after fixing initialization order; slower startup |

The first four optimization experiments operate on existing binaries to avoid
confounding the result with compiler changes. The runtime-initialization
prototype required a fresh Rust build, so its size comparison also includes
the switch from the baseline's one codegen unit to Cargo's default 16.
The one-endpoint experiment is compared with the post-Wizer row directly above
it, not with the original baseline.

A controlled comparison used the same core loader, the same integration
example (which exercises both endpoints), and fresh Typst processes. Both
integration variants used newly built Rust code with the default 16 codegen
units. Three alternating runs gave these document-compilation times:

| Initialization | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Wizer | 4.82 s | 4.79 s | 4.67 s | 4.79 s |
| Runtime | 11.79 s | 11.63 s | 11.28 s | 11.63 s |

Runtime initialization saves 26.35% of the current compressed integration
bundle, but this small document compiles about 2.4 times slower. These are
local measurements, not a general performance guarantee. The cold regression
fixture also passed (11.31 seconds in a separate run). Keep preinitialization
as the default.

## Core: compression is now optional for fitting under the limit

Symbolica's C API module is compiled unconditionally in `src/api.rs`. Its
`no_mangle` exports become linker roots even though the Typst wrapper never
calls them. The exported C API includes `init`, `drop`, `set_vars`, `simplify`,
and `simplify_factorized`.

The core build now removes these unrelated exports before Binaryen's dead-code
elimination and size optimization. All 60 Typst endpoints remain, including
our own `simplify_expr`. The integration build already removed these exports.
An upstream feature gate around `symbolica::api::cpp` would prevent this
unnecessary retention at link time and remove the need for a downstream list.

The optimized core is about 7.80 MiB raw, below the repository's 10 MiB plugin
asset limit. Direct loading without the inflater was tested successfully.
This is a viable way to remove the compression helper from the core package.
However, it makes the shipped core file larger than the 2.94 MiB compressed
version. The normal package therefore keeps compression while benefiting
from the code reduction.

## Why two integration endpoints still produce a large module

The distributed integration module already exports only `integrate` and
`integrate_with_steps`, plus its memory. Fewer endpoint wrappers help only when
they make substantial implementation code unreachable.

The baseline's Wasm code section is 22,131,612 bytes. Its data section after
Wizer is 15,656,218 bytes, versus 931,908 bytes before Wizer. Wizer trades a
larger initial memory image for avoiding runtime construction of the rules.
This behavior is also described in the [Wizer documentation](https://github.com/bytecodealliance/wizer#usage).

In the upstream engine, `rubi_rules()` constructs and sorts the complete rule
collection. Each `RubiRule` retains a pattern/replacement, condition and action
function pointers, and many vectors of symbols and matching constraints.
The complete collection keeps the generated code reachable through those
function pointers. This remains true when only `integrate` is exported.

The compressed source/description catalog itself is 200,931 bytes, about
196 KiB. It is not the main source of the tens of megabytes. Removing the
steps *export* is also different from building the upstream crate without its
`steps` feature: tracing hooks and metadata can remain reachable in the first
case. A no-steps build is a separate, unmeasured capability tradeoff.

## Other approaches worth pursuing

1. **Compact rule storage and shared rule execution.** Move repeated pattern,
   guard, and action construction into compact tables or a rule bytecode
   interpreted by shared routines. This targets both the many generated
   functions and the large initialized heap. It is the most promising route
   to a full integration engine small enough to ship uncompressed, but needs
   upstream design work and measurements. Savings are not yet quantified.
2. **Avoid shipping the expanded heap.** The runtime-initialization prototype
   is about 5.67 MiB compressed. Its first attempt panicked because Rubi's
   function catalog was entered before Symbolica finished the registry
   callback that also initializes that catalog. Initializing Symbolica first
   fixes this, and the integration example and derivative regression pass.
   An explicit [Typst plugin transition](https://typst.app/docs/reference/foundations/plugin/#definitions-transition)
   could provide a deterministic runtime preparation step. This still pays
   initialization during document compilation and requires further testing;
   it does not make the current 22.15 MiB raw module small enough by itself.
3. **Compile-time optional rule families.** An explicitly limited algebraic or
   elementary integration variant could exclude whole families of rules and
   their helpers. This reduces mathematical coverage and should be opt-in.
   Simply splitting full engines into several plugins can duplicate Symbolica
   and increase total package size; splitting alone is not a size solution.
4. **Measure compile-time tracing removal and alternate size settings.** Runtime
   logging is disabled already, but that differs from compiling tracing code
   out. `wasm`, `binary_size`, LTO, `opt-level = "z"`, panic abort, stripping,
   and Binaryen optimization are already enabled. Further compiler tuning is
   secondary to removing reachable code and compacting the rule representation.

## Combined core and integration package

A follow-up experiment at `7cf8569` links the complete core API and integration
bridge into **one Wasm module**, loaded by one Typst package. It retains all
60 core endpoints, both integration endpoints, rule explanations, and Wizer
preinitialization. Merely putting the existing two binaries in one package
would not achieve this saving.

| Distribution | Optimized raw engine bytes | Compressed engine bytes | Engines plus inflater(s) |
| --- | ---: | ---: | ---: |
| Current core | 8,178,996 | 3,080,353 | 3,109,597 (2.97 MiB) |
| Current integration | 38,038,007 | 8,065,811 | 8,095,055 (7.72 MiB) |
| Current pair, total | 46,217,003 | 11,146,164 | 11,204,652 (10.69 MiB) |
| Combined, preinitialized | 39,300,520 | 8,496,528 | 8,525,772 (8.13 MiB) |

The combined runtime assets save **2,678,880 bytes (23.91%)** versus shipping
both packages. Each inflater is 29,244 bytes. Adding all core endpoints to the
integration engine costs only **430,717 compressed bytes (420.62 KiB)**,
consistent with substantial overlap in their reachable implementations.

Including `lib.typ`, `render.typ`, and the package manifest gives a minimal
combined runtime package of **8,618,769 bytes (8.22 MiB)**. The scratch tree with
both existing manuals, examples, and test fixtures is **9,853,046 bytes
(9.40 MiB)**. These are sums of file sizes, not tarball sizes; the final
published layout and documentation would determine the actual package total.

This is a useful reduction for users needing integration. Core-only users
would instead download 8.13 MiB of runtime assets instead of 2.97 MiB. The
combined compressed engine fits the repository's 10 MiB asset check, but its
raw engine is still 37.48 MiB, so merging does **not** remove the need for the
current compression stage. Its code section is 23,459,266 bytes and its data
section after Wizer is 15,772,486 bytes. Integration's rule implementation and
initialized heap remain the dominant costs.

Three alternating runs in fresh Typst processes, with compressed loading in
both variants, gave these document-compilation times:

| Document | Current split: runs / median | Combined: runs / median |
| --- | --- | --- |
| Core basic example | 1.002, 0.982, 0.984 s / **0.984 s** | 2.718, 2.715, 2.827 s / **2.718 s** |
| Integration example, both endpoints | 5.484, 5.439, 5.521 s / **5.484 s** | 5.012, 4.780, 4.944 s / **4.944 s** |

The combined integration example is about 10% faster in this small local
sample, while the core-only example is about 2.8 times slower. These are
whole-document timings, including loading, decompression, and execution;
they do not isolate algebra or integration throughput. A single full-featured
package is attractive when integration is part of the default offering, with
a clear download and startup cost for core-only use.

### Prototype method

The experiment uses Rust 1.98.1 and the same dependency versions and release
profile as the repository: LTO, `opt-level = "z"`, panic abort, stripping,
and Cargo's default **16 codegen units**. The comparison above is against the
currently committed bundles, not a rebuild of the old one-unit core baseline.

1. Create a standalone scratch Cargo workspace with the root crate's
   dependencies plus `symbolica-integrate` 2.0.1 and its `steps` feature.
   Copy the lockfile and use the repository's `.cargo/config.toml` settings.
2. Copy `src/lib.rs` as the scratch crate root and add the integration bridge
   from `crates/rubi-plugin/src/lib.rs` as a submodule. Remove its duplicate
   custom random backend. Replace its `initiate_protocol!()` with imports of
   the root's `__BytesOrResultBytes`, `__send_result_to_host`, and
   `__write_args_to_buffer` helpers.
3. Keep the integration crate's `compressed-step-metadata` and
   `wizer-preinitialize` feature definitions, then build the combined cdylib
   with `--release --target wasm32-unknown-unknown --no-default-features
   --features wizer-preinitialize`.
4. Apply the core build's exact export-pruning and Binaryen options. Keep
   `simplify_expr`: the integration-only `simplify*` removal would incorrectly
   delete this core endpoint. Run Wizer with `--init-func wizer.initialize`,
   then `symbolica-typst-compress` on its output.
5. In a scratch copy of the Typst package, append the integration wrappers to
   `symbolica/lib.typ`, have both call `_bundled_plugin()`, and replace the
   core bundle with the combined output. Remove the second engine and loader.
   Adapt integration fixtures to use the same library and `@local/symbolica`
   package import.

The final Wasm export set exactly matches the union of the two sources:
62 endpoint functions plus memory. All 12 non-manual Typst distribution
fixtures pass, including the core API surface, metadata, worked examples,
both local package imports, integration rule explanations, and differentiating
the primitive back to the integrand. Production packages and binaries are
unchanged by this experiment.

## Validation and reproduction

- Five Rust integration bridge tests pass after the initialization-order fix.
- All 14 Typst distribution compilations and both manual freshness comparisons
  pass with the pruned core and rebuilt preinitialized integration bundle.
- The optimized core export list matches every `#[wasm_func]` in `src/lib.rs`.
- Direct loading of the pruned core works without the decompression helper.
- The runtime-initialization prototype passes the integration example and the
  regression that differentiates its result back to the original integrand.

`nix run .#build-engine` applies the adopted core export pruning. The
`engineBuildScript` and `rubiBuildScript` in `flake.nix` contain the exact
Binaryen options used by the normal builds. This environment could not use
`/nix/store`, so the same shell steps were executed directly.

To investigate runtime initialization, build `symbolica-typst-integrate-plugin`
with `--release --target wasm32-unknown-unknown --no-default-features --features
compressed-step-metadata`, apply the integration build's export pruning, and
compress that output without running Wizer. Run tests in a separate package
copy. This prototype is not the default distributed integration bundle.

General background: [Rust/Wasm code-size guidance](https://rustwasm.github.io/book/reference/code-size.html)
and [LLVM Wasm linker garbage collection](https://lld.llvm.org/WebAssembly.html#garbage-collection).
