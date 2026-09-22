"""Run with `nix develop -c python3 symbolica/tests/parsing-check.py`."""
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "dist" / "parsing-check" / "errors"
OUTPUT.mkdir(parents=True, exist_ok=True)

CASES = (
    ("string-grammar", 'parse("x+1", grammar: (:))', "grammar applies only to Typst math content"),
    ("integer-grammar", "parse(2, grammar: (:))", "grammar applies only to Typst math content"),
    ("float-grammar", "parse(0.5, grammar: (:))", "grammar applies only to Typst math content"),
    ("payload-grammar", 'parse(parse("x"), grammar: (:))', "grammar applies only to Typst math content"),
    ("engine-grammar", "(init().parse)(2, grammar: (:))", "grammar applies only to Typst math content"),
    ("none-input", "parse(none)", "found type(none)"),
    ("boolean-input", "parse(true)", "found bool"),
    ("array-input", "parse((1, 2))", "found array"),
    ("dictionary-input", "parse((x: 2))", "found dictionary"),
    ("function-input", "parse(x => x)", "found function"),
    ("atom-wildcard-import", "atom(2)", "unknown variable: atom"),
)


def check(name, source_text, expected):
    source = OUTPUT / f"{name}.typ"
    source.write_text(source_text)
    result = subprocess.run(
        ["typst", "compile", "--root", str(ROOT), str(source), str(source.with_suffix(".pdf"))],
        capture_output=True, text=True,
    )
    assert result.returncode != 0, name + " unexpectedly succeeded"
    assert expected.lower() in result.stderr.lower(), name + ": " + result.stderr
    print("Passed parse diagnostic: " + name, flush=True)


for name, expression, expected in CASES:
    check(name, '#import "../../../symbolica/lib.typ": *\n#let result = ' + expression + "\n", expected)

check("atom-named-import", '#import "../../../symbolica/lib.typ": atom\n', "unresolved import")
check(
    "atom-module-access",
    '#import "../../../symbolica/lib.typ" as public\n#let result = public.atom(2)\n',
    "does not contain `atom`",
)
