//! The viewer's "Copy image" context-menu action decodes fetched image bytes
//! via `tauri::image::Image::from_bytes`, whose supported formats depend on
//! the tauri `image-*` cargo features enabled in Cargo.toml. Remote images on
//! the web are typically JPEG/WebP/PNG — pin all three so dropping a feature
//! fails loudly instead of surfacing "format Jpeg is not supported" at
//! runtime.

#[test]
fn image_from_bytes_supports_jpeg_webp_png() {
    for (bytes, format) in [
        (&include_bytes!("fixtures/images/2x2.jpg")[..], "jpeg"),
        (&include_bytes!("fixtures/images/2x2.webp")[..], "webp"),
        (&include_bytes!("fixtures/images/2x2.png")[..], "png"),
    ] {
        tauri::image::Image::from_bytes(bytes)
            .unwrap_or_else(|e| panic!("{format} decoding must be enabled: {e}"));
    }
}
