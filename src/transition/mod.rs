pub(crate) mod animation;
mod effects;

use std::time::Instant;

use effects::Effect;

use crate::config::TransitionConfig;

pub(crate) struct Transition {
    effect: Option<Effect>,
    target: Vec<u8>,
    start: Instant,
    width: u32,
    height: u32,
}

impl Transition {
    pub(crate) fn new(config: &TransitionConfig, dimensions: (u32, u32), target: Vec<u8>) -> Self {
        tracing::info!(
            transition_type = ?config.transition_type,
            duration = config.duration,
            fps = config.fps,
            "creating transition",
        );
        Self { effect: Some(Effect::new(config, dimensions)), target, start: Instant::now(), width: dimensions.0, height: dimensions.1 }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.effect.is_none()
    }

    pub(crate) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(crate) fn frame(&mut self, canvas: &mut [u8]) -> bool {
        let Some(effect) = self.effect.as_mut() else {
            return true;
        };

        let elapsed = self.start.elapsed().as_secs_f64();
        let done = effect.execute(canvas, &self.target, elapsed);
        if done {
            tracing::info!("transition finished at {elapsed:.2}s");
            self.effect = None;
        }
        done
    }
}
