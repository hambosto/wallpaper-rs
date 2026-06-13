use super::keyframe::AnimationSequence;
use crate::config::{Position, TransitionConfig, TransitionType};

pub(crate) enum Effect {
    None,
    Cleanup { step: u8 },
    Fade(Fade),
    Radial(Radial),
    Wave(Wave),
}

impl Effect {
    pub fn new(config: &TransitionConfig, dimensions: (u32, u32)) -> Self {
        match config.transition_type {
            TransitionType::None => Self::None,
            TransitionType::Simple => Self::Cleanup { step: 2 },
            TransitionType::Fade => Self::Fade(Fade::new(config.bezier, config.duration)),
            TransitionType::Grow => Self::Radial(Radial::new(config.bezier, config.duration, config.step, config.pos, config.invert_y, dimensions, RadialMode::Grow)),
            TransitionType::Outer => Self::Radial(Radial::new(config.bezier, config.duration, config.step, config.pos, config.invert_y, dimensions, RadialMode::Outer)),
            TransitionType::Wipe | TransitionType::Wave => Self::Wave(Wave::new(config.bezier, config.duration, config.step, config.angle, config.wave, dimensions)),
        }
    }

    pub fn execute(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        match self {
            Self::None => {
                canvas.copy_from_slice(target);
                return true;
            }
            Self::Cleanup { step } => {
                let step = *step;
                let mut done = true;
                for (old, new) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
                    for (o, &n) in old.iter_mut().zip(new) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                    done &= old == new;
                }
                return done;
            }
            Self::Fade(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.alpha as u8 / 4 + 4 };
            }
            Self::Radial(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.step / 4 + 4 };
            }
            Self::Wave(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.step / 4 + 4 };
            }
        }
        false
    }
}

pub(crate) struct Fade {
    seq: AnimationSequence,
    alpha: u16,
}

impl Fade {
    fn new(bezier: (f32, f32, f32, f32), duration: f32) -> Self {
        Self { seq: AnimationSequence::new(bezier, duration, 0.0, 1.0, 0.0), alpha: 0 }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let alpha = self.alpha;
        let inv = 256 - alpha;
        for (old, &new) in canvas.iter_mut().zip(target) {
            *old = (((*old as u16) * inv + (new as u16) * alpha) >> 8) as u8;
        }
        self.seq.advance_to(elapsed);
        self.alpha = (256.0 * self.seq.now() as f64) as u16;
        self.seq.finished()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RadialMode {
    Grow,
    Outer,
}

pub(crate) struct Radial {
    seq: AnimationSequence,
    center: (usize, usize),
    radius: f32,
    step: u8,
    width: usize,
    height: usize,
    mode: RadialMode,
}

impl Radial {
    fn new(bezier: (f32, f32, f32, f32), duration: f32, step: u8, pos: Position, invert_y: bool, dimensions: (u32, u32), mode: RadialMode) -> Self {
        let (w, h) = (dimensions.0 as f32, dimensions.1 as f32);
        let (cx, cy) = pos.to_pixel(dimensions, invert_y);
        let far_x = if cx < w / 2.0 { w - 1.0 - cx } else { cx };
        let far_y = if cy < h / 2.0 { h - 1.0 - cy } else { cy };
        let max_dist = far_x.hypot(far_y);

        let (start_val, end_val, radius) = match mode {
            RadialMode::Grow => (0.0, max_dist, 0.0),
            RadialMode::Outer => (max_dist, 0.0, max_dist),
        };

        Self {
            seq: AnimationSequence::new(bezier, duration, start_val, end_val, 0.0),
            center: (cx as usize, cy as usize),
            radius,
            step,
            width: dimensions.0 as usize,
            height: dimensions.1 as usize,
            mode,
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let r = self.radius;
        let r2 = r * r;
        let stride = self.width * 4;
        let step = self.step;
        let (cx, cy) = self.center;

        for line in 0..self.height {
            let dy = cy as f32 - line as f32;
            let dx = if r * r > dy * dy { (r2 - dy * dy).sqrt() as usize } else { 0 };

            let inner = cx.saturating_sub(dx) * 4..self.width.min(cx + dx) * 4;
            let row = line * stride;

            match self.mode {
                RadialMode::Grow => {
                    let slice = &mut canvas[row + inner.start..row + inner.end];
                    let tgt = &target[row + inner.start..row + inner.end];
                    for (o, &n) in slice.iter_mut().zip(tgt) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                }
                RadialMode::Outer => {
                    let left = &mut canvas[row..row + inner.start];
                    let left_tgt = &target[row..row + inner.start];
                    for (o, &n) in left.iter_mut().zip(left_tgt) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                    let right = &mut canvas[row + inner.end..row + stride];
                    let right_tgt = &target[row + inner.end..row + stride];
                    for (o, &n) in right.iter_mut().zip(right_tgt) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.radius = self.seq.now();
        self.seq.finished()
    }
}

pub(crate) struct Wave {
    seq: AnimationSequence,
    center: (f64, f64),
    sin: f64,
    cos: f64,
    scale_x: f64,
    scale_y: f64,
    circle_radius: f64,
    step: u8,
    width: usize,
    height: usize,
}

impl Wave {
    fn new(bezier: (f32, f32, f32, f32), duration: f32, step: u8, angle: f64, wave: (f32, f32), dimensions: (u32, u32)) -> Self {
        let (width, height) = (dimensions.0 as f64, dimensions.1 as f64);
        let (sin, cos) = angle.to_radians().sin_cos();
        let scale_x = wave.0 as f64;
        let scale_y = wave.1 as f64;
        let circle_radius = width.hypot(height) / 2.0;
        let r2 = circle_radius * circle_radius;

        Self {
            seq: AnimationSequence::new(bezier, duration, ((sin.abs() * width + cos.abs() * height) * 2.0) as f32, (r2 * 2.0) as f32, 0.0),
            center: (width / 2.0, height / 2.0),
            sin,
            cos,
            scale_x,
            scale_y,
            circle_radius,
            step,
            width: dimensions.0 as usize,
            height: dimensions.1 as usize,
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let offset = self.seq.now() as f64;
        let stride = self.width * 4;
        let step = self.step;
        let r2 = self.circle_radius * self.circle_radius;
        let a = self.circle_radius * self.cos;
        let b = self.circle_radius * self.sin;

        for line in 0..self.height {
            let hy = (self.height - line) as f64 - self.center.1;
            let row = line * stride;

            let x0 = ((r2 - (hy - self.scale_y * self.sin) * b - offset) / a + self.center.0 + self.scale_y * self.cos).clamp(0.0, self.width as f64);
            let x1 = ((r2 - (hy + self.scale_y * self.sin) * b - offset) / a + self.center.0 - self.scale_y * self.cos).clamp(0.0, self.width as f64);

            let (primary_begin, primary_end) = if a.is_sign_negative() { (0, x0 as usize * 4) } else { (x0 as usize * 4, stride) };
            let primary = &mut canvas[row + primary_begin..row + primary_end];
            let primary_tgt = &target[row + primary_begin..row + primary_end];

            for (o, &n) in primary.iter_mut().zip(primary_tgt) {
                let delta = step.min(o.abs_diff(n));
                *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
            }

            let (band_begin, band_end) = if x0 < x1 { (x0 as usize * 4, x1 as usize * 4) } else { (x1 as usize * 4, x0 as usize * 4) };
            for col in (band_begin..band_end).step_by(4) {
                let x = col as f64 / 4.0;
                let y = line as f64;

                let rx = x - self.center.0;
                let ry = y - self.center.1;

                let lhs = ry * self.sin - rx * self.cos;
                let wave = ((rx * self.sin + ry * self.cos) / self.scale_x).sin() * self.scale_y;

                if lhs <= wave - self.circle_radius + offset / self.circle_radius {
                    let i = row + col;
                    let pixel = &mut canvas[i..i + 4];
                    let target_pixel = &target[i..i + 4];

                    for (o, &n) in pixel.iter_mut().zip(target_pixel) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
