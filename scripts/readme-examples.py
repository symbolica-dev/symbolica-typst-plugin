#!/usr/bin/env python3
"""Compile README examples and check their generated equation previews."""

import argparse
import base64
from html.parser import HTMLParser
from pathlib import Path
import re
import subprocess
import tempfile
import tomllib
from urllib.parse import unquote_to_bytes
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = re.compile(
    r"(?:^<!-- readme-example: ([a-z][a-z0-9-]*) -->\n)?"
    r"^```typst\n(.*?)^```[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
IMPORTS = re.compile(r'"@(preview|local)/symbolica:([^"\s]+)"')
ARTWORK = re.compile(r"^<!-- readme-artwork: ([a-z][a-z0-9-]*) -->$", re.MULTILINE)
ARTWORK_DEPENDENCIES = re.compile(
    r"^// readme-artwork-dependency: (.*?)$", re.MULTILINE,
)
ARTWORK_NAME = re.compile(r"[a-z][a-z0-9-]*")
PREAMBLE = {
    "light": """#set page(width: auto, height: auto, margin: 10pt, fill: white)
#set text(font: "New Computer Modern", size: 11pt, fill: black)
""",
    "dark": """#set page(width: auto, height: auto, margin: 10pt, fill: rgb("#132c29"))
#set text(font: "New Computer Modern", size: 11pt, fill: rgb("#f2f3e9"))
""",
}
SVG_NS = "http://www.w3.org/2000/svg"
XLINK_NS = "http://www.w3.org/1999/xlink"
SVG_URL = re.compile(r"url\(\s*(['\"]?)#([^'\"\s)]+)\1\s*\)")
ET.register_namespace("", SVG_NS)
ET.register_namespace("xlink", XLINK_NS)


class ReadmeImages(HTMLParser):
    """Read decoded image attributes in light/dark picture markup."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.destinations = set()

    def handle_starttag(self, tag, attributes):
        if tag not in ("img", "source"):
            return
        attributes = dict(attributes)
        if tag == "img" and attributes.get("src"):
            self.destinations.add(attributes["src"])
        if attributes.get("srcset"):
            for candidate in attributes["srcset"].split(","):
                parts = candidate.split()
                if parts:
                    self.destinations.add(parts[0])


def preview_destinations(readme):
    prose = re.sub(r"```[^\n]*\n.*?```", "", readme, flags=re.S)
    images = ReadmeImages()
    images.feed(prose)
    return images.destinations | set(re.findall(r"!?\[[^\]\n]*\]\(([^\s)]+)\)", prose))


def inline_svg_images(data: bytes) -> bytes:
    """Keep nested vector artwork visible when an SVG is used as an image."""
    root = ET.fromstring(data)
    if root.tag != f"{{{SVG_NS}}}svg":
        raise ValueError("README artwork must have an SVG root")
    used_ids = {element.get("id") for element in root.iter() if element.get("id")}
    embedded_count = 0

    def prefix_ids(svg):
        nonlocal embedded_count
        while True:
            prefix = f"readme-svg-{embedded_count}-"
            embedded_count += 1
            if not any(identifier.startswith(prefix) for identifier in used_ids):
                break
        ids = {element.get("id"): prefix + element.get("id")
               for element in svg.iter() if element.get("id")}
        used_ids.update(ids.values())

        def rewrite_urls(value):
            return SVG_URL.sub(lambda match: f"url(#{ids.get(match[2], match[2])})", value)

        for element in svg.iter():
            if element.get("id"):
                element.set("id", ids[element.get("id")])
            for attribute, value in tuple(element.attrib.items()):
                value = rewrite_urls(value)
                if attribute in ("href", f"{{{XLINK_NS}}}href") and value.startswith("#"):
                    value = "#" + ids.get(value[1:], value[1:])
                elif attribute in ("aria-labelledby", "aria-describedby"):
                    value = " ".join(ids.get(identifier, identifier) for identifier in value.split())
                element.set(attribute, value)
            if element.tag == f"{{{SVG_NS}}}style" and element.text:
                # Rewrite selector IDs separately so CSS color literals stay intact.
                selectors = re.compile(r"([^{}]+)(?=\{)")

                def rewrite_selector(match):
                    return re.sub(r"#([\w-]+)",
                                  lambda target: "#" + ids.get(target[1], target[1]), match[0])

                element.text = rewrite_urls(selectors.sub(rewrite_selector, element.text))

    def inline_children(parent):
        for index, element in enumerate(list(parent)):
            if element.tag != f"{{{SVG_NS}}}image":
                inline_children(element)
                continue
            href = element.get("href") or element.get(f"{{{XLINK_NS}}}href", "")
            if not href.lower().startswith("data:image/svg+xml"):
                continue
            header, separator, payload = href.partition(",")
            if not separator:
                raise ValueError("README artwork has an invalid embedded SVG URL")
            decoded = (base64.b64decode(payload, validate=True)
                       if ";base64" in header.lower() else unquote_to_bytes(payload))
            embedded = ET.fromstring(decoded)
            if embedded.tag != f"{{{SVG_NS}}}svg":
                raise ValueError("Embedded README artwork must have an SVG root")
            if "width" not in element.attrib or "height" not in element.attrib:
                raise ValueError("Embedded README artwork needs an explicit image viewport")
            inline_children(embedded)
            prefix_ids(embedded)
            geometry = {"x", "y", "width", "height", "preserveAspectRatio"}
            wrapper = ET.Element(f"{{{SVG_NS}}}g", {
                attribute: value for attribute, value in element.attrib.items()
                if attribute not in geometry and attribute not in ("href", f"{{{XLINK_NS}}}href")
            })
            embedded.set("x", element.get("x", "0"))
            embedded.set("y", element.get("y", "0"))
            embedded.set("width", element.get("width"))
            embedded.set("height", element.get("height"))
            embedded.set("preserveAspectRatio", element.get("preserveAspectRatio", "xMidYMid meet"))
            embedded.set("overflow", "hidden")
            wrapper.append(embedded)
            wrapper.tail = element.tail
            parent.remove(element)
            parent.insert(index, wrapper)

    inline_children(root)
    return ET.tostring(root, encoding="utf-8") if embedded_count else data


def check_examples(package_root: Path, update: bool, readme_path: Path | None = None):
    package = tomllib.loads((package_root / "typst.toml").read_text())["package"]
    readme_path = readme_path or package_root / "README.md"
    readme = readme_path.read_text()
    # Only the repository guide documents a local development installation.
    namespace = "local" if readme_path == ROOT / "README.md" else "preview"
    examples = list(EXAMPLES.finditer(readme))
    if not examples:
        raise ValueError("README has no Typst examples")
    marked = re.findall(r"^<!-- readme-example: (.*?) -->$", readme, re.MULTILINE)
    names = [match[1] for match in examples if match[1]]
    if marked != names or len(names) != len(set(names)):
        raise ValueError("Each preview marker must have a unique name and immediately precede a Typst code block")
    artwork = ARTWORK.findall(readme)
    if len(artwork) != len(set(artwork)) or set(artwork) & set(names):
        raise ValueError("Artwork and example preview names must be unique")
    destinations = preview_destinations(readme)

    def preview_path(name, theme):
        suffix = "-dark" if theme == "dark" else ""
        return Path("symbolica/readme") / f"{name}{suffix}.svg"

    for name in (*names, *artwork):
        for theme in PREAMBLE:
            relative = preview_path(name, theme)
            if relative.as_posix() not in destinations:
                raise ValueError(f"README preview {name} has no {theme} image reference")

    stale = []
    with tempfile.TemporaryDirectory(prefix="symbolica-readme-") as temporary:
        work = Path(temporary)
        packages = work / "packages"
        link = packages / namespace / package["name"] / package["version"]
        link.parent.mkdir(parents=True)
        link.symlink_to(package_root, target_is_directory=True)
        for theme in PREAMBLE:
            (work / theme).mkdir()

        def compile_source(source, output, theme):
            subprocess.run(
                ["typst", "compile", "--creation-timestamp", "0", "--package-path", str(packages),
                 "--input", f"theme={theme}", "--root", str(work), str(source), str(output)],
                check=True,
            )

        def check_preview(output, name, theme):
            relative = preview_path(name, theme)
            expected = package_root / relative
            if update:
                destination = ROOT / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(output.read_bytes())
            elif not expected.is_file() or output.read_bytes() != expected.read_bytes():
                stale.append(str(relative))

        for index, match in enumerate(examples, 1):
            name, code = match[1], match[2]
            imports = IMPORTS.findall(code)
            if not imports or any(import_namespace != namespace or version != package["version"]
                                  for import_namespace, version in imports):
                raise ValueError(f"README example {index} must import @{namespace}/symbolica:{package['version']}")
            for theme in PREAMBLE if name else ("light",):
                source = work / theme / f"example-{index}.typ"
                output = source.with_suffix(".svg" if name else ".pdf")
                source.write_text(PREAMBLE[theme] + code)
                compile_source(source, output, theme)
                if name:
                    check_preview(output, name, theme)
            print(f"Compiled README example {index}" + (f" ({name})" if name else ""))
        compiled_artwork = set()
        compiling_artwork = []

        def compile_artwork(name, theme):
            key = (name, theme)
            if key in compiled_artwork:
                return
            if key in compiling_artwork:
                cycle = " -> ".join(asset for asset, _ in (*compiling_artwork, key))
                raise ValueError(f"README artwork dependency cycle: {cycle}")
            if name in names:
                raise ValueError(f"README artwork {name} conflicts with an example preview")
            compiling_artwork.append(key)
            code = (ROOT / "docs/readme-assets" / f"{name}.typ").read_text()
            imports = IMPORTS.findall(code)
            if not imports or any(import_namespace != namespace or version != package["version"]
                                  for import_namespace, version in imports):
                raise ValueError(f"README artwork {name} must import @{namespace}/symbolica:{package['version']}")
            dependencies = ARTWORK_DEPENDENCIES.findall(code)
            if any(not ARTWORK_NAME.fullmatch(dependency) for dependency in dependencies):
                raise ValueError(f"README artwork {name} has an invalid dependency name")
            if len(dependencies) != len(set(dependencies)):
                raise ValueError(f"README artwork {name} repeats a dependency name")
            for dependency in dependencies:
                compile_artwork(dependency, theme)
            source = work / theme / f"artwork-{name}.typ"
            # Dependencies are addressed by their declared SVG filename in Typst.
            output = work / theme / f"{name}.svg"
            source.write_text(code)
            compile_source(source, output, theme)
            # Browsers and SVG renderers can reject images nested inside image SVGs.
            # Flatten dependencies before a parent artwork imports them.
            output.write_bytes(inline_svg_images(output.read_bytes()))
            check_preview(output, name, theme)
            compiling_artwork.pop()
            compiled_artwork.add(key)
            print(f"Compiled README artwork {name} ({theme})")

        for name in artwork:
            for theme in PREAMBLE:
                compile_artwork(name, theme)
    if stale:
        raise ValueError("Stale README previews: " + ", ".join(stale)
                         + "; run python3 scripts/readme-examples.py --readme README-universe.md --update")
    print(f"Checked {len(examples)} README examples, {len(names) * 2} equation previews, and {len(compiled_artwork)} artwork previews")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-root", type=Path, default=ROOT,
                        help="Package whose README, manifest, and Wasm to test")
    parser.add_argument("--readme", type=Path,
                        help="README to test instead of PACKAGE_ROOT/README.md")
    parser.add_argument("--update", action="store_true",
                        help="Regenerate previews in the repository's symbolica/readme directory")
    args = parser.parse_args()
    check_examples(args.package_root.resolve(), args.update,
                   args.readme.resolve() if args.readme else None)


if __name__ == "__main__":
    main()
