# Spenso Wasm compatibility snapshot

This directory is a source snapshot of `../gammaloop/crates/spenso` from the
Gammaloop worktree at Git commit `114c5e158c06c2450a467557c31ec1b1f4cb725d`.
The Idenso plugin itself remains a direct path dependency on
`../gammaloop/crates/idenso`.

`src/shadowing/symbolica_utils.rs` is synchronized with Gammaloop commit
`092811c505463cca1f1ec9aa9ee90fe51b56a87f` so the snapshot exposes the print
backend resolver required by that Idenso revision. The remaining source stays
at the base snapshot unless a newer Idenso API requires it.

The snapshot is temporary. It makes the current Spenso code usable in a
`wasm32-unknown-unknown` Typst plugin without changing the sibling Gammaloop
worktree:

- workspace dependencies are made explicit and the Hakari workspace feature
  unifier is omitted;
- native compiled-evaluator and C++ export APIs are gated behind the new
  `native-code-generation` feature; and
- the 4 GiB term-streaming limit is converted safely on 32-bit targets.

Once equivalent Wasm feature gates are available upstream, this snapshot and
the path-source patch in the repository root can be removed.
