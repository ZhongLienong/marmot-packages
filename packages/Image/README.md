# Image

Basic image IO for Marmot, backed by Rust's
[`image` 0.25.10](https://crates.io/crates/image/0.25.10) crate. Only its PNG,
JPEG, and WebP codec features are enabled; default features are disabled.

## Build

From the collection's root, with Rust 1.88+ and Python 3.10+ installed:

```text
python packages/Image/native/build.py
```

On Windows, Rust needs the MSVC toolchain and Visual Studio C++ Build Tools.
The script builds a release library into `lib/windows/x64/`,
`lib/linux/x86_64/`, or `lib/macos/`. Generated libraries are ignored by Git.
Build on the same platform as the Marmot program. Windows and Linux are checked
in CI; macOS builds are provided by the script but have not been verified.

Run all native and Marmot checks from the collection's root:

```text
python scripts/check.py
```

## Use

Build the native library before installing the package into an application.
Add the collection to your application's `project.marmot`, using the correct
relative path for your checkout:

```toml
[project]
marmot_path = ["../marmot-packages/packages"]

[dependencies]
Image = "^0.1.0"
```

Merge these fields into the existing manifest, then run `marmot install`.
Import the module with `import { <Image> }`. The native library is included in
the vendored package. After rebuilding an already installed version, remove its
vendored `packages/Image-0.1.0/` folder and run `marmot install` again: Marmot's
current source checksum does not include native binaries.

From `packages/Image/`, try the included example:

```text
marmot run examples/create.mmt
```

It creates `target/example.png`, reads it back, and prints its dimensions.

## API

Every operation returns `Result<T, Image::ImageError>`. On failure,
`error.message` contains a description. Import `<Prelude.Result>` to match
`Result::Ok(value)` or `Result::Err(error)`.

| Function | Successful result | Behavior |
| --- | --- | --- |
| `Read(path)` | `Image::Image` | Detect PNG, JPEG, or WebP from file contents and decode to RGBA8 |
| `Create(width, height, color)` | `Image::Image` | Create an image filled with a color |
| `Width(image)`, `Height(image)` | `Int` | Read dimensions |
| `GetPixel(image, x, y)` | `Image::Color` | Read a pixel |
| `SetPixel(image, x, y, color)` | `Unit` | Change a pixel |
| `Write(image, path)` | `Unit` | Select PNG, JPEG, or WebP from the output extension |
| `WriteJpeg(image, path, quality)` | `Unit` | Write JPEG at quality 1–100, regardless of extension |
| `Close(image)` | `Unit` | Release the image buffer |

`Image::Color(r, g, b, a)` has integer channels from 0 to 255. Coordinates are
zero-based, with `(0, 0)` at the top left. Dimensions must be positive; each
RGBA buffer is limited to 256 MiB. Decoders also receive a 256 MiB allocation
limit, subject to the codec's allocation accounting.

PNG and WebP output are lossless and preserve alpha. JPEG is lossy, drops alpha
without compositing, and uses quality 90 with `Write`. Decoded JPEG alpha is
255. Output extensions are `.png`, `.jpg`, `.jpeg`, and `.webp`, ignoring case.
Writing replaces an existing file and does not create parent directories.

Call `Close` once when finished with each image. Handles share a mutable buffer:
copying a handle does not copy its pixels, and closing it invalidates every
alias. Buffers are owned by the native library and are not reclaimed by
Marmot's garbage collector. Registry operations are synchronized for use from
Marmot workers.

This version handles still images only. It does not expose animation,
metadata, orientation correction, resizing, or conversion to Marmot arrays.
