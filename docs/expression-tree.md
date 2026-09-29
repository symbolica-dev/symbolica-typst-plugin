# Semantic expression tree, version 1

Use this contract to consume or rebuild Symbolica scalar expressions without
parsing printed mathematics. It describes normalized algebra: `x+x` is already
`2*x`, division uses negative powers, and original parentheses and term order
are not retained. It is independent of Parsely and the internal rendering tree.

## Typst

```typ
#import "@preview/symbolica:0.1.0": *

#let result = inspect(parse($x^2/(1+x)$))
#assert(result.ok)
#let tree = result.tree
#assert.eq(tree.root.kind, "product")
#to-typst(from-tree(tree))
```

`inspect(value)` returns `(ok: true, tree: ...)` or `(ok: false, error: str)`.
It accepts existing scalar payload bytes, not mathematical content or strings.
Other input types, incompatible envelopes, and matrices return an error value.
`from-tree(tree)` validates the contract, imports its leaves, and builds a new
expression. Invalid trees raise an error, like other algebra operations.

These functions are on the development branch; the published 0.1.0 package
does not yet provide them.

## Rust

The `expression_tree` module is part of the reusable crate, with no `plugin`
feature required:

```rust
use symbolica_typst_plugin::{expression_tree::ExpressionTree, payload};

fn example(input: &[u8]) -> Result<Vec<u8>, String> {
    let parsed = payload::parse_payload(input).map_err(|e| e.to_string())?;
    // Inspect parsed.attachments() and register any required symbol-data readers.
    let atom = parsed.import_atom().map_err(|e| e.to_string())?;
    let tree = ExpressionTree::from_atom(&atom, &parsed.attachment_set())?;
    let cbor = tree.encode()?;

    let received = ExpressionTree::decode(&cbor)?;
    // Decoding does not import native leaves. Inspect received.attachments and
    // register your symbol-data readers before the next step.
    let rebuilt = received.to_atom()?;
    let output = payload::encode_atom_from_set(&rebuilt.atom, &rebuilt.attachments)
        .map_err(|e| e.to_string())?;
    Ok(output)
}
```

`Node` is an enum with a variant for each kind below. `from_value` and `to_value`
convert to `ciborium::Value`. `decode` also checks the wire byte limit and rejects
trailing CBOR data. `validate` merges all leaf attachments without importing
Atoms; callers constructing a tree directly can use its returned attachment set
for registration before `to_atom`.

## CBOR contract

The envelope is a map with exactly these fields:

| Field | Value |
| --- | --- |
| `protocol` | `"symbolica"` |
| `version` | `1` |
| `kind` | `"expression-tree"` |
| `root` | Node |
| `attachments` | Array of attachment maps |

Each node is a map with `kind` and the fields in this table. There are no generic
`args` or `slots` fields.

| `kind` | Fields |
| --- | --- |
| `rational` | `numerator: str`, `denominator: str` |
| `complex` | `real: Rational`, `imaginary: Rational` |
| `float` | `real: float`, `imaginary: float`, `payload: bytes` |
| `coefficient` | `payload: bytes` |
| `symbol` | `symbol: Symbol` |
| `call` | `head: Symbol`, `arguments: [Node, ...]` |
| `sum` | `terms: [Node, ...]` |
| `product` | `factors: [Node, ...]` |
| `power` | `base: Node`, `exponent: Node` |

A `Rational` map has `numerator` and `denominator`, without a `kind` field.
Both are decimal integer strings; a numerator may begin with `-`, and the
denominator must be positive. Integers have denominator `"1"`. Import reduces
fractions, so `2/4` becomes `1/2`.

Float components are approximate binary64 values for consumers doing numerical
work. The native payload retains the original precision and is authoritative;
import rejects approximations that disagree with it. Replace the entire node
to change its value. Other coefficient domains, including finite fields,
rational polynomials, infinities, and indeterminate values, use an opaque
`coefficient` node. A consumer that cannot interpret one should report that
rather than approximate it silently.

A `Symbol` map has exactly:

| Field | Value |
| --- | --- |
| `name` | Full Symbolica name, including namespace |
| `namespace` | Namespace string |
| `short-name` | Name without the namespace |
| `tags` | Array of strings |
| `attributes` | Array of attribute names |
| `payload` | Native Atom envelope for that symbol alone |

Attribute names are `symmetric`, `antisymmetric`, `cyclesymmetric`, `linear`,
`flat`, `scalar`, `real`, `integer`, and `positive`. Import checks the descriptor
against the payload. Compare full names when identifying variables or built-in
functions. Custom symbols in other namespaces can have the same short name.

Decorated variables and function heads are single symbols. For example,
`C_(i j)` does not expose `i` and `j` as algebraic operands. Its label remains
in symbol data and portable `symbolica.math-display` attachments. The symbol
payload also preserves native data that another plugin may understand.

An attachment map contains `schema: str`, `version: uint`, `identity: bytes`,
and `data: bytes`, with the same meaning as the Atom payload envelope. Preserve
unknown attachments. Identical entries merge; conflicting data for the same
key is rejected before importing any leaf.

Exports collect annotations on the tree envelope; native leaf envelopes have
no separate attachment entries. When combining nodes from different trees,
merge their attachment environments too. Import also accepts attachments on
leaf envelopes and merges them during validation.

Empty sums and products import as zero and one. Calls can have no arguments.
Rebuilding uses Symbolica normalization and symbol attributes, so it can combine,
sort, or simplify nodes. There is no retained whole-expression payload that
could override edits to the algebraic children.

## Validation and compatibility

Unknown fields and node kinds, duplicate map fields, unsupported versions,
invalid rational components, and conflicting attachments are errors. Trees are
limited to 64 edges of algebraic nesting, 8,192 nodes, and 10 MiB of encoded CBOR.
Rational components have at most 4,096 digits. Existing payload and attachment
limits also apply.

Native leaf payloads require a compatible Symbolica export format. Schema
validation checks their envelopes without importing them; it does not validate
the opaque native bodies. As with the existing payload API, import is intended
for trusted, compatible plugin exports, not arbitrary forged native Atom data.
Tree versioning does not remove that native format requirement.
