# Balanced Wasm optimization

Measured on 2026-09-21 with Rust 1.98.1, Binaryen 130, and Typst 0.15.1. MB is decimal (1,000,000 bytes).

## Selected profile

Use Rust `opt-level = "s"` with a package override of `"z"` for `symbolica-integrate`. The generated rule code remains compact while shared algebra and numeric operations receive less aggressive size optimization. Full LTO, 16 release codegen units, panic abort, stripping, export pruning, and Binaryen `-Oz` are unchanged. No rules, step explanations, or API features are removed. No Wizer preinitialization is used.

```toml
[profile.release]
lto = true
opt-level = "s"
panic = "abort"
strip = true

[profile.release.package.symbolica-integrate]
opt-level = "z"
```

The proposed runtime archive is **7,796,758 bytes (7.797 MB)**, below both the preferred 8 MB target and the 10 MB limit requested for this experiment. The raw Wasm is 28,231,762 bytes. Archive contents and gzip settings match `scripts/prepare-distribution.py`; the existing packaging guard is unchanged. The archive includes the proposed rebuilding documentation.

## Final paired benchmark

Seven fresh-process compilations per build and workload, alternating build order and shuffling workload order. One discarded warm-up per combination. Same AMD EPYC 9754 host, fixed CPU affinity 382, Typst --jobs 1, warm filesystem caches, embedded fonts only, creation timestamp 0. All Rust candidate builds had finished before these final measurements. These are fresh compiler processes, not incremental editor updates. The host is shared; medians and interquartile ranges are reported.

| Workload | Size build seconds | Selected seconds | Speedup | Size IQR | Selected IQR |
| --- | ---: | ---: | ---: | ---: | ---: |
| startup | 0.2058 | 0.2317 | 0.888x | 0.2052–0.2069 | 0.2313–0.2326 |
| integration-first | 8.3263 | 4.9051 | 1.697x | 8.2652–8.3845 | 4.9007–4.9355 |
| algebra-heavy | 3.2103 | 2.0284 | 1.583x | 3.2053–3.2193 | 2.0272–2.0460 |
| evaluate-batch | 0.9102 | 0.8518 | 1.069x | 0.9031–0.9128 | 0.8498–0.8550 |

Three separate profiled paired runs measured the integration initialization transition at **7.5294 → 4.3264 seconds**, a 42.5% reduction. This span includes preparing the Rubi rules, the existing warm-up integral, and creating the initialized plugin snapshot. It excludes initial Wasm loading and is reported separately from unprofiled wall-clock timing.

The first-integration workload loads the plugin, integrates `x/(x+1)`, verifies its derivative against the integrand, and prints the result. Algebra expands, factors, and verifies 100 distinct three-variable degree-8 polynomials. Batched evaluation computes two transcendental expressions at 20,000 distinct points. Inputs vary to avoid memoized calculation results. All paired runs produce byte-identical PDFs between builds, including numeric checksums and exact algebra/integration assertions.

## Candidate screening

Candidate wall times below are medians of three screening runs per build, run sequentially. Some candidate Rust compilations were still running elsewhere on the shared host. The final paired measurements above determine the selected profile; small differences in screening timings should not be overinterpreted. Candidate archive sizes use the original rebuilding documentation to hold runtime content constant.

| Profile | Runtime archive bytes | First integration seconds | Heavy algebra seconds |
| --- | ---: | ---: | ---: |
| Rust z, Binaryen -Oz | 6,494,095 | 8.3454 | 3.3378 |
| Rust z, Binaryen -O2 | 6,591,906 | 8.2344 | 3.1476 |
| Rust s, Binaryen -Oz | 9,698,862 | 4.6833 | 2.2316 |
| Rust z, Symbolica 2, Binaryen -Oz | 9,036,047 | 5.8568 | 2.4660 |
| Rust 2, integration z, Binaryen -Oz | 12,525,163 | Not benchmarked (over budget) | Not benchmarked (over budget) |
| Rust z, Symbolica s, Binaryen -Oz | 7,406,358 | 6.0357 | 2.3831 |
| Rust z, Symbolica 1, Binaryen -Oz | 7,865,716 | 5.9969 | 2.4108 |
| Rust s, integration z, Binaryen -Oz | 7,796,569 | 4.9023 | 2.0306 |

The earlier all-speed Rust 3 / Binaryen -O3 runtime archive was 17,511,466 bytes. A Binaryen-only change barely improved initialization. The selected mixed s/z profile stays below 8 MB and substantially improves initialization and algebra, while adding only a small plugin-startup cost.

## Validation and reproduction

Validation passed: the paired benchmarks produce identical outputs; all 64 Wasm exports match the shipped build; nix flake check passes on x86_64-linux, including four regression fixtures, eight examples, and manual freshness; an integration-with-steps example compiled from the extracted runtime archive also passes. The Nix check independently produced the same 7,796,758-byte runtime archive. Other host architectures were not executed. See validation.log and runtime-check/check.typ in the local study directory.

Rebuild with Rust 1.98.1, its wasm32-unknown-unknown standard library, Binaryen 130, and the profile above using `bash scripts/build-engine.sh`. Package with `python3 scripts/prepare-distribution.py` and check that the resulting archive remains below 8,000,000 bytes for the preferred target. The complete local experiment scripts, workload sources, raw JSON timings, traces, and artifacts are saved under `dist/wasm-balanced-study-2026-09-21`. The paired benchmark can be repeated there with `nix develop -c python3 paired.py core-s` after running from that directory.

Selected Wasm SHA-256: `8461f5a090dce0c02243e9ddba30657c7482ee977d2f1388be1b4be8bd46d1de`.
