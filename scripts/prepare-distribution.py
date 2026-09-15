#!/usr/bin/env python3
"""Stage the Universe submission and a deterministic runtime archive."""
import gzip
import io
import shutil
import tarfile
import tempfile
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ("typst.toml", "README.md", "LICENSE", "LICENSE-SYMBOLICA.md",
           "LICENSE-SYMBOLICA-TYPST.md", "THIRD_PARTY.md", "THIRD_PARTY_LICENSES.txt",
           "REBUILDING.md", "symbolica/lib.typ", "symbolica/render.typ",
           "symbolica/symbolica.wasm")
DOCUMENTATION = ("CHANGELOG.md", "symbolica/manual.pdf",
                 *(f"symbolica/examples/{name}.typ" for name in
                   ("basic", "showcase", "expression-grid", "lotka-volterra",
                    "phase-portrait", "integration")))


def write_archive(destination, paths):
    with destination.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w", format=tarfile.GNU_FORMAT) as tar:
                for name in sorted(paths):
                    data = (ROOT / name).read_bytes()
                    info = tarfile.TarInfo(name)
                    info.size, info.mode, info.mtime = len(data), 0o644, 0
                    tar.addfile(info, io.BytesIO(data))


def main():
    package = tomllib.loads((ROOT / "typst.toml").read_text())["package"]
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    stage = dist / "universe/packages/preview" / package["name"] / package["version"]
    # Replace only this script's generated staging directory, never the checkout.
    with tempfile.TemporaryDirectory(dir=dist, prefix="stage-") as temporary:
        prepared = Path(temporary)
        for name in (*RUNTIME, *DOCUMENTATION):
            target = prepared / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, target)
        if stage.exists():
            shutil.rmtree(stage)
        stage.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(prepared, stage)
    archive = dist / f"{package['name']}-{package['version']}.tar.gz"
    write_archive(archive, RUNTIME)
    if archive.stat().st_size > 10 * 1024 * 1024:
        raise RuntimeError("Runtime archive exceeds our 10 MiB download budget")
    print(f"Submission: {stage}\nRuntime archive: {archive} ({archive.stat().st_size:,} bytes)")
    print("Custom licensing and Wasm size require Universe maintainer review before submission.")


if __name__ == "__main__":
    main()
