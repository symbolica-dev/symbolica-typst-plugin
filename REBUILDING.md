# Corresponding source and rebuilding

The joint WebAssembly engine includes the LGPL-3.0-only Malachite libraries
and MPL-covered components. Their license texts and notices are included in
`THIRD_PARTY_LICENSES.txt`. You may inspect and modify these components and
rebuild the plugin with your changes. The accompanying Symbolica Typst
permission allows the application-side source and relinking needed for this
purpose; see `LICENSE-SYMBOLICA-TYPST.md`.

## Source availability

The plugin's application source, exact dependency versions in `Cargo.lock`,
and build scripts are developed at
https://github.com/symbolica-dev/symbolica-typst-plugin.
The source archive named `symbolica-0.1.0-source.tar.gz` contains those sources,
the source of all locked Cargo dependencies (including Symbolica and
Malachite), an offline Cargo configuration, and the required notices.

For the 0.1.0 release, that archive must accompany the release assets at
https://github.com/symbolica-dev/symbolica-typst-plugin/releases/tag/v0.1.0.
Before publication this is a release preparation requirement, not a claim
that the archive has already been uploaded. Maintainers must publish the
source archive before submitting or distributing the release's binary.
Do not put the source archive in the Typst Universe runtime package.

## Build or relink with modified dependencies

The distributed engine was built with Rust 1.98.1, the
`wasm32-unknown-unknown` standard library, and Binaryen 130 (`wasm-opt`). Install
these tools separately; they are not part of the source archive.

Extract `symbolica-0.1.0-source.tar.gz`, enter the extracted directory, and run:

```sh
CARGO_NET_OFFLINE=true bash scripts/build-engine.sh
```

The result is `symbolica/symbolica.wasm`. The release profile uses Cargo's
default 16 codegen units. There is no Wizer preinitialization stage.

To replace a library, copy the relevant directory out of `vendor/`, modify it,
and add a Cargo `[patch.crates-io]` path override. This avoids changing files
covered by Cargo's vendor checksums. For example:

```toml
[patch.crates-io]
malachite-base = { path = "modified/malachite-base" }
```

Update the lockfile offline if Cargo requests it, then rebuild with the command
above. Corresponding application code is supplied as source, so rebuilding
does not require extracting linkable objects from the optimized Wasm. You may
reverse engineer the combined plugin to debug modifications to LGPL-covered
components as permitted by their license and the Symbolica Typst permission.

## Prepare a release from the development repository

```sh
python3 scripts/update-licenses.py
python3 scripts/prepare-source.py
python3 scripts/prepare-distribution.py
```

The source and runtime archives are generated separately under `dist/`.
Retain both for each released version. License notices alone do not replace
the obligation to supply corresponding source and application code.
