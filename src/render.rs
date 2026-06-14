use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, GenericImageView, ImageDecoder, ImageReader, Rgba, RgbaImage};

use crate::config::{CropGravity, ResizeConfig, ResizeStrategy};

pub struct Render {
    rgba: RgbaImage,
}

impl Render {
    pub fn new(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!("path does not exist: {}", path.display());
        }
        if !path.is_file() {
            anyhow::bail!("path is not a file: {}", path.display());
        }

        let file = File::open(path).context("failed to open image")?;
        let reader = ImageReader::new(BufReader::new(file)).with_guessed_format().context("failed to detect image format")?;

        let mut decoder = reader.into_decoder().context("failed to create decoder")?;
        let orientation = decoder.orientation().context("failed to read orientation")?;

        let mut image = DynamicImage::from_decoder(decoder).context("failed to decode image")?;
        image.apply_orientation(orientation);

        Ok(Self { rgba: image.into_rgba8() })
    }

    pub fn render(&self, width: u32, height: u32, dst: &mut [u8], resize: &ResizeConfig) -> Result<()> {
        let resized = apply_resize(&self.rgba, width, height, resize)?;
        garb::bytes::rgba_to_bgra(resized.as_raw(), dst).context("pixel format conversion failed")
    }
}

fn apply_resize(src: &RgbaImage, width: u32, height: u32, config: &ResizeConfig) -> Result<RgbaImage> {
    let (src_w, src_h) = src.dimensions();
    if (src_w, src_h) == (width, height) {
        return Ok(src.clone());
    }

    let filter = config.filter.into();
    match config.strategy {
        ResizeStrategy::No => resize_no(src, width, height, config.fill_color),
        ResizeStrategy::Crop => resize_crop(src, width, height, config.crop_gravity, filter),
        ResizeStrategy::Fit => resize_fit(src, width, height, config.fill_color, filter),
        ResizeStrategy::Stretch => resize_stretch(src, width, height, filter),
    }
}

fn fast_resize(src: &RgbaImage, width: u32, height: u32, options: &ResizeOptions) -> Result<RgbaImage> {
    let (src_w, src_h) = src.dimensions();
    let src_ref = ImageRef::new(src_w, src_h, src.as_raw(), PixelType::U8x4).context("failed to create image reference")?;

    let mut dst = Image::new(width, height, PixelType::U8x4);
    let mut resizer = Resizer::new();
    resizer.resize(&src_ref, &mut dst, Some(options)).context("resize operations failed")?;
    resizer.reset_internal_buffers();

    RgbaImage::from_raw(width, height, dst.into_vec()).context("failed to construct RGBA image after resize")
}

fn resize_no(src: &RgbaImage, width: u32, height: u32, fill_color: [u8; 4]) -> Result<RgbaImage> {
    let (src_w, src_h) = src.dimensions();

    let crop = if src_w > width || src_h > height {
        let x = src_w.saturating_sub(width) / 2;
        let y = src_h.saturating_sub(height) / 2;
        src.view(x, y, width.min(src_w), height.min(src_h)).to_image()
    } else {
        src.view(0, 0, src_w, src_h).to_image()
    };

    let (cw, ch) = crop.dimensions();
    let ox = (width.saturating_sub(cw) / 2) as i64;
    let oy = (height.saturating_sub(ch) / 2) as i64;

    let mut canvas = RgbaImage::from_pixel(width, height, Rgba(fill_color));
    image::imageops::overlay(&mut canvas, &crop, ox, oy);

    Ok(canvas)
}

fn resize_crop(src: &RgbaImage, width: u32, height: u32, gravity: CropGravity, filter: FilterType) -> Result<RgbaImage> {
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter)).fit_into_destination(Some(gravity.as_centering()));
    fast_resize(src, width, height, &options)
}

fn resize_fit(src: &RgbaImage, width: u32, height: u32, fill_color: [u8; 4], filter: FilterType) -> Result<RgbaImage> {
    let (src_w, src_h) = src.dimensions();

    let scale = (f64::from(width) / f64::from(src_w)).min(f64::from(height) / f64::from(src_h));
    let trg_w = ((f64::from(src_w) * scale) as u32).max(1);
    let trg_h = ((f64::from(src_h) * scale) as u32).max(1);

    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));
    let resized = fast_resize(src, trg_w, trg_h, &options)?;

    let ox = (width.saturating_sub(trg_w) / 2) as i64;
    let oy = (height.saturating_sub(trg_h) / 2) as i64;

    let mut canvas = RgbaImage::from_pixel(width, height, Rgba(fill_color));
    image::imageops::overlay(&mut canvas, &resized, ox, oy);

    Ok(canvas)
}

fn resize_stretch(src: &RgbaImage, width: u32, height: u32, filter: FilterType) -> Result<RgbaImage> {
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));
    fast_resize(src, width, height, &options)
}
