use std::time::Duration;

use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::SlotPool;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_shm::Format;

use crate::config::Config;
use crate::render::Render;
use crate::transition::Transition;

pub(super) struct Surface {
    layer_surface: LayerSurface,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pool: Option<SlotPool>,
    transition: Option<Transition>,
}

impl Surface {
    fn new(layer_surface: LayerSurface, width: u32, height: u32) -> Self {
        Self { layer_surface, width, height, pixels: Vec::new(), pool: None, transition: None }
    }

    fn tick(&mut self) -> bool {
        let Some(transition) = &mut self.transition else {
            return false;
        };

        let finished = transition.frame(&mut self.pixels);
        if finished {
            let (w, h) = transition.dimensions();
            tracing::info!(width = w, height = h, "transition completed");
            self.transition = None;
            self.pool = None;
        }

        !finished
    }

    fn commit(&mut self, shm: &Shm) -> Result<()> {
        let needed = self.width.saturating_mul(4).saturating_mul(self.height) as usize;
        if self.pool.as_ref().is_none_or(|p| p.len() < needed) {
            self.pool = Some(SlotPool::new(needed, shm).context("failed to allocate shm pool for commit")?);
        }
        let pool = self.pool.as_mut().context("shm pool not initialized")?;
        let (buffer, canvas) = pool
            .create_buffer(self.width.cast_signed(), self.height.cast_signed(), self.width.saturating_mul(4).cast_signed(), Format::Xrgb8888)
            .context("failed to create buffer")?;
        canvas.copy_from_slice(&self.pixels);

        let wl_surface = self.layer_surface.wl_surface();
        buffer.attach_to(wl_surface).context("failed to attach buffer")?;
        wl_surface.damage_buffer(0, 0, self.width.cast_signed(), self.height.cast_signed());

        self.layer_surface.commit();

        Ok(())
    }
}

pub(super) struct WaylandState {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    pub(super) shm: Shm,
    pending: Vec<Surface>,
    surfaces: Vec<Surface>,
    animation_token: Option<RegistrationToken>,
}

impl WaylandState {
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(global_list),
            output_state: OutputState::new(global_list, queue_handle),
            compositor: CompositorState::bind(global_list, queue_handle).context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(global_list, queue_handle).context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(global_list, queue_handle).context("wl_shm not available")?,
            pending: Vec::new(),
            surfaces: Vec::new(),
            animation_token: None,
        })
    }

    pub(super) fn create_surfaces(&mut self, queue_handle: &QueueHandle<Self>) {
        for handle in self.output_state.outputs() {
            let Some(info) = self.output_state.info(&handle) else {
                continue;
            };

            let Some((w, h)) = info
                .logical_size
                .filter(|(w, h)| *w > 0 && *h > 0)
                .or_else(|| info.modes.iter().find(|m| m.current).map(|m| m.dimensions))
            else {
                continue;
            };

            let wl_surface = self.compositor.create_surface(queue_handle);
            let layer_surface = self.layer_shell.create_layer_surface(queue_handle, wl_surface, Layer::Background, Some("wallpaper-rs"), Some(&handle));

            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();

            self.pending.push(Surface::new(layer_surface, w.cast_unsigned(), h.cast_unsigned()));
        }

        tracing::info!(count = self.pending.len(), "surfaces created");
    }

    pub(super) fn apply_wallpaper(&mut self, config: &Config, loop_handle: &LoopHandle<'_, Self>) -> Result<()> {
        if self.pending.is_empty() && self.surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        tracing::info!(image = %config.image.path.display(), "applying wallpaper");
        let renderer = Render::new(&config.image.path)?;

        self.stop_animation(loop_handle);

        if self.surfaces.is_empty() {
            self.surfaces = std::mem::take(&mut self.pending);
            tracing::info!(count = self.surfaces.len(), "initial wallpaper (transitioning from black)");
        } else {
            tracing::info!(count = self.surfaces.len(), "transitioning to new wallpaper");
        }

        for surface in &mut self.surfaces {
            let buffer_size = surface.width.saturating_mul(surface.height).saturating_mul(4) as usize;
            surface.pixels = vec![0u8; buffer_size];

            let mut target = vec![0u8; buffer_size];
            renderer.render(surface.width, surface.height, &mut target, &config.resize)?;
            surface.transition = Some(Transition::new(&config.transition, (surface.width, surface.height), target));
        }

        self.start_animation(config, loop_handle)?;

        Ok(())
    }

    fn start_animation(&mut self, config: &Config, loop_handle: &LoopHandle<'_, Self>) -> Result<()> {
        let interval_ms = 1000.0 / f64::from(config.transition.fps);
        let interval = Duration::from_millis(interval_ms as u64);

        tracing::info!(fps = config.transition.fps, interval_ms = interval.as_millis(), "animation timer started");

        let token = loop_handle
            .insert_source(Timer::from_duration(interval), move |_, _, state: &mut Self| match state.tick_and_commit() {
                Ok(true) => TimeoutAction::ToDuration(interval),
                Ok(false) => {
                    state.animation_token = None;
                    TimeoutAction::Drop
                }
                Err(err) => {
                    tracing::error!(?err, "frame commit failed; stopping animation");
                    state.animation_token = None;
                    TimeoutAction::Drop
                }
            })
            .map_err(|err| anyhow::anyhow!("failed to insert animation timer: {err}"))?;

        self.animation_token = Some(token);

        Ok(())
    }

    fn stop_animation(&mut self, loop_handle: &LoopHandle<'_, Self>) {
        if let Some(token) = self.animation_token.take() {
            loop_handle.remove(token);
        }

        for surface in &mut self.surfaces {
            if surface.transition.as_ref().is_some_and(|t| !t.is_done()) {
                tracing::info!(width = surface.width, height = surface.height, "interrupting active transition");
            }
            surface.transition = None;
            surface.pool = None;
            surface.pixels = Vec::new();
        }
    }

    fn tick_and_commit(&mut self) -> Result<bool> {
        let mut running = false;

        for surface in &mut self.surfaces {
            if surface.tick() {
                running = true;
            }
            surface.commit(&self.shm)?;
        }

        if !running {
            for surface in &mut self.surfaces {
                surface.pool = None;
                surface.pixels = Vec::new();
            }
        }

        Ok(running)
    }
}
