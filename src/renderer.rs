use std::path::Path;

use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{ImageReader, RgbaImage};

pub struct ImageRenderer {
    image: RgbaImage,
}

impl ImageRenderer {
    pub fn open(path: &Path) -> Result<Self> {
        let image = ImageReader::open(path).context("Open wallpaper")?.decode().context("Decode wallpaper")?.into_rgba8();

        Ok(Self { image })
    }

    pub fn render_into(&self, dst: &mut [u8], w: u32, h: u32) -> Result<()> {
        debug_assert_eq!(dst.len(), w as usize * h as usize * 4);

        let scaled = self.scale_to_fill(w, h)?;
        crop_center_convert(&scaled, w, h, dst);

        Ok(())
    }

    fn scale_to_fill(&self, target_w: u32, target_h: u32) -> Result<Image<'static>> {
        let (src_w, src_h) = (self.image.width(), self.image.height());
        let scale = f64::max(target_w as f64 / src_w as f64, target_h as f64 / src_h as f64);

        let scaled_w = ((src_w as f64 * scale).ceil() as u32).max(target_w);
        let scaled_h = ((src_h as f64 * scale).ceil() as u32).max(target_h);

        let mut scaled = Image::new(scaled_w, scaled_h, PixelType::U8x4);
        let src = ImageRef::new(src_w, src_h, self.image.as_raw(), PixelType::U8x4).context("Create source ImageRef")?;

        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear)).fit_into_destination(Some((0.5, 0.5)));

        Resizer::new().resize(&src, &mut scaled, Some(&options)).context("Resize wallpaper")?;

        Ok(scaled)
    }
}

fn crop_center_convert(scaled: &Image<'_>, tw: u32, th: u32, dst: &mut [u8]) {
    let (sw, sh) = (scaled.width() as usize, scaled.height() as usize);
    let (tw, th) = (tw as usize, th as usize);
    let x_off = (sw - tw) / 2;
    let y_off = (sh - th) / 2;
    let src = scaled.buffer();
    let src_stride = sw * 4;

    for (y, dst_row) in dst.chunks_exact_mut(tw * 4).enumerate() {
        let row_start = (y_off + y) * src_stride + x_off * 4;
        let src_row = &src[row_start..row_start + tw * 4];

        for (d, s) in dst_row.chunks_exact_mut(4).zip(src_row.chunks_exact(4)) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            d[3] = 0xFF;
        }
    }
}
