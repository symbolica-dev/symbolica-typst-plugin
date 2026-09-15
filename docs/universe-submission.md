# Universe submission preparation

Run `python3 scripts/prepare-distribution.py` to create the clean submission
directory at `dist/universe/packages/preview/symbolica/0.1.0` and the runtime
archive at `dist/symbolica-0.1.0.tar.gz`. The submission includes linked manuals,
public examples, and the changelog; those documentation files are excluded
from runtime downloads. Build scripts, tests, source files, and split upload
chunks are not copied into the submission directory.

Run `python3 scripts/prepare-source.py` to create
`dist/symbolica-0.1.0-source.tar.gz`. Publish that corresponding-source archive
alongside the binary release before submitting it. Its offline dependencies
and application source support modification and relinking of the LGPL
components. The source archive belongs in the upstream release assets, not
the Universe runtime download.

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
The joint engine is about 23.36 MiB raw and the complete runtime archive is
about 6 MiB compressed. It shares the algebra and integration implementation,
uses compressed step metadata, removes unrelated C exports, and initializes
rules through a cached Typst transition instead of a large Wizer snapshot.
The repository's 10 MiB download budget is not a Universe limit.

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
permission and the approximately 23.36 MiB Wasm. The permission grants runtime
use, redistribution through Universe and mirrors, and rebuilding with a modified
Typst interface or separately licensed dependencies. It grants no additional
right to modify Symbolica's own source or distribute such modifications, and
preserves third-party license rights. The accompanying
source release supplies the application and dependency source needed to
rebuild and relink the engine. The compressed runtime download remains much
smaller than the raw Wasm.

Do not submit this description until the corresponding-source archive is
published and the licensing arrangement is ready for review.
