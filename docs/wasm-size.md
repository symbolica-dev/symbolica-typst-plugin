# Wasm size investigation

Measurements from 2026-09-15, starting at commit `c7d6848`, with Symbolica
3.0.0, symbolica-integrate 2.0.1, Binaryen 130, Wizer 11.0.3, and Typst
0.15.1. MiB means 1,048,576 bytes. Compressed sizes use this repository's
`symbolica-typst-compress` tool (miniz_oxide, level 10), not an estimated ratio.
New Rust builds use Cargo's default release codegen-unit count, not one unit.
The tables below record successive experiments; the current runtime
initialization design supersedes the initial recommendation to retain Wizer.

## Measured results

| Engine / experiment | Raw Wasm bytes | Compressed bytes | Result |
| --- | ---: | ---: | --- |
| Core, original | 11,068,132 | 4,161,063 | Baseline |
| Core, remove unrelated C API exports | 8,178,996 | 3,080,353 | Adopted: 25.97% smaller compressed |
| Integration, original preinitialized bundle | 37,851,832 | 8,193,846 | Baseline; two public function exports |
| Integration, optimize again after Wizer | 37,504,691 | 8,498,836 | Rejected: compressed size increases |
| Same post-Wizer module, remove `integrate_with_steps` export | 37,489,576 | 8,493,931 | Only 15,115 raw bytes / 4,905 compressed bytes saved |
| Integration, refreshed default-16-unit preinitialized build | 38,038,007 | 8,065,811 | Previous distributed bundle |
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
fixture also passed (11.31 seconds in a separate run). This initially favored
Wizer for startup speed; the later live-edit experiment supports moving this
cost to cached runtime initialization instead.

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
   The [Typst plugin transition](https://typst.app/docs/reference/foundations/plugin/#definitions-transition)
   API now provides the runtime preparation step. It still pays initialization
   on first use, but the initialized snapshot is reused during live editing
   and supplied to additional plugin instances.
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
raw engine is still 37.48 MiB. This exceeds our current build check; it does
not establish that Typst requires the custom compression stage (see the
archive experiment below). Its code section is 23,459,266 bytes and its data
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

### Removing the custom compression wrapper

Universe already creates `.tar.gz` downloads using `flate2::GzEncoder` with
default compression in its [package bundler](https://github.com/typst/packages/blob/main/bundler/src/main.rs).
The [Typst CLI package loader](https://github.com/typst/typst/blob/v0.15.1/crates/typst-kit/src/packages.rs)
extracts those archives into the package cache. Archive compression reduces
download size; it does not leave the installed files compressed.

A second scratch package replaces the compressed engine and inflater with
the raw combined Wasm, removes `_decompress-bundled`, and directly calls
`plugin("symbolica.wasm")`. Both variants below include the manifest, Typst
runtime sources, README, LICENSE, and THIRD_PARTY notice; manuals, examples,
and tests are omitted from the archives.

| Combined package | Installed file bytes | Gzipped tar bytes |
| --- | ---: | ---: |
| With custom compression and inflater | 8,628,891 (8.23 MiB) | 8,501,396 (8.11 MiB) |
| Raw Wasm, direct loading | 39,403,503 (37.58 MiB) | 8,636,778 (8.24 MiB) |

These are measured local archives, using deterministic GNU tar headers and
flate2 1.1.4's default Rust backend and compression level, matching Universe's
compression approach. Exact published bytes can vary with archive metadata,
file selection, and compressor version. The previous minimal-runtime figures
omit the three informational files included here.

Removing the custom layer increases the download by only **135,382 bytes
(132.21 KiB, 1.59%)**. All 12 non-manual Typst fixtures pass with direct loading,
including both integration endpoints. No Rust engine rebuild was necessary:
the decompression helper is a separate module. The upstream compressed rule
metadata feature remains enabled; this experiment removes the outer bundle
compression only.

The earlier conclusion that the large raw engine requires custom compression
was too strong. Direct loading is viable in the tested CLI, with almost the
same compressed download size and a larger installed footprint. Our 10 MiB
asset check is a repository build constraint, not a demonstrated Typst runtime
limit. Universe's acceptance of the larger individual file is a separate
publication question; its [package-size policy discussion](https://github.com/typst/packages/issues/4175)
does not establish a universal 10 MiB limit. Production loading is unchanged.

## Adopted: cached runtime initialization

The integration dependency now enables `compressed-step-metadata` directly,
including when Wizer is absent. Rechecking the earlier Wizer prototype with
this direct declaration produced byte-for-byte identical raw and compressed
binaries: that feature was already enabled through `wizer-preinitialize`.

Wizer has now been removed from the integration build and development tools.
A new `initialize` Wasm endpoint warms the rules through the upstream public
integration API. The Typst wrapper invokes it through
`plugin.transition(module.initialize)`. Typst caches the resulting module and
keeps a memory snapshot, which it copies into additional plugin instances.
This avoids independently constructing all rules for each execution worker.
The public Typst API remains `integrate` and `integrate-with-steps`.

The distributed integration engine is now **23,232,072 raw bytes** and
**5,940,523 compressed bytes (5.67 MiB)**, a **26.35%** compressed reduction
from the previous 8,065,811-byte Wizer bundle. Its only function exports are
`initialize`, `integrate`, and `integrate_with_steps`.

The [Typst 0.15.1 implementation](https://github.com/typst/typst/blob/v0.15.1/crates/typst-library/src/foundations/plugin.rs)
memoizes module loading, transitions, and function calls, and maintains a pool
of reusable instances. Rules are prepared on first integration use while the
cached module is alive. Ordinary edits reuse it. Restarting the compiler,
changing the Wasm, or evicting the module cache requires preparation again;
this is not persistent installation-time initialization. The transition API
also exists in the declared minimum Typst version, 0.14.0.

Typst's transition currently snapshots linear memory, not Wasm globals.
The optimized combined module has one mutable global, its stack pointer,
which returns to its initial value on a successful initializer return.
The rule heap and Rust initialization flags reside in the snapshotted memory.

The combined scratch library defers its transition until integration is used:
core calls use the base module, while integration calls use the cached derived
module. Core-only use therefore does not construct the Rubi rule tables.

| Combined engine | Raw Wasm bytes | Custom-compressed bytes |
| --- | ---: | ---: |
| Earlier Wizer build | 39,300,520 | 8,496,528 |
| Runtime transition | 24,488,612 | 6,376,501 |

The combined compressed engine shrinks **24.95%**. All 62 user endpoints remain,
plus the initialization endpoint. Using the same runtime file selection and
archive method as the preceding experiment:

| Combined runtime package | Installed file bytes | Gzipped tar bytes |
| --- | ---: | ---: |
| With custom compression and inflater | 6,509,123 (6.21 MiB) | 6,386,054 (6.09 MiB) |
| Raw Wasm, direct loading | 24,591,854 (23.45 MiB) | 6,436,801 (6.14 MiB) |

The combined engine remains a scratch experiment; the distributed packages
remain separate. The production integration package now uses the same runtime
transition approach. Release builds use 16 codegen units.

Live editing was tested in one persistent `typst watch` process per variant:
ten successive text edits followed by three different integrands. The latter
changed `x/(x+1)` to `x/(x+2)`, `x/(x+3)`, and `x/(x+4)`.

| Runtime initialization variant | First compilation | Ten text edits | Three integrand edits |
| --- | ---: | ---: | ---: |
| Distributed integration package | 13.351 s | 0.125–0.126 s each | 2.133, 2.108, 2.108 s |
| Combined engine, direct Wasm loading | 11.121 s | 0.126 s each | 2.133, 2.158, 2.233 s |

These are single-session wall-clock observations, including file watching,
compilation, and PDF export. They confirm that the initial preparation delay
does not recur on ordinary saves or changes to the integrand in these tests.
They are not isolated integration throughput benchmarks.

Validation of this change: five Rust integration tests, all 14 production
Typst distribution compilations and both manual freshness checks, all 12
non-manual combined-prototype fixtures, and direct-loading core API and
integration regression fixtures pass. The updated integration manual was
rendered and visually checked on all three pages.

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

The integration build now uses `--release --target wasm32-unknown-unknown
--no-default-features`, applies export pruning, and compresses the output
without Wizer. Its upstream dependency enables `compressed-step-metadata`
unconditionally, which also enables `steps`. The Typst wrapper calls
`plugin.transition(module.initialize)` before using either integration
endpoint.

General background: [Rust/Wasm code-size guidance](https://rustwasm.github.io/book/reference/code-size.html)
and [LLVM Wasm linker garbage collection](https://lld.llvm.org/WebAssembly.html#garbage-collection).
