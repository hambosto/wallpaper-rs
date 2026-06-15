mod fade;
mod radial;
mod wave;

use fade::Fade;
use radial::{Radial, RadialMode};
use wave::Wave;

use crate::config::{TransitionConfig, TransitionType};

pub(crate) enum Effect {
    None,
    Cleanup { step: u8 },
    Fade(Fade),
    Radial(Radial),
    Wave(Wave),
}

impl Effect {
    pub(crate) fn new(config: &TransitionConfig, dimensions: (u32, u32)) -> Self {
        match config.transition_type {
            TransitionType::None => Self::None,
            TransitionType::Simple => Self::Cleanup { step: 2 },
            TransitionType::Fade => Self::Fade(Fade::new(config.fade.bezier, config.duration)),
            TransitionType::Grow => Self::Radial(Radial::new(config.radial.bezier, config.duration, config.radial.step, config.radial.pos, config.radial.invert_y, dimensions, RadialMode::Grow)),
            TransitionType::Outer => Self::Radial(Radial::new(config.radial.bezier, config.duration, config.radial.step, config.radial.pos, config.radial.invert_y, dimensions, RadialMode::Outer)),
            TransitionType::Wipe | TransitionType::Wave => Self::Wave(Wave::new(config.wave.bezier, config.duration, config.wave.step, config.wave.angle, config.wave.wave, dimensions)),
        }
    }

    pub(crate) fn execute(&mut self, canvas: &mut [u8], target: &[u8], elapsed: f64) -> bool {
        match self {
            Self::None => {
                canvas.copy_from_slice(target);
                return true;
            }
            Self::Cleanup { step } => {
                let step = *step;
                let mut done = true;
                for (old, new) in canvas.chunks_exact_mut(4).zip(target.chunks_exact(4)) {
                    for (o, &n) in old.iter_mut().zip(new) {
                        let delta = step.min(o.abs_diff(n));
                        *o = if *o > n { o.wrapping_sub(delta) } else { o.wrapping_add(delta) };
                    }
                    done &= old == new;
                }
                return done;
            }
            Self::Fade(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.alpha as u8 / 4 + 4 };
            }
            Self::Radial(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.step / 4 + 4 };
            }
            Self::Wave(e) => {
                if !e.run(canvas, target, elapsed) {
                    return false;
                }
                *self = Self::Cleanup { step: e.step / 4 + 4 };
            }
        }
        false
    }
}
