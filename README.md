# Symbolica for Typst

The official [Symbolica](https://symbolica.io/) plugin for Typst. Compute with
symbolic expressions, solve equations, differentiate, integrate, and evaluate
formulas directly in your documents. Use within Typst is free, including
commercial use.

**Version 0.1.0 is available on [Typst Universe](https://typst.app/universe/package/symbolica/).**
Use its documentation for the published API. This checkout prepares **version
0.2.0**, including the renamed `parse`, `literal`, and `function-head` functions.
Version 0.2.0 is not yet published; `@preview/symbolica:0.2.0` becomes available
after its registry submission is merged and deployed.
See the [changelog](CHANGELOG.md) for the 0.2.0 changes and the
[migration guide](MIGRATING.md) for updating existing documents and plugins.

The [package guide](README-universe.md) describes the current API and becomes the
README shipped to Universe. See also the [user manual](symbolica/manual.pdf),
[minimal example](symbolica/examples/basic.typ), and
[polynomial-system showcase](symbolica/examples/showcase.typ).

## Try the development version

Clone this repository and register it as a local Typst package. On Linux:

```sh
git clone https://github.com/symbolica-dev/symbolica-typst-plugin.git
cd symbolica-typst-plugin
mkdir -p "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica"
ln -s "$PWD" \
  "${XDG_DATA_HOME:-$HOME/.local/share}/typst/packages/local/symbolica/0.2.0"
```

On macOS, use `~/Library/Application Support/typst/packages` in place of the
Linux data directory. Then import the checkout with `@local`:

```typst
#import "@local/symbolica:0.2.0": *

#let x = literal("x")
#let f = parse($x^4 - 5 x^2 + 4$)
$ #to-typst(factor(f)), quad #to-typst(derivative(f, x)) $
```

When trying the package guide locally, replace `@preview` with `@local` in its
imports. The version in the local package path must match `typst.toml`.

## Build and check

The Nix development shell provides Typst, its pinned package dependencies, and
the Rust and Wasm build tools:

```sh
nix develop
nix run .#build-engine
nix run .#check
```

`nix flake check` validates the bundled plugin, examples, and manual without
rebuilding the Wasm. Use `nix run .#manual` to regenerate the manual and
`nix run .#typst -- <arguments>` to run Typst without entering the shell.
See [rebuilding instructions](REBUILDING.md) for the Rust features, build
commands, and corresponding source archive.

The package guide includes generated SVG outputs so equations display on both
GitHub and Universe. Check its examples with:

```sh
nix develop -c python3 scripts/readme-examples.py --readme README-universe.md
```

Add `--update` after changing an example to regenerate its preview.
The same command checks and regenerates the banner and gallery artwork from
`docs/readme-assets`.

See [release instructions](docs/releasing.md) for an artifact-only dry run and
the version-tagged publishing workflow.

## Reuse the Rust library

The `symbolica-typst-plugin` crate provides the Typst plugin and a reusable
library for plugins such as tydenso. Its public `payload`, `math_display`, and
`typst_ast` modules provide annotated Symbolica expressions, portable notation,
and Parsely conversion.

Native consumers use the default `native` feature. Wasm consumers disable
defaults and enable only `wasm`:

```toml
[dependencies]
symbolica-typst-plugin = { path = "../symbolica-typst-plugin", default-features = false, features = ["wasm"] }
```

Read and validate attachments before importing their Atom so your plugin can
register any required symbol callbacks. Preserve attachments you do not
interpret when exporting the result.

Normal Cargo builds produce an `rlib`. The optional `plugin` feature adds this
package's entry points, integration engine, and runtime setup; leave it disabled
when embedding the library. The build script selects `cdylib` and `wasm,plugin`
to produce the standalone Wasm module.

The crate is not yet published on crates.io. Use a path dependency or pinned Git
revision. Symbolica 3.0.1 comes from crates.io; no Cargo patch is needed. See
[rebuilding](REBUILDING.md) for build options and dependency overrides.

## Licensing

The original Typst interface and Rust adapter are [MIT licensed](LICENSE).
Symbolica is source-available under its [own terms](LICENSE-SYMBOLICA.md), with
an [explicit permission for Typst](LICENSE-SYMBOLICA-TYPST.md) allowing free use,
including commercial use, without a license key. The bundled Wasm also contains
dependencies under other licenses; see the [third-party notices](THIRD_PARTY.md)
and [complete license texts](THIRD_PARTY_LICENSES.txt).
