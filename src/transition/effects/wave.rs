use crate::transition::animation::AnimationSequence;

pub(crate) struct Wave {
    seq: AnimationSequence,
    center: (f64, f64),
    sin: f64,
    cos: f64,
    scale_x: f64,
    scale_y: f64,
    circle_radius: f64,
    pub(crate) step: u8,
    width: usize,
    height: usize,
}

impl Wave {
    pub(crate) fn new(bezier: (f32, f32, f32, f32), duration: f32, step: u8, angle: f64, wave: (f32, f32), dimensions: (u32, u32)) -> Self {
        let (width, height) = (dimensions.0 as f64, dimensions.1 as f64);
        let (sin, cos) = angle.to_radians().sin_cos();
        let scale_x = wave.0 as f64;
        let scale_y = wave.1 as f64;
        let circle_radius = width.hypot(height) / 2.0;
        let r2 = circle_radius * circle_radius;

        Self {
            seq: AnimationSequence::new(bezier, duration, (sin.abs().mul_add(width, cos.abs() * height) * 2.0) as f32, (r2 * 2.0) as f32, 0.0),
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

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        let offset = self.seq.now() as f64;
        let stride = self.width * 4;
        let step = self.step;
        let r2 = self.circle_radius * self.circle_radius;
        let a = self.circle_radius * self.cos;
        let b = self.circle_radius * self.sin;

        for line in 0..self.height {
            let hy = (self.height - line) as f64 - self.center.1;
            let row = line * stride;

            let x0 = self
                .scale_y
                .mul_add(self.cos, (self.scale_y.mul_add(-self.sin, hy).mul_add(-b, r2) - offset) / a + self.center.0)
                .clamp(0.0, self.width as f64);
            let x1 = self
                .scale_y
                .mul_add(-self.cos, (self.scale_y.mul_add(self.sin, hy).mul_add(-b, r2) - offset) / a + self.center.0)
                .clamp(0.0, self.width as f64);

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

                let lhs = ry.mul_add(self.sin, -(rx * self.cos));
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
