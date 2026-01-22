use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use xcap::Monitor;

#[allow(dead_code)]
pub struct ScreenCapture {
    display_width: u32,
    display_height: u32,
}

#[allow(dead_code)]
impl ScreenCapture {
    pub fn new(display_width: u32, display_height: u32) -> Self {
        Self {
            display_width,
            display_height,
        }
    }

    pub fn capture_screenshot(&self) -> Result<String> {
        let monitors = Monitor::all().context("Failed to get monitors")?;
        let primary = monitors
            .into_iter()
            .find(|m| m.is_primary())
            .context("No primary monitor found")?;

        let image = primary
            .capture_image()
            .context("Failed to capture screen")?;

        let rgba_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_raw(image.width(), image.height(), image.to_vec())
                .context("Failed to create image buffer")?;

        let resized = if rgba_image.width() != self.display_width
            || rgba_image.height() != self.display_height
        {
            image::imageops::resize(
                &rgba_image,
                self.display_width,
                self.display_height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            rgba_image
        };

        let rgb_image = image::DynamicImage::ImageRgba8(resized).to_rgb8();

        let mut jpeg_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut jpeg_bytes);
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 40);
        rgb_image
            .write_with_encoder(encoder)
            .context("Failed to encode JPEG")?;

        Ok(STANDARD.encode(&jpeg_bytes))
    }

    pub fn get_display_size(&self) -> (u32, u32) {
        (self.display_width, self.display_height)
    }
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new(800, 600)
    }
}
