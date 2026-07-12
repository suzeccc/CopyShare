use std::io::Cursor;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{DynamicImage, GenericImageView, ImageFormat};

use crate::models::OcrResponse;

const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 25_000_000;
const PREVIEW_MAX_DIMENSION: u32 = 1400;
const OCR_MAX_DIMENSION: u32 = 2600;

#[derive(Debug)]
struct PreparedImage {
    preview_png: Vec<u8>,
    ocr_png: Vec<u8>,
    image_width: u32,
    image_height: u32,
}

fn validate_decoded_size(size: usize) -> Result<(), String> {
    if size > MAX_IMAGE_BYTES {
        return Err("图片尺寸过大，请缩小后重试。".to_string());
    }
    Ok(())
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), String> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 {
        return Err("无法读取剪贴板中的图片。".to_string());
    }
    if pixels > MAX_IMAGE_PIXELS {
        return Err("图片尺寸过大，请缩小后重试。".to_string());
    }
    Ok(())
}

fn scaled_dimensions(width: u32, height: u32, max_dimension: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_dimension {
        return (width, height);
    }
    let scale = max_dimension as f64 / longest as f64;
    (
        (width as f64 * scale).round().max(1.0) as u32,
        (height as f64 * scale).round().max(1.0) as u32,
    )
}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    image
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| "无法读取剪贴板中的图片。".to_string())?;
    Ok(output.into_inner())
}

fn resized_png(image: &DynamicImage, max_dimension: u32) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    let (target_width, target_height) = scaled_dimensions(width, height, max_dimension);
    if (target_width, target_height) == (width, height) {
        return encode_png(image);
    }
    encode_png(&image.resize_exact(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    ))
}

fn prepare_image(content: &str) -> Result<PreparedImage, String> {
    let decoded = STANDARD
        .decode(content)
        .map_err(|_| "无法读取剪贴板中的图片。".to_string())?;
    validate_decoded_size(decoded.len())?;
    let image = image::load_from_memory(&decoded)
        .map_err(|_| "无法读取剪贴板中的图片。".to_string())?;
    let (image_width, image_height) = image.dimensions();
    validate_dimensions(image_width, image_height)?;
    Ok(PreparedImage {
        preview_png: resized_png(&image, PREVIEW_MAX_DIMENSION)?,
        ocr_png: resized_png(&image, OCR_MAX_DIMENSION)?,
        image_width,
        image_height,
    })
}

fn normalize_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn response_from_engine<F>(prepared: PreparedImage, recognize: F) -> OcrResponse
where
    F: FnOnce(&[u8]) -> Result<String, String>,
{
    let preview_base64 = STANDARD.encode(&prepared.preview_png);
    match recognize(&prepared.ocr_png) {
        Ok(text) => OcrResponse {
            text: normalize_text(&text),
            preview_base64,
            image_width: prepared.image_width,
            image_height: prepared.image_height,
            error: None,
        },
        Err(error) => OcrResponse {
            text: String::new(),
            preview_base64,
            image_width: prepared.image_width,
            image_height: prepared.image_height,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    fn png_base64(width: u32, height: u32) -> String {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
            width,
            height,
            Rgba([255, 255, 255, 255]),
        ));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        STANDARD.encode(bytes.into_inner())
    }

    #[test]
    fn rejects_invalid_base64_and_corrupt_images() {
        assert!(prepare_image("not base64").unwrap_err().contains("无法读取"));
        assert!(prepare_image(&STANDARD.encode(b"not an image"))
            .unwrap_err()
            .contains("无法读取"));
    }

    #[test]
    fn rejects_byte_and_pixel_limits() {
        assert!(validate_decoded_size(MAX_IMAGE_BYTES + 1).is_err());
        assert!(validate_dimensions(5001, 5000).is_err());
        assert!(validate_dimensions(0, 100).is_err());
    }

    #[test]
    fn scales_without_stretching() {
        assert_eq!(scaled_dimensions(4000, 2000, 1400), (1400, 700));
        assert_eq!(scaled_dimensions(800, 600, 1400), (800, 600));
    }

    #[test]
    fn prepares_preview_and_ocr_from_one_source() {
        let prepared = prepare_image(&png_base64(1600, 800)).unwrap();
        assert_eq!((prepared.image_width, prepared.image_height), (1600, 800));
        let preview = image::load_from_memory(&prepared.preview_png).unwrap();
        let ocr = image::load_from_memory(&prepared.ocr_png).unwrap();
        assert_eq!((preview.width(), preview.height()), (1400, 700));
        assert_eq!((ocr.width(), ocr.height()), (1600, 800));
    }

    #[test]
    fn normalizes_lines_and_preserves_empty_success() {
        assert_eq!(normalize_text("  first  \r\nsecond   \n\n"), "first\nsecond");
        assert_eq!(normalize_text(" \r\n "), "");
    }

    #[test]
    fn engine_failure_keeps_the_prepared_preview() {
        let prepared = prepare_image(&png_base64(20, 10)).unwrap();
        let response = response_from_engine(prepared, |_| Err("engine failed".to_string()));
        assert!(response.preview_base64.len() > 10);
        assert_eq!(response.error.as_deref(), Some("engine failed"));
        assert_eq!(response.text, "");
    }
}
