# Universe submission preparation

This checkout prepares **Symbolica 0.2.0**. Version 0.1.0 is already published;
submit the update in a new `packages/preview/symbolica/0.2.0` directory. The
0.2.0 package becomes available after its registry PR is merged and deployment
completes. A GitHub release alone does not publish it to Universe.

## Prepare the submission

Use the [release workflow](releasing.md) for the complete build, validation,
source archive, and checksums. Its manual trigger performs an artifact-only dry
run; a matching version tag prepares the GitHub release and submission branch.

For local package staging after building and checking the plugin:

```sh
nix run .#package
nix develop -c python3 scripts/readme-examples.py --package-root dist/universe/packages/preview/symbolica/0.2.0
```

The generated submission directory is
`dist/universe/packages/preview/symbolica/0.2.0`. Packaging copies
`README-universe.md` to its `README.md` and includes the linked manual, migration
guide, changelog, public examples, and light/dark README SVGs. These documentation
files remain in the registry submission and are excluded from runtime downloads.
Rust source, build scripts, tests, and artwork source files are not copied into
the submission directory.

The release workflow produces:

- `symbolica-0.2.0.tar.gz`: runtime files.
- `symbolica-0.2.0-universe.tar.gz`: the complete registry submission tree.
- `symbolica-0.2.0-source.tar.gz`: matching source and vendored dependencies.
- `symbolica-0.2.0-manual.pdf` and `SHA256SUMS`.

Release staging includes `SOURCE.json` with the source revision and checksums
for the Wasm, `Cargo.lock`, and `flake.lock`. Keep the matching revision publicly
accessible and include the source archive alongside the binaries. See
[rebuilding instructions](../REBUILDING.md) for reproducing the plugin.

## Review the registry update

Follow the current [submission guidelines](https://github.com/typst/packages/blob/main/docs/README.md)
and [licensing guidance](https://github.com/typst/packages/blob/main/docs/licensing.md).
Keep the existing license texts and scope notices with the update:
`license = "MIT"` in the manifest covers the original Typst interface and Rust
adapter, while the bundled Wasm contains components under their own terms.
The [Symbolica Typst permission](../LICENSE-SYMBOLICA-TYPST.md),
[third-party notices](../THIRD_PARTY.md), and
[complete dependency license texts](../THIRD_PARTY_LICENSES.txt) document those
terms. Preserve that distinction in the submission README.

Check that the staged README's current examples import 0.2.0, all local links
resolve, and the light fallback and dark artwork display in supported viewers.
The migration guide deliberately
retains the published 0.1.0 Before example. Rendered equation SVGs replace raw
LaTeX blocks. Review the generated archive sizes; the repository's 10 MiB runtime
budget is a project check, while registry size review remains case by case.

## Suggested update description

Symbolica 0.2.0 updates the Typst interface to `parse`, `literal`, and
`function-head`, with the full rename table and before/after examples in the
migration guide. It adds semantic expression trees, portable literal labels,
and direct special-function constructors. Symbolica 3.0.1 requires regenerating
saved native expression payloads from 0.1.0.

The update also improves rendering and integration initialization, and adds a
computed optimization example. The README has generated light/dark artwork;
its manual, migration guide, examples, and image assets are included as registry
documentation and excluded from runtime downloads. Matching source, dependency
notices, build instructions, and checksums accompany the release.
