const SAMPLE_TABLE_SIZE: usize = 20;
const NEWTON_ITERATIONS: usize = 4;
const NEWTON_MIN_SLOPE: f32 = 0.001;
const SUBDIVISION_PRECISION: f32 = 0.0000001;
const SUBDIVISION_MAX_ITERATIONS: usize = 10;

#[derive(Debug, Clone, Copy)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct BezierCurve {
    sample_table: [f32; SAMPLE_TABLE_SIZE],
    p1: Vector2,
    p2: Vector2,
}

impl BezierCurve {
    #[inline]
    fn a(x1: f32, x2: f32) -> f32 {
        1.0 - 3.0 * x2 + 3.0 * x1
    }
    #[inline]
    fn b(x1: f32, x2: f32) -> f32 {
        3.0 * x2 - 6.0 * x1
    }
    #[inline]
    fn c(x1: f32) -> f32 {
        3.0 * x1
    }

    #[inline]
    fn at(t: f32, x1: f32, x2: f32) -> f32 {
        ((Self::a(x1, x2) * t + Self::b(x1, x2)) * t + Self::c(x1)) * t
    }
    #[inline]
    fn slope(t: f32, x1: f32, x2: f32) -> f32 {
        3.0 * Self::a(x1, x2) * t * t + 2.0 * Self::b(x1, x2) * t + Self::c(x1)
    }

    fn newton_raphson(x: f32, guess: f32, x1: f32, x2: f32) -> f32 {
        let mut guess = guess;
        for _ in 0..NEWTON_ITERATIONS {
            let current_slope = Self::slope(guess, x1, x2);
            if current_slope == 0.0 {
                break;
            }
            let current_x = Self::at(guess, x1, x2) - x;
            guess -= current_x / current_slope;
        }
        guess
    }

    fn binary_subdivide(x: f32, mut a: f32, mut b: f32, x1: f32, x2: f32) -> f32 {
        let mut current_x;
        let mut current_t;
        let mut i = 0;
        loop {
            current_t = a + (b - a) / 2.0;
            current_x = Self::at(current_t, x1, x2) - x;
            if current_x > 0.0 {
                b = current_t;
            } else {
                a = current_t;
            }
            i += 1;
            if current_x.abs() <= SUBDIVISION_PRECISION || i >= SUBDIVISION_MAX_ITERATIONS {
                break;
            }
        }
        current_t
    }

    fn t_for_x(&self, x: f32) -> f32 {
        let mut interval_start = 0.0;
        let mut current_sample = 1;
        let last_sample = SAMPLE_TABLE_SIZE - 1;
        let sample_step_size = 1.0 / (SAMPLE_TABLE_SIZE as f32 - 1.0);

        while current_sample != last_sample && self.sample_table[current_sample] <= x {
            interval_start += sample_step_size;
            current_sample += 1;
        }
        current_sample -= 1;

        let dist = (x - self.sample_table[current_sample])
            / (self.sample_table[current_sample + 1] - self.sample_table[current_sample]);
        let guess_for_t = interval_start + dist * sample_step_size;

        match Self::slope(guess_for_t, self.p1.x, self.p2.x) {
            s if s >= NEWTON_MIN_SLOPE => {
                Self::newton_raphson(x, guess_for_t, self.p1.x, self.p2.x)
            }
            0.0 => guess_for_t,
            _ => Self::binary_subdivide(
                x,
                interval_start,
                interval_start + sample_step_size,
                self.p1.x,
                self.p2.x,
            ),
        }
    }

    pub fn from(p1: Vector2, p2: Vector2) -> Self {
        let mut sample_table = [0.0f32; SAMPLE_TABLE_SIZE];
        for (i, slot) in sample_table.iter_mut().enumerate() {
            *slot = Self::at(i as f32 / (SAMPLE_TABLE_SIZE as f32 - 1.0), p1.x, p2.x);
        }
        BezierCurve {
            sample_table,
            p1,
            p2,
        }
    }

    pub fn y(&self, x: f64) -> f64 {
        match x {
            _ if x == 0.0 => 0.0,
            _ if x == 1.0 => 1.0,
            _ => Self::at(self.t_for_x(x as f32), self.p1.y, self.p2.y) as f64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Keyframe {
    value: f32,
    time: f64,
    function: BezierCurve,
}

impl Keyframe {
    pub fn new(value: f32, time: f64, function: BezierCurve) -> Self {
        Keyframe {
            value,
            time: time.max(0.0),
            function,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn tween_to(&self, next: &Keyframe, time: f64) -> f32 {
        match time {
            t if t < self.time => self.value,
            t if t > next.time => next.value,
            _ if next.time < self.time => next.value,
            t => {
                let t_scaled = (t - self.time) / (next.time - self.time);
                let t_eased = self.function.y(t_scaled);
                self.value + (next.value - self.value) * t_eased as f32
            }
        }
    }
}

pub struct AnimationSequence {
    sequence: [Keyframe; 2],
    keyframe: Option<usize>,
    time: f64,
}

impl AnimationSequence {
    fn update_current_keyframe(&mut self) {
        if !self.sequence.is_empty() && self.time == 0.0 {
            self.keyframe = Some(0);
            return;
        }
        if !self.sequence.is_empty() && self.time == self.duration() {
            self.keyframe = Some(self.sequence.len() - 1);
            return;
        }

        if let Some(k) = self.keyframe {
            if self.sequence[k].time() > self.time {
                for i in (0..k).rev() {
                    if self.sequence[i].time() <= self.time {
                        self.keyframe = Some(i);
                        return;
                    }
                    self.keyframe = None;
                }
            } else {
                let copy = self.keyframe;
                self.keyframe = None;
                for i in copy.unwrap_or(0)..self.sequence.len() {
                    if self.sequence[i].time() > self.time {
                        break;
                    } else {
                        self.keyframe = Some(i);
                    }
                }
            }
        } else if !self.sequence.is_empty() {
            self.keyframe = Some(0);
            self.update_current_keyframe();
        }
    }

    pub fn now(&self) -> f32 {
        match self.pair() {
            (Some(s1), Some(s2)) => s1.tween_to(s2, self.time),
            (Some(s1), None) => s1.value(),
            (None, Some(s2)) => {
                let origin = Keyframe::new(
                    0.0,
                    0.0,
                    BezierCurve::from(Vector2 { x: 0.0, y: 0.0 }, Vector2 { x: 1.0, y: 1.0 }),
                );
                origin.tween_to(s2, self.time)
            }
            _ => 0.0,
        }
    }

    fn pair(&self) -> (Option<&Keyframe>, Option<&Keyframe>) {
        match self.keyframe {
            Some(c) if c == self.sequence.len() - 1 => (Some(&self.sequence[c]), None),
            Some(c) => (Some(&self.sequence[c]), Some(&self.sequence[c + 1])),
            None if !self.sequence.is_empty() => (None, Some(&self.sequence[0])),
            None => (None, None),
        }
    }

    pub fn advance_to(&mut self, timestamp: f64) -> f64 {
        self.time = timestamp.clamp(0.0, self.duration());
        self.update_current_keyframe();
        timestamp - self.time
    }

    pub fn duration(&self) -> f64 {
        self.sequence.last().map_or(0.0, Keyframe::time)
    }

    pub fn finished(&self) -> bool {
        self.time >= self.duration()
    }
}

impl From<[Keyframe; 2]> for AnimationSequence {
    fn from(mut sequence: [Keyframe; 2]) -> Self {
        if sequence[0].time() > sequence[1].time() {
            sequence.swap(0, 1);
        } else if sequence[0].time() == sequence[1].time() {
            let bits = sequence[1].time.to_bits();
            sequence[1].time = f64::from_bits(bits + 4);
        }

        let mut me = AnimationSequence {
            sequence,
            keyframe: None,
            time: 0.0,
        };
        me.update_current_keyframe();
        me
    }
}

pub fn bezier_seq(
    bezier: (f32, f32, f32, f32),
    duration: f32,
    start: f32,
    end: f32,
    start_time: f64,
) -> AnimationSequence {
    let curve = BezierCurve::from(
        Vector2 {
            x: bezier.0,
            y: bezier.1,
        },
        Vector2 {
            x: bezier.2,
            y: bezier.3,
        },
    );
    AnimationSequence::from([
        Keyframe::new(start, start_time, curve),
        Keyframe::new(end, start_time + duration as f64, curve),
    ])
}
