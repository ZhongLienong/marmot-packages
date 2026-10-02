#!/usr/bin/env python3
"""Build the Image package's native library for the current host."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys


NATIVE = Path(__file__).resolve().parent
PACKAGE = NATIVE.parent
# Marmot vendors every file in a package, so keep Cargo's build cache outside
# the package directory and copy only the finished library into it.
TARGET = PACKAGE.parents[1] / "target" / PACKAGE.name / "native"


def host_library() -> tuple[str, Path]:
    machine = platform.machine().lower()
    if os.name == "nt" and machine in ("amd64", "x86_64"):
        return "marmot_image.dll", PACKAGE / "lib" / "windows" / "x64"
    if sys.platform.startswith("linux") and machine == "x86_64":
        return "libmarmot_image.so", PACKAGE / "lib" / "linux" / "x86_64"
    if sys.platform == "darwin" and machine in ("arm64", "x86_64"):
        return "libmarmot_image.dylib", PACKAGE / "lib" / "macos"
    raise ValueError(f"Unsupported host: {sys.platform} {machine}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Check formatting, run Clippy and native tests before building")
    args = parser.parse_args()
    try:
        filename, destination = host_library()
        commands = []
        if args.check:
            commands.extend([
                ["cargo", "fmt", "--check"],
                ["cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings"],
                ["cargo", "test", "--locked"],
            ])
        commands.append(["cargo", "build", "--locked", "--release"])
        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(TARGET)
        # Compile the Windows CRT into this DLL, so distributing the package
        # does not require a separate Visual C++ runtime installation.
        if os.name == "nt":
            environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(["-C", "target-feature=+crt-static"])
        for command in commands:
            print("+ " + " ".join(command), flush=True)
            result = subprocess.run(command, cwd=NATIVE, env=environment)
            if result.returncode != 0:
                return result.returncode
        destination.mkdir(parents=True, exist_ok=True)
        library = destination / filename
        shutil.copy2(TARGET / "release" / filename, library)
        print(f"Built {library}")
        return 0
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
