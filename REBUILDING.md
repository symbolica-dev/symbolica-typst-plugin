# Source locations and rebuilding

The plugin source, `Cargo.lock`, and build scripts are available in the
[upstream repository](https://github.com/symbolica-dev/symbolica-typst-plugin).
Version 0.1.0 is published on Typst Universe. Future releases use a matching
`v<version>` tag and include `SOURCE.json` with the exact source revision and
hashes of the engine and dependency lockfiles. See the repository's
[release instructions](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/docs/releasing.md).

Dependencies are fetched from crates.io at the versions and checksums recorded
in `Cargo.lock`. The plugin, integration engine, and reusable payload modules
use Symbolica 3.0.1 from crates.io. [Third-party notices](THIRD_PARTY_LICENSES.txt)
provide exact source archive URLs for the Wasm build.

Consumers can use the same crates.io release without a Cargo patch. To test a
compatible Git revision or local checkout, add a `[patch.crates-io]` override
at the build root and update its lockfile. Cargo does not inherit patches from
dependencies. The payload implementation is part of `symbolica-typst-plugin`;
there is no separate payload crate.

## Build

Check out the source revision matching the binary. Install Rust 1.97.0, its
`wasm32-unknown-unknown` standard library, and Binaryen 130 (`wasm-opt`), then run:

```sh
bash scripts/build-engine.sh
```

The build uses `cargo rustc --locked`, so it retains the recorded dependency
versions. Update the lockfile explicitly when upgrading dependencies. Cargo
downloads the locked dependencies. The result is
`symbolica/symbolica.wasm`. The release profile uses the default 16 codegen
units, full LTO, and no Wizer preinitialization. Rust uses `opt-level = "s"`
except for the generated `symbolica-integrate` rule code, which stays at `"z"`.
Binaryen still applies `-Oz`. This mixed profile improves rule initialization
while keeping the measured runtime archive below 8,000,000 bytes (8 MB).
See [the optimization measurements](https://github.com/symbolica-dev/symbolica-typst-plugin/blob/main/docs/wasm-optimization.md).

Normal Cargo builds produce a reusable Rust library (`rlib`). Its default
`native` feature uses Symbolica's native GMP/MPFR backend. For Wasm, disable
defaults and select `wasm`. The independent `plugin` feature enables the Typst
entry points, integration engine, and plugin runtime setup; other plugins using
the shared library should leave it disabled.

The build script explicitly selects `cdylib` when producing the standalone
plugin, then optimizes that Wasm module:

```sh
cargo rustc --locked --release --target wasm32-unknown-unknown \
  --package symbolica-typst-plugin --lib --crate-type cdylib \
  --no-default-features --features wasm,plugin
```

Run the library and full native plugin tests with:

```sh
cargo test --locked
cargo test --locked --features plugin
```

Symbolica is resolved from crates.io; the plugin crate itself is not yet
published. The Cargo package includes Rust sources, tests, the README, and
license; generated Wasm and the manual PDF are distributed
with the Typst package separately.

To rebuild with a modified dependency whose license permits modification,
add a Cargo `[patch.crates-io]` path override, update the lockfile as needed,
and rebuild. For example:

```toml
[patch.crates-io]
malachite-base = { path = "modified/malachite-base" }
```

Symbolica's own source remains governed by `LICENSE-SYMBOLICA.md` and the
limited permission in `LICENSE-SYMBOLICA-TYPST.md`.

## Optional offline archive

Maintainers can run `python3 scripts/prepare-source.py` to generate
`dist/symbolica-0.2.0-source.tar.gz`, containing the plugin source and vendored
dependencies and Cargo source replacements needed to rebuild without fetching
dependencies. This is an optional convenience and availability backup, not a
required archive format. It is not included in the Typst runtime package.

Source access must remain available to binary recipients. Hosting sources on
crates.io or GitHub does not remove the distributor's responsibility to ensure
their availability. See `THIRD_PARTY.md` for the applicable licenses.
