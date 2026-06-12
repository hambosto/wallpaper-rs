use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use fast_image_resize::images::Image;
use fast_image_resize::images::ImageRef;
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::imageops::FilterType::Lanczos3;
use image::{AnimationDecoder, DynamicImage, GenericImageView, ImageReader, RgbaImage};
use jxl_oxide::integration::JxlDecoder;
use resvg::tiny_skia::{Color, Pixmap};
use resvg::usvg::{Options, Transform, Tree};

use crate::config::{CropGravity, ResizeConfig, ResizeStrategy};

enum DecodedImage {
    Raster(DynamicImage),
    Svg {
        tree: Box<Tree>,
        width: u32,
        height: u32,
    },
}

#[derive(Clone)]
pub struct AnimatedFrame {
    pub pixels: Vec<u8>,
    pub delay_ms: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Render {
    image: DecodedImage,
    frames: Option<Vec<AnimatedFrame>>,
}

impl Render {
    pub fn new(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read file: {}", path.display()))?;

        let reader = ImageReader::new(Cursor::new(&bytes))
            .with_guessed_format()
            .context("cannot detect image format")?;

        let format = reader.format();

        if let Some(_format) = format {
            let dynimage = reader
                .decode()
                .with_context(|| format!("cannot decode image: {}", path.display()))?;

            let frames = Self::extract_animation_frames(&bytes, &dynimage);

            return Ok(Self {
                image: DecodedImage::Raster(dynimage),
                frames,
            });
        }

        let cursor = Cursor::new(&bytes);
        if let Ok(jxl_decoder) = JxlDecoder::new(cursor) {
            let dynimage =
                DynamicImage::from_decoder(jxl_decoder).context("cannot decode JPEG-XL image")?;
            return Ok(Self {
                image: DecodedImage::Raster(dynimage),
                frames: None,
            });
        }

        let tree =
            Tree::from_data(&bytes, &Options::default()).context("unsupported image format")?;
        let size = tree.size();
        let width = size.width() as u32;
        let height = size.height() as u32;
        Ok(Self {
            image: DecodedImage::Svg {
                tree: Box::new(tree),
                width,
                height,
            },
            frames: None,
        })
    }

    fn extract_animation_frames(
        bytes: &[u8],
        _static: &DynamicImage,
    ) -> Option<Vec<AnimatedFrame>> {
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let format = reader.format()?;

        match format {
            image::ImageFormat::Gif => {
                let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
                let frames = decoder.into_frames().collect_frames().ok()?;
                Self::collect_frames(frames)
            }
            image::ImageFormat::WebP => {
                let decoder = WebPDecoder::new(Cursor::new(bytes)).ok()?;
                if !decoder.has_animation() {
                    return None;
                }
                let frames = decoder.into_frames().collect_frames().ok()?;
                Self::collect_frames(frames)
            }
            image::ImageFormat::Png => {
                let decoder = PngDecoder::new(Cursor::new(bytes)).ok()?;
                if !decoder.is_apng().ok()? {
                    return None;
                }
                let frames = decoder.apng().ok()?.into_frames().collect_frames().ok()?;
                Self::collect_frames(frames)
            }
            _ => None,
        }
    }

    fn collect_frames(frames: Vec<image::Frame>) -> Option<Vec<AnimatedFrame>> {
        let mut result = Vec::new();
        for frame in frames {
            let delay = frame.delay();
            let (numer, denom) = delay.numer_denom_ms();
            let delay_ms = numer.checked_div(denom).map(|d| d.max(1)).unwrap_or(100);
            let rgba = frame.into_buffer();
            let w = rgba.width();
            let h = rgba.height();
            result.push(AnimatedFrame {
                pixels: rgba.into_raw(),
                delay_ms,
                width: w,
                height: h,
            });
        }
        tracing::info!(
            total_frames = result.len(),
            "animation frame extraction complete"
        );
        if result.len() > 1 { Some(result) } else { None }
    }

    pub fn is_animated(&self) -> bool {
        self.frames.is_some()
    }

    pub fn animation_frames(&self) -> Option<&[AnimatedFrame]> {
        self.frames.as_deref()
    }

    fn to_rgba(&self, target_w: u32, target_h: u32) -> Result<RgbaImage> {
        match &self.image {
            DecodedImage::Raster(img) => {
                let (w, h) = img.dimensions();
                if (w, h) == (target_w, target_h) {
                    Ok(img.to_rgba8())
                } else {
                    Ok(img.resize_exact(target_w, target_h, Lanczos3).to_rgba8())
                }
            }
            DecodedImage::Svg { tree, .. } => {
                let size = tree.size();
                let scale = {
                    let ratio = size.width() / size.height();
                    let w = target_w as f32;
                    let h = target_h as f32;
                    let img_r = w / h;
                    if ratio < img_r {
                        h / size.height()
                    } else {
                        w / size.width()
                    }
                };
                let transform = Transform::from_scale(scale, scale);
                let render_w = (size.width() * scale) as u32;
                let render_h = (size.height() * scale) as u32;
                let mut pixmap = Pixmap::new(render_w, render_h)
                    .context("failed to create pixmap for SVG rendering")?;
                pixmap.fill(Color::TRANSPARENT);
                resvg::render(tree, transform, &mut pixmap.as_mut());
                let ts_bytes = pixmap.data();
                let rgba = RgbaImage::from_raw(render_w, render_h, ts_bytes.to_vec())
                    .context("failed to construct RGBA image from SVG pixmap")?;

                if (render_w, render_h) == (target_w, target_h) {
                    Ok(rgba)
                } else {
                    let mut canvas =
                        RgbaImage::from_pixel(target_w, target_h, image::Rgba([0, 0, 0, 255]));
                    let ox = target_w.saturating_sub(render_w) / 2;
                    let oy = target_h.saturating_sub(render_h) / 2;
                    image::imageops::overlay(&mut canvas, &rgba, ox as i64, oy as i64);
                    Ok(canvas)
                }
            }
        }
    }

    pub fn render(
        &self,
        width: u32,
        height: u32,
        dst: &mut [u8],
        resize: &ResizeConfig,
    ) -> Result<()> {
        let filter = resize.filter.into();

        let (src_w, src_h) = self.dimensions();
        let rgba = self.to_rgba(src_w, src_h)?;

        let resized = match resize.strategy {
            ResizeStrategy::No => resize_no(&rgba, width, height, resize.fill_color)?,
            ResizeStrategy::Crop => resize_crop(&rgba, width, height, resize.crop_gravity, filter)?,
            ResizeStrategy::Fit => resize_fit(&rgba, width, height, resize.fill_color, filter)?,
            ResizeStrategy::Stretch => resize_stretch(&rgba, width, height, filter)?,
        };

        let src = resized.as_raw();
        for (i, dst_pixel) in dst.chunks_exact_mut(4).enumerate() {
            let offset = i * 4;
            dst_pixel.copy_from_slice(&[
                src[offset + 2],
                src[offset + 1],
                src[offset],
                src[offset + 3],
            ]);
        }
        Ok(())
    }

    pub fn render_animation_frames(
        &self,
        target_w: u32,
        target_h: u32,
        frames: &[AnimatedFrame],
        resize: &ResizeConfig,
    ) -> Result<Vec<AnimatedFrame>> {
        let filter = resize.filter.into();
        let mut out = Vec::with_capacity(frames.len());

        for frame in frames {
            let rgba = RgbaImage::from_raw(frame.width, frame.height, frame.pixels.clone())
                .context("failed to construct RGBA image from animation frame")?;

            let resized = match resize.strategy {
                ResizeStrategy::No => resize_no(&rgba, target_w, target_h, resize.fill_color)?,
                ResizeStrategy::Crop => {
                    resize_crop(&rgba, target_w, target_h, resize.crop_gravity, filter)?
                }
                ResizeStrategy::Fit => {
                    resize_fit(&rgba, target_w, target_h, resize.fill_color, filter)?
                }
                ResizeStrategy::Stretch => resize_stretch(&rgba, target_w, target_h, filter)?,
            };

            let mut xrgb = vec![0u8; (target_w * target_h * 4) as usize];
            let src = resized.as_raw();
            for (i, dst_pixel) in xrgb.chunks_exact_mut(4).enumerate() {
                let offset = i * 4;
                dst_pixel.copy_from_slice(&[
                    src[offset + 2],
                    src[offset + 1],
                    src[offset],
                    src[offset + 3],
                ]);
            }

            out.push(AnimatedFrame {
                pixels: xrgb,
                delay_ms: frame.delay_ms,
                width: target_w,
                height: target_h,
            });
        }

        Ok(out)
    }

    fn dimensions(&self) -> (u32, u32) {
        match &self.image {
            DecodedImage::Raster(img) => img.dimensions(),
            DecodedImage::Svg { width, height, .. } => (*width, *height),
        }
    }
}

fn resize_no(rgba: &RgbaImage, width: u32, height: u32, fill_color: [u8; 4]) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();

    let crop = if src_w > width || src_h > height {
        let crop_x = src_w.saturating_sub(width) / 2;
        let crop_y = src_h.saturating_sub(height) / 2;
        rgba.view(crop_x, crop_y, width, height).to_image()
    } else {
        rgba.clone()
    };

    let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba(fill_color));
    let (cw, ch) = crop.dimensions();
    let ox = width.saturating_sub(cw) / 2;
    let oy = height.saturating_sub(ch) / 2;
    image::imageops::overlay(&mut canvas, &crop, ox as i64, oy as i64);
    Ok(canvas)
}

fn resize_crop(
    rgba: &RgbaImage,
    width: u32,
    height: u32,
    gravity: CropGravity,
    filter: FilterType,
) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();

    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }

    let raw = rgba.as_raw();

    let src = ImageRef::new(src_w, src_h, raw.as_ref(), PixelType::U8x4)
        .context("failed to create image reference for crop resize")?;

    let centering_tuple = gravity.as_centering();
    let mut dst = Image::new(width, height, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(filter))
        .fit_into_destination(Some(centering_tuple));

    resizer
        .resize(&src, &mut dst, Some(&options))
        .context("crop resize operation failed")?;

    let bytes = dst.into_vec();
    RgbaImage::from_raw(width, height, bytes)
        .context("failed to construct RGBA image after crop resize")
}

fn resize_fit(
    rgba: &RgbaImage,
    width: u32,
    height: u32,
    fill_color: [u8; 4],
    filter: FilterType,
) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();

    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }

    if src_w == width || src_h == height {
        let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba(fill_color));
        let ox = width.saturating_sub(src_w) / 2;
        let oy = height.saturating_sub(src_h) / 2;
        image::imageops::overlay(&mut canvas, rgba, ox as i64, oy as i64);
        return Ok(canvas);
    }

    let ratio = width as f32 / height as f32;
    let img_r = src_w as f32 / src_h as f32;

    let (trg_w, trg_h) = if ratio > img_r {
        let scale = height as f32 / src_h as f32;
        ((src_w as f32 * scale) as u32, height)
    } else {
        let scale = width as f32 / src_w as f32;
        (width, (src_h as f32 * scale) as u32)
    };

    let raw = rgba.as_raw();
    let src = ImageRef::new(src_w, src_h, raw.as_ref(), PixelType::U8x4)
        .context("failed to create image reference for fit resize")?;

    let mut dst = Image::new(trg_w, trg_h, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));

    resizer
        .resize(&src, &mut dst, Some(&options))
        .context("fit resize operation failed")?;

    let resized_bytes = dst.into_vec();
    let resized = RgbaImage::from_raw(trg_w, trg_h, resized_bytes)
        .context("failed to construct RGBA image after fit resize")?;

    let mut canvas = RgbaImage::from_pixel(width, height, image::Rgba(fill_color));
    let ox = width.saturating_sub(trg_w) / 2;
    let oy = height.saturating_sub(trg_h) / 2;
    image::imageops::overlay(&mut canvas, &resized, ox as i64, oy as i64);
    Ok(canvas)
}

fn resize_stretch(
    rgba: &RgbaImage,
    width: u32,
    height: u32,
    filter: FilterType,
) -> Result<RgbaImage> {
    let (src_w, src_h) = rgba.dimensions();

    if (src_w, src_h) == (width, height) {
        return Ok(rgba.clone());
    }

    let raw = rgba.as_raw();
    let src = ImageRef::new(src_w, src_h, raw.as_ref(), PixelType::U8x4)
        .context("failed to create image reference for stretch resize")?;

    let mut dst = Image::new(width, height, PixelType::U8x4);
    let mut resizer = Resizer::new();
    let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));

    resizer
        .resize(&src, &mut dst, Some(&options))
        .context("stretch resize operation failed")?;

    let bytes = dst.into_vec();
    RgbaImage::from_raw(width, height, bytes)
        .context("failed to construct RGBA image after stretch resize")
}
