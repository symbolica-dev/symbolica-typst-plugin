# Source locations and rebuilding

The plugin source, `Cargo.lock`, and build scripts are available in the
[upstream repository](https://github.com/symbolica-dev/symbolica-typst-plugin).
Each binary release must identify the matching source revision. The planned
0.1.0 release uses tag `v0.1.0`; publish that tag before distributing the release.

Dependencies are fetched from crates.io at the versions and checksums recorded
in `Cargo.lock`, or from Git at the recorded commit. Symbolica uses `main` at
[`06906976bca24fefc5203aee699d90d62ebe08cd`](https://github.com/symbolica-dev/symbolica/tree/06906976bca24fefc5203aee699d90d62ebe08cd).
The workspace dependency and crates.io patch keep the plugin, integration
engine, and shared Atom payload crate on this revision. [Third-party notices](THIRD_PARTY_LICENSES.txt) provide
exact source archive URLs and Git revisions for the Wasm build.

## Build

Check out the source revision matching the binary. Install Rust 1.98.1, its
`wasm32-unknown-unknown` standard library, and Binaryen 130 (`wasm-opt`), then run:

```sh
bash scripts/build-engine.sh
```

The build uses `cargo build --locked`, so it retains the recorded Git revision
even when `main` advances. Update the lockfile explicitly when upgrading
dependencies. Cargo downloads the locked dependencies. The result is
`symbolica/symbolica.wasm`. The release profile uses the default 16 codegen
units, full LTO, and no Wizer preinitialization. Rust uses `opt-level = "s"`
except for the generated `symbolica-integrate` rule code, which stays at `"z"`.
Binaryen still applies `-Oz`. This mixed profile improves rule initialization
while keeping the measured runtime archive below 8,000,000 bytes (8 MB).
See [the optimization measurements](docs/wasm-optimization.md).

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
`dist/symbolica-0.1.0-source.tar.gz`, containing the plugin source and vendored
dependencies, including the locked Git checkout and Cargo source replacements
needed to rebuild without fetching dependencies. This is an optional convenience
and availability backup, not a required archive format. It is not included in the Typst runtime package.

Source access must remain available to binary recipients. Hosting sources on
crates.io or GitHub does not remove the distributor's responsibility to ensure
their availability. See `THIRD_PARTY.md` for the applicable licenses.
