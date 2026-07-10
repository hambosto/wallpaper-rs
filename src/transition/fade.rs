use super::animation::AnimationSequence;

pub(crate) struct Fade {
    seq: AnimationSequence,
    pub(super) alpha: u16,
}

impl Fade {
    pub(crate) fn new(bezier: (f32, f32, f32, f32), duration: f32) -> Self {
        Self { seq: AnimationSequence::new(bezier, duration, 0.0, 1.0, 0.0), alpha: 0 }
    }

    pub(crate) fn run(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
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
