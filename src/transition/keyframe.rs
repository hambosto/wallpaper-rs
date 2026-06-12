const SAMPLE_TABLE_SIZE: usize = 20;
const NEWTON_ITERATIONS: usize = 4;
const NEWTON_MIN_SLOPE: f32 = 0.001;
const SUBDIVISION_PRECISION: f32 = 1e-7;
const SUBDIVISION_MAX_ITERATIONS: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct BezierCurve {
    sample_table: [f32; SAMPLE_TABLE_SIZE],
    p1: (f32, f32),
    p2: (f32, f32),
}

impl BezierCurve {
    #[inline]
    fn coeff_a(x1: f32, x2: f32) -> f32 {
        1.0 - 3.0 * x2 + 3.0 * x1
    }
    #[inline]
    fn coeff_b(x1: f32, x2: f32) -> f32 {
        3.0 * x2 - 6.0 * x1
    }
    #[inline]
    fn coeff_c(x1: f32) -> f32 {
        3.0 * x1
    }

    #[inline]
    fn cubic_at(t: f32, x1: f32, x2: f32) -> f32 {
        ((Self::coeff_a(x1, x2) * t + Self::coeff_b(x1, x2)) * t + Self::coeff_c(x1)) * t
    }

    #[inline]
    fn cubic_slope(t: f32, x1: f32, x2: f32) -> f32 {
        3.0 * Self::coeff_a(x1, x2) * t * t + 2.0 * Self::coeff_b(x1, x2) * t + Self::coeff_c(x1)
    }

    fn newton_raphson(x: f32, mut t: f32, x1: f32, x2: f32) -> f32 {
        for _ in 0..NEWTON_ITERATIONS {
            let slope = Self::cubic_slope(t, x1, x2);
            if slope == 0.0 {
                break;
            }
            t -= (Self::cubic_at(t, x1, x2) - x) / slope;
        }
        t
    }

    fn binary_subdivide(x: f32, mut lo: f32, mut hi: f32, x1: f32, x2: f32) -> f32 {
        let mut t = 0.0;
        for _ in 0..SUBDIVISION_MAX_ITERATIONS {
            t = (lo + hi) * 0.5;
            let dx = Self::cubic_at(t, x1, x2) - x;
            if dx.abs() <= SUBDIVISION_PRECISION {
                break;
            }
            if dx > 0.0 {
                hi = t;
            } else {
                lo = t;
            }
        }
        t
    }

    fn t_for_x(&self, x: f32) -> f32 {
        let step = 1.0 / (SAMPLE_TABLE_SIZE as f32 - 1.0);
        let mut interval_start = 0.0_f32;
        let mut sample = 1;

        while sample < SAMPLE_TABLE_SIZE - 1 && self.sample_table[sample] <= x {
            interval_start += step;
            sample += 1;
        }
        sample -= 1;

        let lo = self.sample_table[sample];
        let hi = self.sample_table[sample + 1];
        let guess = interval_start + (x - lo) / (hi - lo) * step;

        let slope = Self::cubic_slope(guess, self.p1.0, self.p2.0);
        if slope >= NEWTON_MIN_SLOPE {
            Self::newton_raphson(x, guess, self.p1.0, self.p2.0)
        } else if slope == 0.0 {
            guess
        } else {
            Self::binary_subdivide(
                x,
                interval_start,
                interval_start + step,
                self.p1.0,
                self.p2.0,
            )
        }
    }

    pub fn new(p1: (f32, f32), p2: (f32, f32)) -> Self {
        let mut sample_table = [0.0_f32; SAMPLE_TABLE_SIZE];
        for (i, slot) in sample_table.iter_mut().enumerate() {
            *slot = Self::cubic_at(i as f32 / (SAMPLE_TABLE_SIZE as f32 - 1.0), p1.0, p2.0);
        }
        Self {
            sample_table,
            p1,
            p2,
        }
    }

    pub fn ease(&self, x: f64) -> f64 {
        if x == 0.0 || x == 1.0 {
            return x;
        }
        Self::cubic_at(self.t_for_x(x as f32), self.p1.1, self.p2.1) as f64
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    pub value: f32,
    pub time: f64,
    pub curve: BezierCurve,
}

impl Keyframe {
    pub fn new(value: f32, time: f64, curve: BezierCurve) -> Self {
        Self {
            value,
            time: time.max(0.0),
            curve,
        }
    }

    pub fn tween(&self, next: &Keyframe, time: f64) -> f32 {
        if time <= self.time {
            return self.value;
        }
        if time >= next.time || next.time <= self.time {
            return next.value;
        }
        let t = (time - self.time) / (next.time - self.time);
        self.value + (next.value - self.value) * self.curve.ease(t) as f32
    }
}

pub struct AnimationSequence {
    start: Keyframe,
    end: Keyframe,
    time: f64,
}

impl AnimationSequence {
    pub fn new(
        bezier: (f32, f32, f32, f32),
        duration: f32,
        start_val: f32,
        end_val: f32,
        start_time: f64,
    ) -> Self {
        let curve = BezierCurve::new((bezier.0, bezier.1), (bezier.2, bezier.3));
        let end_time = start_time + duration as f64;
        Self {
            start: Keyframe::new(start_val, start_time, curve),
            end: Keyframe::new(end_val, end_time, curve),
            time: 0.0,
        }
    }

    pub fn now(&self) -> f32 {
        self.start.tween(&self.end, self.time)
    }

    pub fn advance_to(&mut self, timestamp: f64) -> f64 {
        let duration = self.end.time;
        self.time = timestamp.clamp(0.0, duration);
        timestamp - self.time
    }

    pub fn finished(&self) -> bool {
        self.time >= self.end.time
    }
}
