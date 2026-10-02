#!/usr/bin/env python3
"""Check the package collection with an installed Marmot or a Release checkout."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
PACKAGES = ROOT / "packages"


def discover() -> list[Path]:
    packages = []
    for directory in sorted(PACKAGES.iterdir()):
        if directory.is_dir():
            if not (directory / "package.marmot").is_file():
                raise ValueError(f"{directory.relative_to(ROOT)} is missing package.marmot")
            packages.append(directory)
    return packages


def toolchain(args: argparse.Namespace) -> tuple[str, dict[str, str]]:
    environment = os.environ.copy()
    search_paths = [str(PACKAGES)]
    if args.toolchain is not None:
        checkout = args.toolchain.resolve()
        if os.name == "nt":
            preset, suffix = "x64-release", ".exe"
        elif sys.platform.startswith("linux"):
            preset, suffix = "linux-release", ""
        else:
            raise ValueError("--toolchain supports Windows and Linux")
        marmot = checkout / "projects" / "marmot" / "target" / "release" / f"marmot{suffix}"
        compiler = checkout / "out" / "build" / "marmotc" / preset / "out" / f"marmotc{suffix}"
        vm = checkout / "out" / "build" / "marmotvm" / preset / "out" / f"marmotvm{suffix}"
        for executable in (marmot, compiler, vm):
            if not executable.is_file():
                raise ValueError(
                    f"{executable} is not built; in the Marmot checkout run "
                    "python scripts/dev.py build compiler vm tool --build Release"
                )
        environment["MARMOTC"] = str(compiler)
        environment["MARMOTVM"] = str(vm)
        search_paths.append(str(checkout / "MarmotPrelude"))
    else:
        executable = shutil.which(args.marmot)
        if executable is None:
            raise ValueError(f"Cannot find {args.marmot}; install Marmot or pass --marmot or --toolchain")
        marmot = Path(executable).resolve()
    if environment.get("MARMOT_PATH"):
        search_paths.append(environment["MARMOT_PATH"])
    environment["MARMOT_PATH"] = os.pathsep.join(search_paths)
    return str(marmot), environment


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    tools = parser.add_mutually_exclusive_group()
    tools.add_argument("--marmot", default="marmot", help="Installed tool name or executable path")
    tools.add_argument("--toolchain", type=Path, help="Marmot checkout built in Release mode")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--list", action="store_true", help="List package folders as JSON")
    modes.add_argument("--github-output", action="store_true", help="Write CI package discovery output")
    args = parser.parse_args()

    try:
        packages = discover()
        if args.list:
            print(json.dumps([directory.name for directory in packages]))
            return 0
        if args.github_output:
            with Path(os.environ["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
                output.write(f"has_packages={str(bool(packages)).lower()}\n")
            print(f"Discovered {len(packages)} package(s).")
            return 0
        if not packages:
            print("No packages yet; nothing to check.")
            return 0

        marmot, environment = toolchain(args)
        for package in packages:
            print(f"Checking {package.name}", flush=True)
            native_build = package / "native" / "build.py"
            if native_build.is_file():
                result = subprocess.run([sys.executable, str(native_build), "--check"], cwd=package, env=environment)
                if result.returncode != 0:
                    return result.returncode
            for command in (["fmt", "--check"], ["check"], ["test"]):
                result = subprocess.run([marmot, *command], cwd=package, env=environment)
                if result.returncode != 0:
                    return result.returncode
        print(f"All {len(packages)} package(s) passed.")
        return 0
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
