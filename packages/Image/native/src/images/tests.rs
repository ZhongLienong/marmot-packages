use super::*;

#[test]
fn png_and_webp_round_trip_rgba_and_guess_content_without_an_extension() {
    let directory = tempfile::tempdir().unwrap();
    let source = create(2, 2, 0xff0000ff).unwrap();
    set_pixel(source, 1, 0, 0x12345678).unwrap();
    for extension in ["png", "webp"] {
        let path = directory.path().join(format!("画像.{extension}"));
        write(source, &path, None).unwrap();
        let renamed = directory.path().join("image.data");
        std::fs::rename(path, &renamed).unwrap();
        let loaded = read(&renamed).unwrap();
        assert_eq!(width(loaded).unwrap(), 2);
        assert_eq!(height(loaded).unwrap(), 2);
        assert_eq!(get_pixel(loaded, 1, 0).unwrap(), 0x12345678);
        assert_eq!(get_pixel(loaded, 0, 1).unwrap(), 0xff0000ff);
        close(loaded).unwrap();
        std::fs::remove_file(renamed).unwrap();
    }
    close(source).unwrap();
}

#[test]
fn jpeg_writes_rgb_with_the_requested_quality() {
    let directory = tempfile::tempdir().unwrap();
    let source = create(16, 16, 0xe0201000).unwrap();
    let path = directory.path().join("photo.bin");
    write(source, &path, Some(95)).unwrap();
    let loaded = read(&path).unwrap();
    let pixel = get_pixel(loaded, 8, 8).unwrap() as u32;
    let [red, green, blue, alpha] = pixel.to_be_bytes();
    assert!(red.abs_diff(224) <= 3);
    assert!(green.abs_diff(32) <= 3);
    assert!(blue.abs_diff(16) <= 3);
    assert_eq!(alpha, 255);
    assert_eq!(width(loaded).unwrap(), 16);
    assert_eq!(height(loaded).unwrap(), 16);
    close(source).unwrap();
    close(loaded).unwrap();
}

#[test]
fn invalid_dimensions_colors_coordinates_and_closed_handles_are_errors() {
    for (width, height) in [
        (0, 1),
        (-1, 1),
        (1, 0),
        (1, -1),
        (i64::MAX, 1),
        (100_000, 100_000),
        (i64::from(u32::MAX), i64::from(u32::MAX)),
    ] {
        assert!(create(width, height, 0).is_err());
    }
    assert!(create(1, 1, -1).is_err());
    let source = create(2, 3, 0).unwrap();
    for (x, y) in [(-1, 0), (0, -1), (2, 0), (0, 3)] {
        assert!(get_pixel(source, x, y).is_err());
        assert!(set_pixel(source, x, y, 0).is_err());
    }
    assert!(set_pixel(source, 0, 0, i64::MAX).is_err());
    close(source).unwrap();
    assert!(width(source).is_err());
    assert!(close(source).is_err());
    let next = create(1, 1, 0).unwrap();
    assert_ne!(source, next);
    assert!(height(source).is_err());
    close(next).unwrap();
}

#[test]
fn missing_corrupt_and_unsupported_files_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("missing.png");
    assert!(read(&path).unwrap_err().contains("cannot read"));
    std::fs::write(&path, b"not an image").unwrap();
    assert!(
        read(&path)
            .unwrap_err()
            .contains("unsupported image format")
    );
    std::fs::write(&path, b"\x89PNG\r\n\x1a\ntruncated").unwrap();
    assert!(read(&path).unwrap_err().contains("cannot decode"));
    std::fs::write(&path, b"GIF89a").unwrap();
    assert!(
        read(&path)
            .unwrap_err()
            .contains("unsupported image format")
    );
}

#[test]
fn invalid_output_requests_do_not_truncate_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    let source = create(1, 1, 0).unwrap();
    let path = directory.path().join("existing.gif");
    std::fs::write(&path, "preserved").unwrap();
    assert!(write(source, &path, None).is_err());
    for quality in [0, 101, -1] {
        assert!(write(source, &path, Some(quality)).is_err());
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "preserved");
    assert!(write(source, &directory.path().join("missing/out.png"), None).is_err());
    close(source).unwrap();
}

#[test]
fn resizing_and_cropping_make_new_images_and_flips_change_the_original() {
    let source = create(3, 2, 0x000000ff).unwrap();
    set_pixel(source, 0, 0, 0xff0000ff).unwrap();
    set_pixel(source, 2, 1, 0x00ff00ff).unwrap();

    let plain = create(4, 4, 0x336699ff).unwrap();
    let resized = resize(plain, 7, 2).unwrap();
    assert_eq!((width(resized).unwrap(), height(resized).unwrap()), (7, 2));
    assert_eq!(get_pixel(resized, 6, 1).unwrap(), 0x336699ff);
    assert_eq!(get_pixel(plain, 3, 3).unwrap(), 0x336699ff);
    assert!(resize(plain, 0, 4).is_err());
    close(resized).unwrap();
    close(plain).unwrap();

    let cropped = crop(source, 1, 1, 2, 1).unwrap();
    assert_eq!((width(cropped).unwrap(), height(cropped).unwrap()), (2, 1));
    assert_eq!(get_pixel(cropped, 1, 0).unwrap(), 0x00ff00ff);
    assert!(crop(source, 2, 0, 2, 1).is_err());
    assert!(crop(source, 3, 0, 1, 1).is_err());
    close(cropped).unwrap();

    flip_horizontal(source).unwrap();
    assert_eq!(get_pixel(source, 2, 0).unwrap(), 0xff0000ff);
    flip_vertical(source).unwrap();
    assert_eq!(get_pixel(source, 2, 1).unwrap(), 0xff0000ff);
    assert_eq!(get_pixel(source, 0, 0).unwrap(), 0x00ff00ff);
    close(source).unwrap();
}
