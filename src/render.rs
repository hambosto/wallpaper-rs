use std::path::Path;

use anyhow::{Context, Result};
use image::ImageReader;

pub fn render_into(path: &Path, width: u32, height: u32, dst: &mut [u8]) -> Result<()> {
    let src = ImageReader::open(path).context("Cannot open image")?.decode().context("Cannot decode image")?.into_rgba8();

    let scale = f64::max(width as f64 / src.width() as f64, height as f64 / src.height() as f64);
    let x_off = ((src.width() as f64).mul_add(scale, -(width as f64))) / 2.0;
    let y_off = ((src.height() as f64).mul_add(scale, -(height as f64))) / 2.0;

    for y in 0..height {
        let sy = (((y_off + y as f64) / scale) as u32).min(src.height() - 1);
        for x in 0..width {
            let sx = (((x_off + x as f64) / scale) as u32).min(src.width() - 1);
            let [r, g, b, _] = src.get_pixel(sx, sy).0;
            let i = ((y * width + x) * 4) as usize;
            dst[i] = b;
            dst[i + 1] = g;
            dst[i + 2] = r;
            dst[i + 3] = 0xFF;
        }
    }

    Ok(())
}
