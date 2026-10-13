# README artwork

These Typst sources generate the Universe README banner and three gallery
previews in light and dark palettes. Every equation, solution, and integration step is
computed by the package while Typst renders the artwork.

Regenerate and check them together with the README's runnable examples:

```sh
nix develop -c python3 scripts/readme-examples.py --readme README-universe.md --update
nix develop -c python3 scripts/readme-examples.py --readme README-universe.md
```

A `readme-artwork` comment in the README selects a source in this directory
and its SVG in `symbolica/readme`. The generator passes `theme=light` and
`theme=dark` to the same Typst source; the dark output has a `-dark.svg` suffix.
Small `<picture>` elements select the theme while the rest of the README stays
Markdown. Both image paths are checked during package staging.
SVGs are staged as Universe documentation,
and excluded from the runtime download. The sources stay in this repository.

The banner declares its supporting SVGs with `// readme-artwork-dependency:`
comments. The same command generates and checks those dependencies first.
`algebraic-curves.typ` samples a family of cubic curves, while
`optimization-valley.typ` derives the Rosenbrock gradient and Hessian, solves
for its minimum, and draws a local damped Newton iteration through the contours.

The palette follows [the manual artwork](../manual-assets/README.md). The banner
uses the package's existing `logo()` function; see [logo provenance](../../symbolica/assets/README.md).
The optimization gallery card reuses the banner's computed valley and links to
the standalone [optimization example](../../symbolica/examples/optimization.typ).
The integration card shows a large antiderivative; its linked example contains
the complete Rubi rule trace.
