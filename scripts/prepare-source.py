#!/usr/bin/env python3
"""Prepare an optional offline source archive with locked dependencies."""
import gzip
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    package = tomllib.loads((ROOT / "typst.toml").read_text())["package"]
    name = f"{package['name']}-{package['version']}-source"
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=dist, prefix="source-") as temporary:
        source = Path(temporary) / name
        source.mkdir()
        for filename in ("Cargo.toml", "Cargo.lock", "flake.nix", "flake.lock",
                         "typst.toml", "README.md", "REBUILDING.md", "LICENSE",
                         "LICENSE-SYMBOLICA.md", "LICENSE-SYMBOLICA-TYPST.md",
                         "THIRD_PARTY.md", "THIRD_PARTY_LICENSES.txt", "CHANGELOG.md"):
            shutil.copyfile(ROOT / filename, source / filename)
        for directory in ("src", "crates", "scripts", "docs/license-sources"):
            shutil.copytree(ROOT / directory, source / directory,
                            ignore=shutil.ignore_patterns("__pycache__", "*.pyc"))
        for path in (ROOT / "symbolica").rglob("*.typ"):
            target = source / path.relative_to(ROOT)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
        config = subprocess.check_output(
            ["cargo", "vendor", "--locked", "--versioned-dirs", str(source / "vendor")],
            cwd=ROOT, text=True)
        config = config.replace(str(source / "vendor"), "vendor")
        (source / ".cargo").mkdir()
        (source / ".cargo/config.toml").write_text(config)
        archive = dist / f"{name}.tar.gz"
        with archive.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
                with tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as tar:
                    for path in sorted(source.rglob("*")):
                        if not path.is_file():
                            continue
                        info = tar.gettarinfo(str(path), arcname=f"{name}/{path.relative_to(source)}")
                        info.uid = info.gid = info.mtime = 0
                        info.uname = info.gname = ""
                        info.mode = 0o644
                        with path.open("rb") as file:
                            tar.addfile(info, file)
        print(f"Corresponding source: {archive} ({archive.stat().st_size:,} bytes)")
        print("Optional offline archive; source access may instead use the matching repository revision and crates.io.")


if __name__ == "__main__":
    main()
