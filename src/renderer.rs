use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::ImageReader;

pub struct ImageRenderer {
    path: PathBuf,
}

impl ImageRenderer {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self { path: path.to_path_buf() })
    }

    pub fn render_into(&self, dst: &mut [u8], w: u32, h: u32) -> Result<()> {
        debug_assert_eq!(dst.len(), w as usize * h as usize * 4);

        let src = ImageReader::open(&self.path).context("Open wallpaper")?.decode().context("Decode wallpaper")?.into_rgba8();

        let src_w = src.width() as f64;
        let src_h = src.height() as f64;
        let src_stride = src.width() as i64 * 4;
        let src = src.into_raw();

        let scale = f64::max(w as f64 / src_w, h as f64 / src_h);

        let scaled_w = (src_w * scale).ceil() as i64;
        let scaled_h = (src_h * scale).ceil() as i64;

        let x_off = (scaled_w - w as i64) / 2;
        let y_off = (scaled_h - h as i64) / 2;

        for y in 0..h {
            let src_y = ((y_off + y as i64) as f64 / scale) as i64;

            for x in 0..w {
                let src_x = ((x_off + x as i64) as f64 / scale) as i64;

                let src_idx = ((src_y * src_stride) + (src_x * 4)) as usize;
                let dst_idx = ((y * w + x) * 4) as usize;

                dst[dst_idx] = src[src_idx + 2];
                dst[dst_idx + 1] = src[src_idx + 1];
                dst[dst_idx + 2] = src[src_idx];
                dst[dst_idx + 3] = 0xFF;
            }
        }

        Ok(())
    }
}
