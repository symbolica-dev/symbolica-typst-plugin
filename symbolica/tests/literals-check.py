"""Run with `nix develop -c python3 symbolica/tests/literals-check.py`."""
from pathlib import Path
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "dist" / "literal-check" / "errors"
OUTPUT.mkdir(parents=True, exist_ok=True)

CASES = (
    ("wildcard-name", 'literal("x_")', "names must not end with an underscore"),
    ("wildcard-whitespace", 'literal("x_ ")', "surrounding whitespace"),
    ("empty-name", 'literal("")', "nonempty string"),
    ("literal-type", 'literal(2)', "expected a symbol-name string"),
    ("empty-label", 'literal([])', "label must contain visible content"),
    ("blank-label", 'literal([ ])', "label must contain visible content"),
    ("named-wildcard", 'literal($x$, name: "x_")', "names must not end with an underscore"),
    ("named-string", 'literal("x", name: "y")', "name is only available for content labels"),
    ("wild-base", 'wild("x_")', "names must not end with an underscore"),
    ("wild-level-zero", 'wild("x", level: 0)', "level must be a positive integer"),
    ("wild-level-negative", 'wild("x", level: -1)', "level must be a positive integer"),
    ("wild-level-float", 'wild("x", level: 1.5)', "level must be a positive integer"),
    ("styled", 'literal(text(red)[x])', "unsupported element styled"),
    ("context", 'literal(context [x])', "unsupported element context"),
    ("nested-layout", 'literal($a_(#rect())$)', "bottom attachment of a"),
    ("unsupported-field", 'literal($vec(x,y,delim:"[")$)', "unsupported field vec.delim"),
    ("orphan-metadata", 'literal(metadata((protocol: "symbolica", version: 1, kind: "atom", atom: bytes(()))))', "unsupported standalone element metadata"),
    ("incomplete", 'literal($x+$)', "unparsed content"),
    ("conflicting-label", '{ let a = literal($x$, name: "shared"); literal($y$, name: "shared") }', "conflicting"),
)

for name, expression, expected in CASES:
    source = OUTPUT / f"{name}.typ"
    source.write_text('#import "../../../symbolica/lib.typ": *\n#let result = ' + expression + '\n')
    result = subprocess.run(
        ["typst", "compile", "--root", str(ROOT), str(source), str(source.with_suffix(".pdf"))],
        capture_output=True, text=True,
    )
    assert result.returncode != 0, name + " unexpectedly succeeded"
    assert expected.lower() in result.stderr.lower(), name + ": " + result.stderr
    print("Passed literal diagnostic: " + name, flush=True)

# Export in one process and import in another to test self-contained symbol data.
producer = OUTPUT / "portable-producer.typ"
producer.write_text('''#import "../../../symbolica/lib.typ": *
#let saved = parse(literal($x+y$, name: "portable", namespace: "separate_process", tags: ("model::quantity",)))
#metadata(array(saved)) <literal-payload>
''')
result = subprocess.run(
    ["typst", "eval", "--root", str(ROOT), "--in", str(producer),
     "query(<literal-payload>).first().value", "--format", "json"],
    capture_output=True, text=True, check=True,
)
(OUTPUT / "portable.bin").write_bytes(bytes(json.loads(result.stdout)))
consumer = OUTPUT / "portable-consumer.typ"
consumer.write_text('''#import "../../../symbolica/lib.typ": *
#let value = read("portable.bin", encoding: none)
#assert.eq(canonical(value, namespaces: true), "separate_process::portable")
#assert.eq(to-typst-source(value), "lr((x + y))")
#assert.eq(canonical(parse(to-typst(value)), namespaces: true), canonical(value, namespaces: true))
#let tagged = to-typst(value, notation: notation(tags: ("model::quantity": $theta$)))
#context { assert.eq(measure(tagged).width, measure($theta$).width) }
#tagged
''')
subprocess.run(
    ["typst", "compile", "--root", str(ROOT), str(consumer), str(consumer.with_suffix(".pdf"))],
    check=True,
)
print("Passed fresh-process literal payload round trip", flush=True)
