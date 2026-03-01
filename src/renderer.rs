use std::path::Path;

use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::ImageReader;

pub fn render_into(dst: &mut [u8], path: &Path, w: u32, h: u32) -> Result<()> {
    debug_assert_eq!(dst.len(), w as usize * h as usize * 4);

    let img = ImageReader::open(path).context("Open wallpaper")?.decode().context("Decode wallpaper")?.into_rgba8();

    let (src_w, src_h) = (img.width(), img.height());
    let scale = f64::max(w as f64 / src_w as f64, h as f64 / src_h as f64);
    let scaled_w = ((src_w as f64 * scale).ceil() as u32).max(w);
    let scaled_h = ((src_h as f64 * scale).ceil() as u32).max(h);

    let mut scaled = Image::new(scaled_w, scaled_h, PixelType::U8x4);
    {
        let src = ImageRef::new(src_w, src_h, img.as_raw(), PixelType::U8x4).context("Create source ImageRef")?;
        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear)).fit_into_destination(Some((0.5, 0.5)));
        Resizer::new().resize(&src, &mut scaled, Some(&options)).context("Resize wallpaper")?;
    }
    drop(img);
    crop_and_convert(&scaled, w, h, dst);

    Ok(())
}

fn crop_and_convert(scaled: &Image<'_>, tw: u32, th: u32, dst: &mut [u8]) {
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
