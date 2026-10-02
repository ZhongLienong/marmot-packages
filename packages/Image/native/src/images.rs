use image::codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder};
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader, Limits, Rgba,
    RgbaImage,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

const MAX_RGBA_BYTES: u64 = 256 * 1024 * 1024;

struct Images {
    next: i64,
    buffers: HashMap<i64, RgbaImage>,
}

fn images() -> &'static Mutex<Images> {
    static IMAGES: OnceLock<Mutex<Images>> = OnceLock::new();
    IMAGES.get_or_init(|| {
        Mutex::new(Images {
            next: 1,
            buffers: HashMap::new(),
        })
    })
}

fn register(image: RgbaImage) -> Result<i64, String> {
    let mut images = images().lock().expect("image registry lock poisoned");
    let id = images.next;
    images.next = id.checked_add(1).ok_or("image handle space exhausted")?;
    images.buffers.insert(id, image);
    Ok(id)
}

fn with_image<T>(
    id: i64,
    operation: impl FnOnce(&mut RgbaImage) -> Result<T, String>,
) -> Result<T, String> {
    let mut images = images().lock().expect("image registry lock poisoned");
    let image = images
        .buffers
        .get_mut(&id)
        .ok_or("image is closed or invalid")?;
    operation(image)
}

fn buffer_length(width: u32, height: u32) -> Result<usize, String> {
    if width == 0 || height == 0 {
        return Err("image dimensions must be positive".into());
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_RGBA_BYTES / 4 {
        return Err("image exceeds the 256 MiB RGBA buffer limit".into());
    }
    Ok((pixels * 4) as usize)
}

fn color(packed: i64) -> Result<Rgba<u8>, String> {
    let packed = u32::try_from(packed).map_err(|_| "color must contain four 8-bit channels")?;
    Ok(Rgba(packed.to_be_bytes()))
}

pub(crate) fn create(width: i64, height: i64, packed: i64) -> Result<i64, String> {
    let width = u32::try_from(width).map_err(|_| "image width is outside the supported range")?;
    let height =
        u32::try_from(height).map_err(|_| "image height is outside the supported range")?;
    let length = buffer_length(width, height)?;
    let color = color(packed)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|error| format!("cannot allocate image: {error}"))?;
    for _ in 0..length / 4 {
        bytes.extend_from_slice(&color.0);
    }
    register(RgbaImage::from_raw(width, height, bytes).expect("validated RGBA buffer length"))
}

pub(crate) fn read(path: &Path) -> Result<i64, String> {
    let file =
        File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)
    ) {
        return Err("unsupported image format; expected PNG, JPEG, or WebP".into());
    }
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_RGBA_BYTES);
    reader.limits(limits);
    let decoder = reader
        .into_decoder()
        .map_err(|error| format!("cannot decode image: {error}"))?;
    let (width, height) = decoder.dimensions();
    buffer_length(width, height)?;
    let image = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("cannot decode image: {error}"))?;
    register(image.to_rgba8())
}

pub(crate) fn width(id: i64) -> Result<i64, String> {
    with_image(id, |image| Ok(i64::from(image.width())))
}

pub(crate) fn height(id: i64) -> Result<i64, String> {
    with_image(id, |image| Ok(i64::from(image.height())))
}

fn coordinates(image: &RgbaImage, x: i64, y: i64) -> Result<(u32, u32), String> {
    let x = u32::try_from(x).map_err(|_| "pixel coordinates are outside the image")?;
    let y = u32::try_from(y).map_err(|_| "pixel coordinates are outside the image")?;
    if x >= image.width() || y >= image.height() {
        return Err("pixel coordinates are outside the image".into());
    }
    Ok((x, y))
}

pub(crate) fn get_pixel(id: i64, x: i64, y: i64) -> Result<i64, String> {
    with_image(id, |image| {
        let (x, y) = coordinates(image, x, y)?;
        Ok(i64::from(u32::from_be_bytes(image.get_pixel(x, y).0)))
    })
}

pub(crate) fn set_pixel(id: i64, x: i64, y: i64, packed: i64) -> Result<i64, String> {
    let color = color(packed)?;
    with_image(id, |image| {
        let (x, y) = coordinates(image, x, y)?;
        image.put_pixel(x, y, color);
        Ok(0)
    })
}

fn encode_jpeg(image: &RgbaImage, writer: &mut BufWriter<File>, quality: u8) -> Result<(), String> {
    let rgb: Vec<u8> = image
        .as_raw()
        .chunks_exact(4)
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect();
    JpegEncoder::new_with_quality(writer, quality)
        .encode(&rgb, image.width(), image.height(), ExtendedColorType::Rgb8)
        .map_err(|error| format!("cannot encode JPEG: {error}"))
}

pub(crate) fn write(id: i64, path: &Path, jpeg_quality: Option<i64>) -> Result<i64, String> {
    let quality = jpeg_quality.unwrap_or(90);
    if !(1..=100).contains(&quality) {
        return Err("JPEG quality must be between 1 and 100".into());
    }
    let format = if jpeg_quality.is_some() {
        image::ImageFormat::Jpeg
    } else {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => image::ImageFormat::Png,
            Some("jpg" | "jpeg") => image::ImageFormat::Jpeg,
            Some("webp") => image::ImageFormat::WebP,
            _ => {
                return Err(
                    "unsupported output extension; expected .png, .jpg, .jpeg, or .webp".into(),
                );
            }
        }
    };
    with_image(id, |image| {
        let file = File::create(path)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        let mut writer = BufWriter::new(file);
        match format {
            image::ImageFormat::Png => PngEncoder::new(&mut writer)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|error| format!("cannot encode PNG: {error}"))?,
            image::ImageFormat::WebP => WebPEncoder::new_lossless(&mut writer)
                .write_image(
                    image.as_raw(),
                    image.width(),
                    image.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(|error| format!("cannot encode WebP: {error}"))?,
            image::ImageFormat::Jpeg => encode_jpeg(image, &mut writer, quality as u8)?,
            _ => unreachable!("output formats selected above"),
        }
        writer
            .flush()
            .map_err(|error| format!("cannot finish writing {}: {error}", path.display()))?;
        Ok(0)
    })
}

pub(crate) fn close(id: i64) -> Result<i64, String> {
    let mut images = images().lock().expect("image registry lock poisoned");
    images
        .buffers
        .remove(&id)
        .ok_or("image is closed or invalid")?;
    Ok(0)
}

#[cfg(test)]
mod tests;
