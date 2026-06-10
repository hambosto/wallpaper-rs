use std::path::Path;

use anyhow::{Context, Result};
use image::ImageReader;

pub(crate) fn render_into(path: &Path, width: u32, height: u32, dst: &mut [u8]) -> Result<()> {
    let image = ImageReader::open(path).context("cannot open image")?;
    let decode_image = image.decode().context("cannot decode image")?;
    let image_rgba8 = decode_image.into_rgb8();

    let src_w = f64::from(image_rgba8.width());
    let src_h = f64::from(image_rgba8.height());
    let scale = f64::max(f64::from(width) / src_w, f64::from(height) / src_h);
    let x_off = (src_w.mul_add(scale, -f64::from(width))) / 2.0;
    let y_off = (src_h.mul_add(scale, -f64::from(height))) / 2.0;
    let src_h = image_rgba8.height() - 1;
    let src_w = image_rgba8.width() - 1;

    for y in 0..height {
        let sy = ((y_off + f64::from(y)) / scale) as u32;
        let sy = sy.min(src_h);
        for x in 0..width {
            let sx = ((x_off + f64::from(x)) / scale) as u32;
            let sx = sx.min(src_w);
            let [r, g, b] = image_rgba8.get_pixel(sx, sy).0;
            let i = ((y * width + x) * 4) as usize;
            dst[i] = b;
            dst[i + 1] = g;
            dst[i + 2] = r;
            dst[i + 3] = 0xFF;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_image(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("test.png");
        let img = image::RgbaImage::from_fn(4, 4, |x, y| if x < 2 && y < 2 { image::Rgba([255, 0, 0, 255]) } else { image::Rgba([0, 0, 255, 255]) });
        image::DynamicImage::ImageRgba8(img)
            .write_with_encoder(image::codecs::png::PngEncoder::new(std::fs::File::create(&path).unwrap()))
            .unwrap();
        path
    }

    #[test]
    fn render_into_fills_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_image(dir.path());
        let mut buf = vec![0u8; 8 * 8 * 4];

        render_into(&path, 8, 8, &mut buf).unwrap();

        for chunk in buf.chunks(4) {
            assert_eq!(chunk[3], 0xFF);
            assert!(chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0);
        }
    }

    #[test]
    fn render_into_returns_error_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.png");
        let mut buf = vec![0u8; 4];

        assert!(render_into(&path, 1, 1, &mut buf).is_err());
    }
}
