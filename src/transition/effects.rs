use super::keyframe::AnimationSequence;
use crate::config::{Position, TransitionConfig};

pub(crate) enum Effect {
    None,
    Simple(Simple),
    Fade(Fade),
    Grow(Grow),
    Outer(Outer),
    Wave(Wave),
}

impl Effect {
    pub fn new(config: &TransitionConfig, dimensions: (u32, u32)) -> Self {
        use crate::config::TransitionType as T;
        match config.r#type {
            T::None => Self::None,
            T::Simple => Self::Simple(Simple::new(2)),
            T::Fade => Self::Fade(Fade::new(config.bezier, config.duration)),
            T::Grow => Self::Grow(Grow::new(
                config.bezier,
                config.duration,
                config.step,
                config.pos,
                config.invert_y,
                dimensions,
            )),
            T::Outer => Self::Outer(Outer::new(
                config.bezier,
                config.duration,
                config.step,
                config.pos,
                config.invert_y,
                dimensions,
            )),
            T::Wipe | T::Wave => Self::Wave(Wave::new(
                config.bezier,
                config.duration,
                config.step,
                config.angle,
                config.wave,
                dimensions,
            )),
        }
    }

    /// Advance the transition by one frame. Returns `true` when fully done.
    pub fn execute(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let phase_done = match self {
            Self::None => {
                canvas.copy_from_slice(target);
                true
            }
            Self::Simple(e) => e.run(canvas, target),
            Self::Fade(e) => e.run(canvas, target, elapsed),
            Self::Grow(e) => e.run(canvas, target, elapsed),
            Self::Outer(e) => e.run(canvas, target, elapsed),
            Self::Wave(e) => e.run(canvas, target, elapsed),
        };

        if !phase_done {
            return false;
        }

        let cleanup_step = match self {
            Self::None | Self::Simple(_) => return true,
            Self::Fade(e) => e.step as u8 / 4 + 4,
            Self::Grow(e) => e.step / 4 + 4,
            Self::Outer(e) => e.step / 4 + 4,
            Self::Wave(e) => e.step / 4 + 4,
        };
        *self = Self::Simple(Simple::new(cleanup_step));
        false
    }
}

pub(crate) struct Simple {
    step: u8,
}

impl Simple {
    fn new(step: u8) -> Self {
        Self { step: step }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8]) -> bool {
        let step = self.step;
        let mut done = true;
        for (old, new) in canvas.iter_mut().zip(target) {
            let delta = step.min(old.abs_diff(*new));
            *old = if *old > *new {
                old.wrapping_sub(delta)
            } else {
                old.wrapping_add(delta)
            };
            done &= *old == *new;
        }
        done
    }
}

pub(crate) struct Fade {
    seq: AnimationSequence,
    step: u16,
}

impl Fade {
    fn new(bezier: (f32, f32, f32, f32), duration: f32) -> Self {
        let seq = super::keyframe::bezier_seq(bezier, duration, 0.0, 1.0, 0.0);
        Self { seq, step: 0 }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let alpha = self.step;
        let inv = 256 - alpha;
        for (old, new) in canvas.iter_mut().zip(target) {
            *old = ((*old as u16 * inv + *new as u16 * alpha) >> 8) as u8;
        }
        self.step = (256.0 * self.seq.now() as f64).trunc() as u16;
        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}

pub(crate) struct Grow {
    seq: AnimationSequence,
    center_x: usize,
    center_y: usize,
    dist_center: f32,
    step: u8,
    width: usize,
    height: usize,
}

impl Grow {
    fn new(
        bezier: (f32, f32, f32, f32),
        duration: f32,
        step: u8,
        pos: Position,
        invert_y: bool,
        dimensions: (u32, u32),
    ) -> Self {
        let (w, h) = (dimensions.0 as f32, dimensions.1 as f32);
        let (cx, cy) = pos.to_pixel(dimensions, invert_y);
        let far_x = if cx < w / 2.0 { w - 1.0 - cx } else { cx };
        let far_y = if cy < h / 2.0 { h - 1.0 - cy } else { cy };
        let seq = super::keyframe::bezier_seq(bezier, duration, 0.0, far_x.hypot(far_y), 0.0);
        Self {
            seq,
            center_x: cx as usize,
            center_y: cy as usize,
            dist_center: 0.0,
            step,
            width: dimensions.0 as usize,
            height: dimensions.1 as usize,
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let r = self.dist_center;
        let stride = self.width * 4;
        let step = self.step;
        let blend = |c: &mut [u8], t: &[u8]| {
            for (old, new) in c.iter_mut().zip(t) {
                let delta = step.min(old.abs_diff(*new));
                *old = if *old > *new {
                    old.wrapping_sub(delta)
                } else {
                    old.wrapping_add(delta)
                };
            }
        };

        let line_begin = self.center_y.saturating_sub(r as usize);
        let line_end = self.height.min(self.center_y + r as usize);
        for line in line_begin..line_end {
            let dy = (self.center_y as f32 - line as f32).abs();
            let reach = if r > dy {
                (r * r - dy * dy).sqrt() as usize
            } else {
                0
            };
            let col_begin = self.center_x.saturating_sub(reach) * 4;
            let col_end = self.width.min(self.center_x + reach) * 4;
            let row = line * stride;
            for col in (col_begin..col_end).step_by(4) {
                let i = row + col;
                if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                    blend(c, t);
                }
            }
        }

        self.dist_center = self.seq.now();
        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}

pub(crate) struct Outer {
    seq: AnimationSequence,
    center_x: usize,
    center_y: usize,
    dist_center: f32,
    step: u8,
    width: usize,
    height: usize,
}

impl Outer {
    fn new(
        bezier: (f32, f32, f32, f32),
        duration: f32,
        step: u8,
        pos: Position,
        invert_y: bool,
        dimensions: (u32, u32),
    ) -> Self {
        let (w, h) = (dimensions.0 as f32, dimensions.1 as f32);
        let (cx, cy) = pos.to_pixel(dimensions, invert_y);
        let far_x = if cx < w / 2.0 { w - 1.0 - cx } else { cx };
        let far_y = if cy < h / 2.0 { h - 1.0 - cy } else { cy };
        let dist_start = far_x.hypot(far_y);
        let seq = super::keyframe::bezier_seq(bezier, duration, dist_start, 0.0, 0.0);
        Self {
            seq,
            center_x: cx as usize,
            center_y: cy as usize,
            dist_center: dist_start,
            step,
            width: dimensions.0 as usize,
            height: dimensions.1 as usize,
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let r = self.dist_center;
        let stride = self.width * 4;
        let step = self.step;
        let blend = |c: &mut [u8], t: &[u8]| {
            for (old, new) in c.iter_mut().zip(t) {
                let delta = step.min(old.abs_diff(*new));
                *old = if *old > *new {
                    old.wrapping_sub(delta)
                } else {
                    old.wrapping_add(delta)
                };
            }
        };

        for line in 0..self.height {
            let dy = (self.center_y as f32 - line as f32).abs();
            let reach = if r > dy {
                (r * r - dy * dy).sqrt() as usize
            } else {
                0
            };
            let inner_begin = self.center_x.saturating_sub(reach) * 4;
            let inner_end = self.width.min(self.center_x + reach) * 4;
            let row = line * stride;
            for col in (0..inner_begin).chain(inner_end..stride).step_by(4) {
                let i = row + col;
                if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                    blend(c, t);
                }
            }
        }

        self.dist_center = self.seq.now();
        self.seq.advance_to(elapsed);
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
    a: f64,
    b: f64,
    step: u8,
    width: usize,
    height: usize,
}

impl Wave {
    fn new(
        bezier: (f32, f32, f32, f32),
        duration: f32,
        step: u8,
        angle: f64,
        wave: (f32, f32),
        dimensions: (u32, u32),
    ) -> Self {
        let width = dimensions.0 as f64;
        let height = dimensions.1 as f64;
        let center = (width / 2.0, height / 2.0);
        let (sin, cos) = angle.to_radians().sin_cos();
        let (scale_x, scale_y) = (wave.0 as f64, wave.1 as f64);
        let circle_radius = (width.powi(2) + height.powi(2)).sqrt() / 2.0;
        let a = circle_radius * cos;
        let b = circle_radius * sin;
        let offset = (sin.abs() * width + cos.abs() * height) * 2.0;
        let max_offset = circle_radius.powi(2) * 2.0;
        let seq =
            super::keyframe::bezier_seq(bezier, duration, offset as f32, max_offset as f32, 0.0);
        Self {
            seq,
            center,
            sin,
            cos,
            scale_x,
            scale_y,
            circle_radius,
            a,
            b,
            step,
            width: dimensions.0 as usize,
            height: dimensions.1 as usize,
        }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let offset = self.seq.now() as f64;
        let stride = self.width * 4;
        let step = self.step;
        let blend = |c: &mut [u8], t: &[u8]| {
            for (old, new) in c.iter_mut().zip(t) {
                let delta = step.min(old.abs_diff(*new));
                *old = if *old > *new {
                    old.wrapping_sub(delta)
                } else {
                    old.wrapping_add(delta)
                };
            }
        };
        let is_low = |x: f64, y: f64| -> bool {
            let (rx, ry) = (x - self.center.0, y - self.center.1);
            let lhs = ry * self.sin - rx * self.cos;
            let f = ((rx * self.sin + ry * self.cos) / self.scale_x).sin() * self.scale_y;
            lhs <= f - self.circle_radius + offset / self.circle_radius
        };

        for line in 0..self.height {
            let hy = (self.height - line) as f64 - self.center.1;
            let row = line * stride;

            let x0 =
                ((self.circle_radius.powi(2) - (hy - self.scale_y * self.sin) * self.b - offset)
                    / self.a
                    + self.center.0
                    + self.scale_y * self.cos)
                    .clamp(0.0, self.width as f64);
            let x1 =
                ((self.circle_radius.powi(2) - (hy + self.scale_y * self.sin) * self.b - offset)
                    / self.a
                    + self.center.0
                    - self.scale_y * self.cos)
                    .clamp(0.0, self.width as f64);

            let (primary_begin, primary_end) = if self.a.is_sign_negative() {
                (0, x0 as usize * 4)
            } else {
                (x0 as usize * 4, stride)
            };
            for col in (primary_begin..primary_end).step_by(4) {
                let i = row + col;
                if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                    blend(c, t);
                }
            }

            let (band_begin, band_end) = if x0 < x1 {
                (x0 as usize * 4, x1 as usize * 4)
            } else {
                (x1 as usize * 4, x0 as usize * 4)
            };
            for col in (band_begin..band_end).step_by(4) {
                if is_low(col as f64 / 4.0, line as f64) {
                    let i = row + col;
                    if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                        blend(c, t);
                    }
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
