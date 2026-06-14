use crate::config::Position;
use crate::transition::keyframe::AnimationSequence;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RadialMode {
    Grow,
    Outer,
}

pub(crate) struct Radial {
    seq: AnimationSequence,
    center: (usize, usize),
    radius: f32,
    pub(crate) step: u8,
    width: usize,
    height: usize,
    mode: RadialMode,
}

impl Radial {
    pub(crate) fn new(bezier: (f32, f32, f32, f32), duration: f32, step: u8, pos: Position, invert_y: bool, dimensions: (u32, u32), mode: RadialMode) -> Self {
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

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let r = self.radius;
        let r2 = r * r;
        let stride = self.width * 4;
        let step = self.step;
        let (cx, cy) = self.center;

        for line in 0..self.height {
            let dy = cy as f32 - line as f32;
            let dx = if r * r > dy * dy { dy.mul_add(-dy, r2).sqrt() as usize } else { 0 };

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
