#!/usr/bin/env python3
"""Build the release assets Marmot downloads for a `Name-vX.Y.Z` tag."""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess
import sys
import tarfile
import tomllib


ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ROOT / "packages"
PLATFORMS = ("windows_x64", "linux_x86_64", "macos_arm64", "macos_x86_64")


def released(tag: str) -> tuple[str, str, dict]:
    name, separator, version = tag.rpartition("-v")
    if not separator or not name or not version:
        raise ValueError(f"Tag {tag} is not Name-vX.Y.Z")
    manifest_path = PACKAGES / name / "package.marmot"
    if not manifest_path.is_file():
        raise ValueError(f"Tag {tag} names no package: {manifest_path.relative_to(ROOT)} is missing")
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest["package"]["version"] != version:
        raise ValueError(f"Tag {tag} does not match package.marmot version {manifest['package']['version']}")
    return name, version, manifest


def native(manifest: dict) -> bool:
    return bool(manifest.get("ffi", {}).get("enabled", False))


def describe(tag: str) -> None:
    name, version, manifest = released(tag)
    print(f"name={name}")
    print(f"version={version}")
    print(f"native={'true' if native(manifest) else 'false'}")


def source(tag: str, output: Path) -> None:
    name, version, _ = released(tag)
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"{name}-{version}.tar.gz"
    # Archiving the package's tree at the tag puts its files at the archive
    # root, which is the package directory Marmot unpacks into.
    subprocess.run(
        ["git", "archive", "--format=tar.gz", f"--output={archive}", f"{tag}:packages/{name}"],
        cwd=ROOT,
        check=True,
    )
    print(archive)


def library(tag: str, platform: str, output: Path) -> None:
    name, version, manifest = released(tag)
    if not native(manifest):
        raise ValueError(f"{name} has no native library")
    built = PACKAGES / name / "lib"
    if not any(path.is_file() for path in built.rglob("*")):
        raise ValueError(f"{built.relative_to(ROOT)} is empty; run packages/{name}/native/build.py first")
    output.mkdir(parents=True, exist_ok=True)
    archive = output / f"{name}-{version}-{platform}.tar.gz"
    with tarfile.open(archive, "w:gz", format=tarfile.USTAR_FORMAT) as bundle:
        bundle.add(built, arcname="lib")
    print(archive)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("describe", help="Print name, version and native for GITHUB_OUTPUT").add_argument("tag")
    for command in ("source", "native"):
        subparser = commands.add_parser(command)
        subparser.add_argument("tag")
        subparser.add_argument("--output", type=Path, default=ROOT / "dist")
        if command == "native":
            subparser.add_argument("--platform", choices=PLATFORMS, required=True)
    args = parser.parse_args()
    try:
        if args.command == "describe":
            describe(args.tag)
        elif args.command == "source":
            source(args.tag, args.output)
        else:
            library(args.tag, args.platform, args.output)
        return 0
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
