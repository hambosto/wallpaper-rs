use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, Layer, LayerShell, LayerSurface};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::raw::RawPool;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::Format::Xrgb8888;
use wayland_client::protocol::wl_surface::WlSurface;

use crate::config::{ResizeConfig, TransitionConfig, TransitionType};
use crate::render::{AnimatedFrame, Render};
use crate::transition::Transition;

pub(super) struct PendingSurface {
    pub(super) layer_surface: LayerSurface,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) struct CommittedSurface {
    pub(super) layer_surface: LayerSurface,
    pub(super) pool: RawPool,
    pub(super) buffer_a: WlBuffer,
    pub(super) buffer_b: WlBuffer,
    pub(super) front: usize,
    pub(super) transition: Option<Transition>,
    pub(super) frame_callback: Option<wayland_client::protocol::wl_callback::WlCallback>,
    pub(super) anim_frames: Vec<AnimatedFrame>,
    pub(super) anim_current: usize,
    pub(super) anim_last_frame: Instant,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl CommittedSurface {
    fn buf_size(&self) -> usize {
        (self.width * 4 * self.height) as usize
    }

    fn active_buffer(&self) -> &WlBuffer {
        if self.front == 0 { &self.buffer_a } else { &self.buffer_b }
    }

    fn swap_front(&mut self) -> usize {
        self.front = 1 - self.front;
        self.front * self.buf_size()
    }

    fn present(&mut self, pixels: &[u8]) {
        let buf_size = self.buf_size();
        let offset = self.swap_front();
        let dst = self.pool.mmap();
        dst[offset..offset + pixels.len().min(buf_size)].copy_from_slice(&pixels[..pixels.len().min(buf_size)]);

        let surface = self.layer_surface.wl_surface();
        surface.attach(Some(self.active_buffer()), 0, 0);
        surface.damage_buffer(0, 0, self.width.cast_signed(), self.height.cast_signed());
        self.layer_surface.commit();
    }

    pub(super) fn tick_animation(&mut self) {
        if self.transition.is_some() || self.anim_frames.is_empty() {
            return;
        }

        let frame = &self.anim_frames[self.anim_current];
        if self.anim_last_frame.elapsed().as_millis() < frame.delay_ms as u128 {
            return;
        }

        let pixels = frame.pixels.clone();
        self.anim_current = (self.anim_current + 1) % self.anim_frames.len();
        self.anim_last_frame = Instant::now();
        self.present(&pixels);
    }

    pub(super) fn advance_transition(&mut self, qh: &QueueHandle<WaylandState>) {
        let Some(ref mut transition) = self.transition else {
            return;
        };

        if transition.is_done() {
            tracing::info!("transition complete");
            self.transition = None;
            self.frame_callback = None;
            return;
        }

        let (w, h) = transition.dimensions();
        let buf_size = (w * 4 * h) as usize;
        self.front = 1 - self.front;
        let offset = self.front * buf_size;

        transition.frame(&mut self.pool.mmap()[offset..offset + buf_size]);

        let surface = self.layer_surface.wl_surface();
        surface.attach(Some(self.active_buffer()), 0, 0);
        surface.damage_buffer(0, 0, w.cast_signed(), h.cast_signed());
        self.frame_callback = Some(surface.frame(qh, surface.clone()));
        self.layer_surface.commit();
    }
}

pub(super) struct WaylandState {
    pub(super) registry_state: RegistryState,
    pub(super) output_state: OutputState,
    pub(super) compositor: CompositorState,
    pub(super) layer_shell: LayerShell,
    pub(super) shm: Shm,
    pub(super) pending_surfaces: Vec<PendingSurface>,
    pub(super) committed: Vec<CommittedSurface>,
    pub(super) animation_timer: Option<RegistrationToken>,
}

impl WaylandState {
    pub(super) fn bind(global_list: &GlobalList, qh: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(global_list),
            output_state: OutputState::new(global_list, qh),
            compositor: CompositorState::bind(global_list, qh).context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(global_list, qh).context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(global_list, qh).context("wl_shm not available")?,
            pending_surfaces: Vec::new(),
            committed: Vec::new(),
            animation_timer: None,
        })
    }

    pub(super) fn create_surfaces(&mut self, qh: &QueueHandle<Self>) {
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

            let surface = self.compositor.create_surface(qh);
            let layer_surface = self.layer_shell.create_layer_surface(qh, surface, Layer::Background, Some("wallpaper-rs"), Some(&handle));

            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();

            self.pending_surfaces.push(PendingSurface { layer_surface, width: w.cast_unsigned(), height: h.cast_unsigned() });
        }

        tracing::info!(count = self.pending_surfaces.len(), "surfaces created");
    }

    pub(super) fn start_animation_timer(&mut self, handle: &LoopHandle<'static, WaylandState>) -> Result<()> {
        if self.animation_timer.is_some() {
            return Ok(());
        }
        if !self.committed.iter().any(|cs| !cs.anim_frames.is_empty()) {
            return Ok(());
        }

        tracing::info!("starting animation timer");

        let token = handle
            .insert_source(Timer::immediate(), |_, _, state| {
                for cs in &mut state.committed {
                    cs.tick_animation();
                }
                TimeoutAction::ToDuration(Duration::from_millis(16))
            })
            .map_err(|e| anyhow::anyhow!("failed to insert animation timer: {e}"))?;

        self.animation_timer = Some(token);
        Ok(())
    }

    pub(super) fn set_wallpapers(&mut self, image: &Path, transition: &TransitionConfig, resize: &ResizeConfig, qh: &QueueHandle<Self>) -> Result<()> {
        if self.pending_surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        tracing::info!(
            count = self.pending_surfaces.len(),
            image = %image.display(),
            transition = ?transition.transition_type,
            resize = ?resize.strategy,
            "applying wallpapers"
        );

        let renderer = Render::new(image)?;
        let surfaces = std::mem::take(&mut self.pending_surfaces);

        for surface in surfaces {
            self.commit_surface(surface, &renderer, transition, resize, qh)?;
        }

        Ok(())
    }

    fn commit_surface(&mut self, pending: PendingSurface, renderer: &Render, transition_cfg: &TransitionConfig, resize: &ResizeConfig, qh: &QueueHandle<Self>) -> Result<()> {
        let (w, h) = (pending.width, pending.height);
        let buf_size = (w * 4 * h) as usize;
        let stride = w * 4;

        let mut pixels = vec![0u8; buf_size];
        renderer.render(w, h, &mut pixels, resize)?;

        let anim_frames: Vec<AnimatedFrame> = renderer.animation_frames().map(|f| renderer.render_animation_frames(w, h, f, resize)).transpose()?.unwrap_or_default();

        let with_transition = !matches!(transition_cfg.transition_type, TransitionType::None);
        let needs_double_buf = with_transition || !anim_frames.is_empty();

        let pool_size = if needs_double_buf { buf_size * 2 } else { buf_size };
        let mut pool = RawPool::new(pool_size, &self.shm).context("failed to create shm pool")?;

        pool.mmap()[..buf_size].copy_from_slice(&pixels);

        let (buffer_a, buffer_b) = alloc_buffers(&mut pool, w, h, stride, buf_size, needs_double_buf, qh);

        let wl_surface = pending.layer_surface.wl_surface();
        wl_surface.attach(Some(&buffer_a), 0, 0);
        wl_surface.damage_buffer(0, 0, w.cast_signed(), h.cast_signed());

        let (transition, frame_callback) = if with_transition {
            let t = Transition::new(transition_cfg, (w, h), pixels);
            tracing::info!(w, h, "transition starting");
            let cb = wl_surface.frame(qh, wl_surface.clone());
            (Some(t), Some(cb))
        } else {
            let cb = (!anim_frames.is_empty()).then(|| wl_surface.frame(qh, wl_surface.clone()));
            (None, cb)
        };

        pending.layer_surface.commit();

        self.committed.push(CommittedSurface {
            layer_surface: pending.layer_surface,
            pool,
            buffer_a,
            buffer_b,
            front: 0,
            transition,
            frame_callback,
            anim_frames,
            anim_current: 0,
            anim_last_frame: Instant::now(),
            width: w,
            height: h,
        });

        Ok(())
    }

    pub(super) fn advance_transition(&mut self, surface: &WlSurface, qh: &QueueHandle<Self>) {
        match self.committed.iter_mut().find(|c| c.layer_surface.wl_surface() == surface) {
            Some(cs) => cs.advance_transition(qh),
            None => tracing::warn!("frame callback for unknown surface"),
        }
    }
}

fn alloc_buffers(pool: &mut RawPool, width: u32, height: u32, stride: u32, buf_size: usize, double: bool, qh: &QueueHandle<WaylandState>) -> (WlBuffer, WlBuffer) {
    let mut mk = |offset: i32| pool.create_buffer::<WaylandState, ()>(offset, width.cast_signed(), height.cast_signed(), stride.cast_signed(), Xrgb8888, (), qh);
    let a = mk(0);
    let b = if double { mk(buf_size as i32) } else { a.clone() };
    (a, b)
}
