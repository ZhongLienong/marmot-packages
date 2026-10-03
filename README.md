# marmot-packages

A collection of independently versioned Marmot libraries.

| Package | Version | Purpose |
| --- | --- | --- |
| [Image](packages/Image/) | 0.1.0 | PNG, JPEG, and WebP image IO |

```text
marmot-packages/
  packages/                 Package sources, one folder per package
  scripts/check.py          Check every package
  .github/workflows/check.yml
```

## Adding a package

Run `marmot init packages/Name --package --name Name` from the repository root
when you are ready to add a library. Each package owns its manifest, public entry
module, implementation modules, tests, examples, README, and license.

```text
packages/Name/
  package.marmot
  Name.mmt
  Name/                     Modules such as Name.Parser
  test/                     .mmt tests and .expected snapshots
  examples/
  README.md
  LICENSE
```

Keep importable modules at the package root: Marmot adds that directory to the
compiler's search paths. `Name.mmt` should expose the public API through
`public export`; manifest exports alone do not enforce visibility.

## Checking packages

With Python 3.10+ and Marmot installed:

```text
python scripts/check.py
```

The script checks formatting, type-checks the entry module, and runs each
package's test suite. Packages with `native/build.py` first run that script with
`--check` to check and build their native library. Image requires Rust 1.88+
and the platform's Rust linker tools (Visual Studio C++ Build Tools on Windows).
The script adds this repository's `packages/` directory to the local package
index so sibling dependencies can resolve.

Use `python scripts/check.py --list` to list package folders. With a Release
build of a Marmot checkout, use `--toolchain ../Marmot` instead of the installed
tool. This mode supports Windows and Linux.

CI checks Windows and Linux. Its Marmot revision is pinned in
`.github/workflows/check.yml`; update that revision when adopting a newer
toolchain. Toolchain builds are skipped while the collection is empty.

## Using the collection

Declare package versions under `[dependencies]` and run `marmot install`.
Marmot downloads released versions from this repository, with the prebuilt
native library for the current platform, and vendors them into the project.

To work against unreleased sources instead, clone this repository, build any
native libraries you need (for example `python packages/Image/native/build.py`)
and add its `packages/` directory to the application's `[project].marmot_path`.

## Releases

Version each package in its own `package.marmot`, then push a tag named
`Name-v0.1.0` for that version. `.github/workflows/release.yml` publishes a
GitHub release with the package's sources and, for packages with native code,
the library built for Windows x64, Linux x86_64 and macOS arm64 and x86_64. The
release stays a draft until every build succeeds. Keep released tags unchanged.

`python scripts/release.py` builds the same assets locally into `dist/`.

Generated targets, native libraries, and each package's vendored dependencies
are ignored. The top-level `packages/` source collection remains tracked.
