use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, DynamicImage, Frame, GenericImageView, ImageFormat, ImageReader, RgbaImage};
use jxl_oxide::integration::JxlDecoder;

use crate::config::{CropGravity, ResizeConfig, ResizeStrategy};

#[derive(Clone)]
pub struct AnimatedFrame {
    pub pixels: Vec<u8>,
    pub delay_ms: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Render {
    image: DynamicImage,
    frames: Option<Vec<AnimatedFrame>>,
}

impl Render {
    pub fn new(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("cannot read file: {}", path.display()))?;

        let reader = ImageReader::new(Cursor::new(&bytes)).with_guessed_format().context("cannot detect image format")?;

        if reader.format().is_some() {
            let img = reader.decode().with_context(|| format!("cannot decode image: {}", path.display()))?;
            let frames = extract_animation_frames(&bytes);
            return Ok(Self { image: img, frames });
        }

        if let Ok(dec) = JxlDecoder::new(Cursor::new(&bytes)) {
            let img = DynamicImage::from_decoder(dec).context("cannot decode JPEG-XL image")?;
            return Ok(Self { image: img, frames: None });
        }

        anyhow::bail!("unsupported image format")
    }
}

impl Render {
    pub fn animation_frames(&self) -> Option<&[AnimatedFrame]> {
        self.frames.as_deref()
    }

    pub fn render(&self, width: u32, height: u32, dst: &mut [u8], resize: &ResizeConfig) -> Result<()> {
        let rgba = self.image.to_rgba8();
        let resized = apply_resize(&rgba, width, height, resize)?;
        rgba_to_xrgb(resized.as_raw(), dst);
        Ok(())
    }

    pub fn render_animation_frames(&self, target_w: u32, target_h: u32, frames: &[AnimatedFrame], resize: &ResizeConfig) -> Result<Vec<AnimatedFrame>> {
        frames
            .iter()
            .map(|frame| {
                let rgba = RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone()).context("invalid animation frame buffer")?;
                let resized = apply_resize(&rgba, target_w, target_h, resize)?;
                let mut xrgb = vec![0u8; (target_w * target_h * 4) as usize];
                rgba_to_xrgb(resized.as_raw(), &mut xrgb);
                Ok(AnimatedFrame { pixels: xrgb, delay_ms: frame.delay_ms, width: target_w, height: target_h })
            })
            .collect()
    }
}

fn apply_resize(rgba: &RgbaImage, width: u32, height: u32, config: &ResizeConfig) -> Result<RgbaImage> {
    let filter = config.filter.into();
    match config.strategy {
        ResizeStrategy::No => resize_no(rgba, width, height, config.fill_color),
        ResizeStrategy::Crop => resize_crop(rgba, width, height, config.crop_gravity, filter),
        ResizeStrategy::Fit => resize_fit(rgba, width, height, config.fill_color, filter),
        ResizeStrategy::Stretch => resize_stretch(rgba, width, height, filter),
    }
}

fn fast_resize(rgba: &RgbaImage, width: u32, height: u32, options: &ResizeOptions) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();
    let src = ImageRef::new(src_w, src_h, rgba.as_raw(), PixelType::U8x4).context("failed to create image reference")?;
    let mut dst = Image::new(width, height, PixelType::U8x4);
    Resizer::new().resize(&src, &mut dst, Some(options)).context("resize operation failed")?;
    RgbaImage::from_raw(width, height, dst.into_vec()).context("failed to construct RGBA image after resize")
}

fn resize_no(rgba: &RgbaImage, width: u32, height: u32, fill_color: [u8; 4]) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();
    let cropped = if src_w > width || src_h > height {
        let x = src_w.saturating_sub(width) / 2;
        let y = src_h.saturating_sub(height) / 2;
        rgba.view(x, y, width.min(src_w), height.min(src_h)).to_image()
    } else {
        rgba.clone()
    };
    let (cw, ch) = cropped.dimensions();
    let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba(fill_color));
    image::imageops::overlay(&mut canvas, &cropped, (width.saturating_sub(cw) / 2) as i64, (height.saturating_sub(ch) / 2) as i64);
    Ok(canvas)
}

fn resize_crop(rgba: &RgbaImage, width: u32, height: u32, gravity: CropGravity, filter: FilterType) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();
    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter)).fit_into_destination(Some(gravity.as_centering()));
    fast_resize(rgba, width, height, &options)
}

fn resize_fit(rgba: &RgbaImage, width: u32, height: u32, fill_color: [u8; 4], filter: FilterType) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();
    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }

    let scale = (width as f32 / src_w as f32).min(height as f32 / src_h as f32);
    let trg_w = ((src_w as f32 * scale) as u32).max(1);
    let trg_h = ((src_h as f32 * scale) as u32).max(1);

    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));
    let resized = fast_resize(rgba, trg_w, trg_h, &options)?;

    let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba(fill_color));
    image::imageops::overlay(&mut canvas, &resized, (width.saturating_sub(trg_w) / 2) as i64, (height.saturating_sub(trg_h) / 2) as i64);
    Ok(canvas)
}

fn resize_stretch(rgba: &RgbaImage, width: u32, height: u32, filter: FilterType) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();
    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));
    fast_resize(rgba, width, height, &options)
}

fn extract_animation_frames(bytes: &[u8]) -> Option<Vec<AnimatedFrame>> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?;

    let frames: Vec<Frame> = match reader.format()? {
        ImageFormat::Gif => GifDecoder::new(Cursor::new(bytes)).ok()?.into_frames().collect_frames().ok()?,
        ImageFormat::WebP => {
            let dec = WebPDecoder::new(Cursor::new(bytes)).ok()?;
            if !dec.has_animation() {
                return None;
            }
            dec.into_frames().collect_frames().ok()?
        }
        ImageFormat::Png => {
            let dec = PngDecoder::new(Cursor::new(bytes)).ok()?;
            if !dec.is_apng().ok()? {
                return None;
            }
            dec.apng().ok()?.into_frames().collect_frames().ok()?
        }
        _ => return None,
    };

    let result: Vec<AnimatedFrame> = frames
        .into_iter()
        .map(|frame| {
            let (numer, denom) = frame.delay().numer_denom_ms();
            let delay_ms = numer.checked_div(denom).map(|d| d.max(1)).unwrap_or(100);
            let rgba = frame.into_buffer();
            let (w, h) = rgba.dimensions();
            AnimatedFrame { pixels: rgba.into_raw(), delay_ms, width: w, height: h }
        })
        .collect();

    if result.len() > 1 {
        tracing::info!(total_frames = result.len(), "animation frame extraction complete");
        Some(result)
    } else {
        None
    }
}

#[inline]
fn rgba_to_xrgb(src: &[u8], dst: &mut [u8]) {
    for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
        d[0] = s[2];
        d[1] = s[1];
        d[2] = s[0];
        d[3] = s[3];
    }
}
