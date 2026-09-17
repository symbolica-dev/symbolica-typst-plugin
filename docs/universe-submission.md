# Universe submission preparation

Run `python3 scripts/prepare-distribution.py` to create the clean submission
directory at `dist/universe/packages/preview/symbolica/0.1.0` and the runtime
archive at `dist/symbolica-0.1.0.tar.gz`. The submission includes the linked manual,
public examples, and the changelog; those documentation files are excluded
from runtime downloads. Build scripts, tests, source files, and split upload
chunks are not copied into the submission directory.

Publish the matching plugin source revision, lockfile, and build scripts and
link them from the binary release. `REBUILDING.md` describes those sources and
the exact crates.io dependency sources listed in `THIRD_PARTY_LICENSES.txt`.
Source may be hosted on other servers under GPLv3 Section 6(d), provided there
are clear directions beside the binary and equivalent access is maintained.
A separate archive is not mandatory. `python3 scripts/prepare-source.py`
optionally creates a vendored source archive for offline builds and backup;
keep it out of the Universe runtime download.

## Dependency licensing issue to resolve before distribution

The Symbolica allowance leaves its existing reverse-engineering restrictions
unchanged. LGPLv3 Section 4 requires the combined work's terms to permit
reverse engineering for debugging modifications to its LGPL-covered portions.
Do not treat the current combination as cleared for distribution: resolve
that conflict through an appropriate narrow permission, an alternative
license for the affected dependencies, or a build without those dependencies.
A source archive does not resolve this issue.

## Decisions still required from Universe maintainers

The [licensing rules](https://github.com/typst/packages/blob/main/docs/licensing.md)
require an OSI-approved license or one of the specified Creative Commons
licenses. The custom Typst-only allowance removes runtime requirements and
grants redistribution and rebuilding, but does not make Symbolica an
OSI-licensed component. The manifest declares `LicenseRef-Symbolica-Typst`
instead of presenting the entire Wasm as MIT. The current
[bundler](https://github.com/typst/packages/blob/main/bundler/src/main.rs)
rejects custom license references under its normal validation. An explicit
maintainer exception or an accepted licensing arrangement is still needed;
do not remove the custom term merely to make validation pass.

Size also needs a case-by-case decision under the
[submission guidelines](https://github.com/typst/packages/blob/main/docs/README.md).
The joint engine is shipped as about 6.08 MiB of zlib-compressed data plus a
29 KiB inflater plugin. Typst loads the inflater first, decompresses the engine
in memory, and then loads its original 23.36 MiB of Wasm. The raw engine is
excluded from both the submission and runtime archive. It shares the algebra and integration implementation,
uses compressed step metadata, removes unrelated C exports, and initializes
rules through a cached Typst transition instead of a large Wizer snapshot.
The repository enforces a 10 MiB budget for each plugin asset and the runtime
archive; these are local checks, not a claim about Universe policy.

The README retains the requested LaTeX equivalents. Universe's Markdown
renderer differs from GitHub's; check their presentation in the submission
preview and use rendered equation images if its renderer does not support
the math blocks.

## Suggested submission description

Symbolica is the official Symbolica 3.0 integration for Typst. It provides exact
algebra, symbolic integration with steps, numerical evaluation, and system
solving in one Wasm module. All uses of this plugin within Typst are free and
require no Symbolica subscription, registration, activation, or license key.

Authors can compute symbolic results and numerical values directly in their
papers, without precomputing them in another tool and copying them back by
hand. This avoids error-prone copy-pasting and keeps displayed results in sync
when equations or parameters change.

This submission requests review of two exceptions: the custom Symbolica Typst
permission and the engine's size (about 6.11 MiB of shipped assets, expanding
to 23.36 MiB of Wasm in memory). The permission grants runtime
use, redistribution through Universe and mirrors, and rebuilding unmodified
Symbolica as part of the plugin. It grants no additional right to modify
Symbolica's own source or distribute such modifications. The matching plugin
source, lockfile, build instructions, and exact dependency source links are
provided for rebuilding. The compressed runtime download remains much
smaller than the raw Wasm.

Do not submit this description until the matching source revision is published
and both the dependency licensing issue and Universe licensing arrangement
are resolved.
