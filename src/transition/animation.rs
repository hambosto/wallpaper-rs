use animato_core::Easing;

pub(crate) struct AnimationSequence {
    easing: Easing,
    start_val: f32,
    end_val: f32,
    start_time: f64,
    end_time: f64,
    time: f64,
}

impl AnimationSequence {
    pub(crate) fn new(bezier: (f32, f32, f32, f32), duration: f32, start_val: f32, end_val: f32, start_time: f64) -> Self {
        let easing = Easing::CubicBezier(bezier.0, bezier.1, bezier.2, bezier.3);
        let end_time = start_time + duration as f64;
        Self { easing, start_val, end_val, start_time, end_time, time: 0.0 }
    }

    pub(crate) fn now(&self) -> f32 {
        if self.time <= self.start_time {
            return self.start_val;
        }
        if self.time >= self.end_time {
            return self.end_val;
        }
        let t = ((self.time - self.start_time) / (self.end_time - self.start_time)) as f32;
        let eased = self.easing.apply(t);
        (self.end_val - self.start_val).mul_add(eased, self.start_val)
    }

    pub(crate) fn advance_to(&mut self, timestamp: f64) -> f64 {
        self.time = timestamp.clamp(self.start_time, self.end_time);
        timestamp - self.time
    }

    pub(crate) fn finished(&self) -> bool {
        self.time >= self.end_time
    }
}
