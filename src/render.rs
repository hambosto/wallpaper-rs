use std::path::Path;

use anyhow::{Context, Result};
use image::{ImageReader, RgbImage};

pub struct Render {
    image: RgbImage,
}

impl Render {
    pub fn new(path: &Path) -> Result<Self> {
        let src = ImageReader::open(path).context("cannot open image")?;
        let decoded_image = src.decode().context("cannot decode image")?;
        let image_rgb = decoded_image.into_rgb8();

        Ok(Self { image: image_rgb })
    }

    pub fn render(&self, width: u32, height: u32, dst: &mut [u8]) {
        let (src_w, src_h) = self.image.dimensions();
        let (sw, sh) = (f64::from(src_w), f64::from(src_h));
        let scale = (f64::from(width) / sw).max(f64::from(height) / sh);
        let x_off = sw.mul_add(scale, -f64::from(width)) / 2.0;
        let y_off = sh.mul_add(scale, -f64::from(height)) / 2.0;
        let stride = src_w as usize * 3;
        let pixels = self.image.as_raw();

        for (i, dst_pixel) in dst.chunks_exact_mut(4).enumerate() {
            let dx = (i as u32) % width;
            let dy = (i as u32) / width;

            let sx = ((x_off + f64::from(dx)) / scale) as u32;
            let sy = ((y_off + f64::from(dy)) / scale) as u32;

            let src_x = sx.min(src_w - 1) as usize;
            let src_y = sy.min(src_h - 1) as usize;
            let src = &pixels[src_y * stride + src_x * 3..][..3];

            dst_pixel.copy_from_slice(&[src[2], src[1], src[0], 0xFF]);
        }
    }
}
