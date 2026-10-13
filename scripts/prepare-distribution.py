#!/usr/bin/env python3
"""Stage Universe documentation separately from the runtime download."""
import argparse
import gzip
import hashlib
from html.parser import HTMLParser
import io
import json
import re
import shutil
import tarfile
import tempfile
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ("typst.toml", "README.md", "LICENSE", "LICENSE-SYMBOLICA.md",
           "LICENSE-SYMBOLICA-TYPST.md", "THIRD_PARTY.md", "THIRD_PARTY_LICENSES.txt",
           "REBUILDING.md", "symbolica/lib.typ", "symbolica/render.typ",
           "symbolica/symbolica.wasm", "symbolica/assets/symbolica-logo.svg",
           "symbolica/assets/README.md")
DOCUMENTATION = ("CHANGELOG.md", "symbolica/manual.pdf",
                 *(f"symbolica/examples/{name}.typ" for name in
                   ("basic", "showcase", "expression-grid", "lotka-volterra",
                    "phase-portrait", "integration", "optimization")))


class ReadmeImages(HTMLParser):
    """Collect the image resources used by theme-responsive picture markup."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.destinations = []

    def handle_starttag(self, tag, attributes):
        if tag not in ("img", "source"):
            return
        attributes = dict(attributes)
        if tag == "img" and attributes.get("src"):
            self.destinations.append(attributes["src"])
        if attributes.get("srcset"):
            for candidate in attributes["srcset"].split(","):
                parts = candidate.split()
                if parts:
                    self.destinations.append(parts[0])


def readme_destinations(text):
    """Read Markdown destinations and decoded HTML image attributes."""
    prose = re.sub(r"```[^\n]*\n.*?```", "", text, flags=re.S)
    images = ReadmeImages()
    images.feed(prose)
    return [*re.findall(r"!?\[[^\]\n]*\]\(([^\s)]+)\)", prose),
            *images.destinations]


def validate_readme(package_root):
    """Check the local resources Universe resolves from the submission tree."""
    text = (package_root / "README.md").read_text()
    prose = re.sub(r"```[^\n]*\n.*?```", "", text, flags=re.S)
    if "$$" in prose:
        raise ValueError("README math must use rendered images, not $$ blocks")
    headings = set()
    for title in re.findall(r"^#{1,6}\s+(.+)$", prose, flags=re.M):
        headings.add(re.sub(r"[^\w\- ]", "", title.lower()).replace(" ", "-"))
    for destination in readme_destinations(text):
        link = urlsplit(destination)
        if link.scheme or link.netloc:
            continue
        if not link.path:
            if link.fragment and unquote(link.fragment) not in headings:
                raise ValueError(f"README has a missing section: {destination}")
            continue
        target = (package_root / unquote(link.path)).resolve()
        if not target.is_relative_to(package_root.resolve()) or not target.is_file():
            raise ValueError(f"README resource is absent from the submission: {destination}")


def write_archive(destination, paths, root):
    with destination.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as tar:
                for name in sorted(paths):
                    data = (root / name).read_bytes()
                    info = tarfile.TarInfo(name)
                    info.size, info.mode, info.mtime = len(data), 0o644, 0
                    tar.addfile(info, io.BytesIO(data))


def prepare(root=ROOT, release=False, source_revision=None):
    if release and not re.fullmatch(r"[0-9a-f]{40}", source_revision or ""):
        raise ValueError("Release staging requires --source-revision with the full Git commit")
    package = tomllib.loads((root / "typst.toml").read_text())["package"]
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    registry_path = Path("packages/preview") / package["name"] / package["version"]
    stage = dist / "universe" / registry_path
    runtime = list(RUNTIME)
    documentation = (*DOCUMENTATION, *(str(p.relative_to(root)) for p in
                                      sorted((root / "symbolica/readme").glob("*.svg"))))
    # Replace only this script's generated directory, never the checkout.
    with tempfile.TemporaryDirectory(dir=dist, prefix="stage-") as temporary:
        prepared = Path(temporary)
        for name in (*runtime, *documentation):
            target = prepared / name
            target.parent.mkdir(parents=True, exist_ok=True)
            source_name = "README-universe.md" if name == "README.md" else name
            shutil.copyfile(root / source_name, target)
        if release:
            provenance = {
                "package": package["name"],
                "version": package["version"],
                "repository": package["repository"],
                "revision": source_revision,
                "source": f"{package['repository']}/tree/{source_revision}",
                "sha256": {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                           for name in ("Cargo.lock", "flake.lock", "symbolica/symbolica.wasm")},
            }
            (prepared / "SOURCE.json").write_text(json.dumps(provenance, indent=2) + "\n")
            runtime.append("SOURCE.json")
            rebuilding = prepared / "REBUILDING.md"
            rebuilding.write_text(rebuilding.read_text().replace(
                f"{package['repository']}/blob/main/", f"{package['repository']}/blob/{source_revision}/"))
        validate_readme(prepared)
        if stage.exists():
            shutil.rmtree(stage)
        stage.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(prepared, stage)
    package_id = f"{package['name']}-{package['version']}"
    archive = dist / f"{package_id}.tar.gz"
    write_archive(archive, runtime, stage)
    if archive.stat().st_size > 10 * 1024 * 1024:
        raise RuntimeError("Runtime archive exceeds our 10 MiB download budget")
    submission = dist / f"{package_id}-universe.tar.gz"
    files = [str(p.relative_to(dist / "universe")) for p in stage.rglob("*") if p.is_file()]
    write_archive(submission, files, dist / "universe")
    print(f"Submission: {stage}\nUniverse archive: {submission}")
    print(f"Runtime archive: {archive} ({archive.stat().st_size:,} bytes)")
    return stage


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true", help="record source provenance for a release")
    parser.add_argument("--source-revision", help="full Git commit recorded in SOURCE.json")
    args = parser.parse_args()
    prepare(release=args.release, source_revision=args.source_revision)


if __name__ == "__main__":
    main()
