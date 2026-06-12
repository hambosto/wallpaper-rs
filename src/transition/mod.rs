pub mod effects;
pub mod keyframe;

use std::time::Instant;

use crate::config::TransitionConfig;
use effects::Effect;

pub struct Transition {
    effect: Option<Effect>,
    target: Vec<u8>,
    start: Instant,
    width: u32,
    height: u32,
}

impl Transition {
    pub fn new(config: &TransitionConfig, dimensions: (u32, u32), target: Vec<u8>) -> Self {
        tracing::info!(
            transition_type = ?&config.transition_type,
            duration = config.duration,
            fps = config.fps,
            "creating transition"
        );
        let effect = Some(Effect::new(config, dimensions));

        Self {
            effect,
            target,
            start: Instant::now(),
            width: dimensions.0,
            height: dimensions.1,
        }
    }

    pub fn is_done(&self) -> bool {
        self.effect.is_none()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn frame(&mut self, canvas: &mut [u8]) -> bool {
        let elapsed = self.start.elapsed().as_secs_f64();

        if let Some(ref mut effect) = self.effect {
            let done = effect.execute(canvas, &self.target, elapsed);
            if done {
                tracing::info!("transition effect finished at {:.2}s", elapsed);
                self.effect = None;
                return true;
            }
        }
        false
    }
}
