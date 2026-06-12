use super::keyframe::AnimationSequence;
use crate::config::TransitionType;
use crate::config::{Position, TransitionConfig};

#[inline(always)]
fn blend_pixel(canvas: &mut [u8], target: &[u8], step: u8) {
    for (old, &new) in canvas.iter_mut().zip(target) {
        let delta = step.min(old.abs_diff(new));
        *old = if *old > new {
            old.wrapping_sub(delta)
        } else {
            old.wrapping_add(delta)
        };
    }
}

pub(crate) enum Effect {
    None,
    Simple(Simple),
    Fade(Fade),
    Radial(Radial),
    Wave(Wave),
}

impl Effect {
    pub fn new(config: &TransitionConfig, dimensions: (u32, u32)) -> Self {
        match config.transition_type {
            TransitionType::None => Self::None,
            TransitionType::Simple => Self::Simple(Simple::new(2)),
            TransitionType::Fade => Self::Fade(Fade::new(config.bezier, config.duration)),
            TransitionType::Grow => Self::Radial(Radial::new(
                config.bezier,
                config.duration,
                config.step,
                config.pos,
                config.invert_y,
                dimensions,
                RadialMode::Grow,
            )),
            TransitionType::Outer => Self::Radial(Radial::new(
                config.bezier,
                config.duration,
                config.step,
                config.pos,
                config.invert_y,
                dimensions,
                RadialMode::Outer,
            )),
            TransitionType::Wipe | TransitionType::Wave => Self::Wave(Wave::new(
                config.bezier,
                config.duration,
                config.step,
                config.angle,
                config.wave,
                dimensions,
            )),
        }
    }

    pub fn execute(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let phase_done = match self {
            Self::None => {
                canvas.copy_from_slice(target);
                true
            }
            Self::Simple(e) => e.run(canvas, target),
            Self::Fade(e) => e.run(canvas, target, elapsed),
            Self::Radial(e) => e.run(canvas, target, elapsed),
            Self::Wave(e) => e.run(canvas, target, elapsed),
        };

        if !phase_done {
            return false;
        }

        let cleanup_step = match self {
            Self::None | Self::Simple(_) => return true,
            Self::Fade(e) => e.alpha as u8 / 4 + 4,
            Self::Radial(e) => e.step / 4 + 4,
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
        Self { step }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8]) -> bool {
        let step = self.step;
        let mut done = true;
        for (c, t) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
            blend_pixel(c, t, step);
            done &= c == t;
        }
        done
    }
}
pub(crate) struct Fade {
    seq: AnimationSequence,
    alpha: u16,
}

impl Fade {
    fn new(bezier: (f32, f32, f32, f32), duration: f32) -> Self {
        let seq = AnimationSequence::new(bezier, duration, 0.0, 1.0, 0.0);
        Self { seq, alpha: 0 }
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let alpha = self.alpha;
        let inv = 256 - alpha;
        for (old, &new) in canvas.iter_mut().zip(target.iter()) {
            *old = ((*old as u16 * inv + new as u16 * alpha) >> 8) as u8;
        }
        self.alpha = (256.0 * self.seq.now() as f64) as u16;
        self.seq.advance_to(elapsed);
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
    fn new(
        bezier: (f32, f32, f32, f32),
        duration: f32,
        step: u8,
        pos: Position,
        invert_y: bool,
        dimensions: (u32, u32),
        mode: RadialMode,
    ) -> Self {
        let (w, h) = (dimensions.0 as f32, dimensions.1 as f32);
        let (cx, cy) = pos.to_pixel(dimensions, invert_y);
        let far_x = if cx < w / 2.0 { w - 1.0 - cx } else { cx };
        let far_y = if cy < h / 2.0 { h - 1.0 - cy } else { cy };
        let max_dist = far_x.hypot(far_y);

        let (start_val, end_val, radius) = match mode {
            RadialMode::Grow => (0.0, max_dist, 0.0),
            RadialMode::Outer => (max_dist, 0.0, max_dist),
        };
        let seq = AnimationSequence::new(bezier, duration, start_val, end_val, 0.0);

        Self {
            seq,
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
            let dy = (cy as f32 - line as f32).abs();
            let reach = if r > dy {
                (r2 - dy * dy).sqrt() as usize
            } else {
                0
            };

            let inner_begin = cx.saturating_sub(reach) * 4;
            let inner_end = self.width.min(cx + reach) * 4;
            let row = line * stride;

            let cols: &mut dyn Iterator<Item = usize> = match self.mode {
                RadialMode::Grow => {
                    let line_begin = cy.saturating_sub(r as usize);
                    let line_end = self.height.min(cy + r as usize);
                    if line < line_begin || line >= line_end {
                        continue;
                    }
                    &mut (inner_begin..inner_end).step_by(4)
                }
                RadialMode::Outer => &mut (0..inner_begin).chain(inner_end..stride).step_by(4),
            };

            for col in cols {
                let i = row + col;
                if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                    blend_pixel(c, t, step);
                }
            }
        }

        self.radius = self.seq.now();
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
        let scale_x = wave.0 as f64;
        let scale_y = wave.1 as f64;
        let circle_radius = width.hypot(height) / 2.0;
        let a = circle_radius * cos;
        let b = circle_radius * sin;
        let offset_start = (sin.abs() * width + cos.abs() * height) * 2.0;
        let offset_end = circle_radius * circle_radius * 2.0;
        let seq = AnimationSequence::new(
            bezier,
            duration,
            offset_start as f32,
            offset_end as f32,
            0.0,
        );
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

    #[inline]
    fn is_transitioned(&self, x: f64, y: f64, offset: f64) -> bool {
        let rx = x - self.center.0;
        let ry = y - self.center.1;
        let lhs = ry * self.sin - rx * self.cos;
        let wave = ((rx * self.sin + ry * self.cos) / self.scale_x).sin() * self.scale_y;
        lhs <= wave - self.circle_radius + offset / self.circle_radius
    }

    fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let offset = self.seq.now() as f64;
        let stride = self.width * 4;
        let step = self.step;

        for line in 0..self.height {
            let hy = (self.height - line) as f64 - self.center.1;
            let row = line * stride;

            let r2 = self.circle_radius * self.circle_radius;
            let x0 = ((r2 - (hy - self.scale_y * self.sin) * self.b - offset) / self.a
                + self.center.0
                + self.scale_y * self.cos)
                .clamp(0.0, self.width as f64);
            let x1 = ((r2 - (hy + self.scale_y * self.sin) * self.b - offset) / self.a
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
                    blend_pixel(c, t, step);
                }
            }

            let (band_begin, band_end) = if x0 < x1 {
                (x0 as usize * 4, x1 as usize * 4)
            } else {
                (x1 as usize * 4, x0 as usize * 4)
            };
            for col in (band_begin..band_end).step_by(4) {
                if self.is_transitioned(col as f64 / 4.0, line as f64, offset) {
                    let i = row + col;
                    if let (Some(c), Some(t)) = (canvas.get_mut(i..i + 4), target.get(i..i + 4)) {
                        blend_pixel(c, t, step);
                    }
                }
            }
        }

        self.seq.advance_to(elapsed);
        self.seq.finished()
    }
}
