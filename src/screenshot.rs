use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{ImageBuffer, ImageFormat, Rgba};
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

        let mut png_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut png_bytes);
        resized
            .write_to(&mut cursor, ImageFormat::Png)
            .context("Failed to encode PNG")?;

        Ok(STANDARD.encode(&png_bytes))
    }

    pub fn get_display_size(&self) -> (u32, u32) {
        (self.display_width, self.display_height)
    }
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}
