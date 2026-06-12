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

pub(super) struct AnimationState {
    pub(super) frames: Vec<AnimatedFrame>,
    pub(super) current: usize,
    pub(super) last_frame: Instant,
}

impl AnimationState {
    fn new(frames: Vec<AnimatedFrame>) -> Self {
        Self {
            frames,
            current: 0,
            last_frame: Instant::now(),
        }
    }

    fn is_due(&self) -> bool {
        let delay_ms = self.frames[self.current].delay_ms as u128;
        self.last_frame.elapsed().as_millis() >= delay_ms
    }

    fn advance(&mut self) {
        self.current = (self.current + 1) % self.frames.len();
        self.last_frame = Instant::now();
    }
}

pub(super) struct CommittedSurface {
    pub(super) layer_surface: LayerSurface,
    pub(super) pool: RawPool,
    pub(super) buffer_a: WlBuffer,
    pub(super) buffer_b: WlBuffer,
    pub(super) front: usize,
    pub(super) transition: Option<Transition>,
    pub(super) frame_callback: Option<wayland_client::protocol::wl_callback::WlCallback>,
    pub(super) animation: Option<AnimationState>,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl CommittedSurface {
    fn buf_size(&self) -> usize {
        (self.width * 4 * self.height) as usize
    }

    fn active_buffer(&self) -> &WlBuffer {
        if self.front == 0 {
            &self.buffer_a
        } else {
            &self.buffer_b
        }
    }

    fn swap_front(&mut self) -> usize {
        self.front = 1 - self.front;
        self.front * self.buf_size()
    }

    fn present(&mut self, pixels: &[u8]) {
        let buf_size = self.buf_size();
        let offset = self.swap_front();

        let dst = self.pool.mmap();
        let len = pixels.len().min(buf_size);
        dst[offset..offset + len].copy_from_slice(&pixels[..len]);

        let wl_surface = self.layer_surface.wl_surface();
        wl_surface.attach(Some(self.active_buffer()), 0, 0);
        wl_surface.damage_buffer(0, 0, self.width.cast_signed(), self.height.cast_signed());
        self.layer_surface.commit();
    }

    pub(super) fn tick_animation(&mut self) {
        if self.transition.is_some() {
            return;
        }

        let Some(ref mut anim) = self.animation else {
            return;
        };

        if !anim.is_due() {
            return;
        }

        let frame_pixels = anim.frames[anim.current].pixels.clone();
        anim.advance();

        self.present(&frame_pixels);
    }

    pub(super) fn advance_transition(&mut self, queue_handle: &QueueHandle<WaylandState>) {
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
        let offset = {
            self.front = 1 - self.front;
            self.front * buf_size
        };

        {
            let pixels = self.pool.mmap();
            transition.frame(&mut pixels[offset..offset + buf_size]);
        }

        let wl_surface = self.layer_surface.wl_surface();
        wl_surface.attach(Some(self.active_buffer()), 0, 0);
        wl_surface.damage_buffer(0, 0, w.cast_signed(), h.cast_signed());

        let callback = wl_surface.frame(queue_handle, wl_surface.clone());
        self.frame_callback = Some(callback);

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
    pub(super) fn bind(global_list: &GlobalList, queue_handle: &QueueHandle<Self>) -> Result<Self> {
        Ok(Self {
            registry_state: RegistryState::new(global_list),
            output_state: OutputState::new(global_list, queue_handle),
            compositor: CompositorState::bind(global_list, queue_handle)
                .context("wl_compositor not available")?,
            layer_shell: LayerShell::bind(global_list, queue_handle)
                .context("zwlr_layer_shell_v1 not available")?,
            shm: Shm::bind(global_list, queue_handle).context("wl_shm not available")?,
            pending_surfaces: Vec::new(),
            committed: Vec::new(),
            animation_timer: None,
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

            let surface = self.compositor.create_surface(queue_handle);
            let layer_surface = self.layer_shell.create_layer_surface(
                queue_handle,
                surface,
                Layer::Background,
                Some("wallpaper-rs"),
                Some(&handle),
            );

            layer_surface.set_anchor(Anchor::all());
            layer_surface.set_exclusive_zone(-1);
            layer_surface.set_size(0, 0);
            layer_surface.commit();

            self.pending_surfaces.push(PendingSurface {
                layer_surface,
                width: w.cast_unsigned(),
                height: h.cast_unsigned(),
            });
        }

        tracing::info!(count = self.pending_surfaces.len(), "surfaces created");
    }

    pub(super) fn start_animation_timer(
        &mut self,
        handle: &LoopHandle<'static, WaylandState>,
    ) -> Result<()> {
        if self.animation_timer.is_some() {
            return Ok(());
        }
        if !self.committed.iter().any(|cs| cs.animation.is_some()) {
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

    pub(super) fn set_wallpapers(
        &mut self,
        image: &Path,
        transition: &TransitionConfig,
        resize: &ResizeConfig,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        if self.pending_surfaces.is_empty() {
            anyhow::bail!("no surfaces were configured by the compositor");
        }

        tracing::info!(
            count = self.pending_surfaces.len(),
            image = %image.display(),
            transition = ?&transition.transition_type,
            resize = ?resize.strategy,
            "applying wallpapers"
        );

        let renderer = Render::new(image)?;
        let surfaces = std::mem::take(&mut self.pending_surfaces);

        for surface in surfaces {
            self.commit_wallpaper(surface, &renderer, transition, resize, queue_handle)?;
        }

        Ok(())
    }

    fn commit_wallpaper(
        &mut self,
        pending: PendingSurface,
        renderer: &Render,
        transition: &TransitionConfig,
        resize: &ResizeConfig,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let w = pending.width;
        let h = pending.height;
        let stride = w * 4;
        let buf_size = (stride * h) as usize;

        let mut pixels = vec![0u8; buf_size];
        renderer.render(w, h, &mut pixels, resize)?;

        let anim_frames = renderer
            .animation_frames()
            .map(|f| renderer.render_animation_frames(w, h, f, resize))
            .transpose()?;

        match transition.transition_type {
            TransitionType::None => {
                self.commit_static(pending, pixels, anim_frames, buf_size, stride, queue_handle)
            }
            _ => self.commit_with_transition(
                pending,
                pixels,
                anim_frames,
                transition,
                buf_size,
                stride,
                queue_handle,
            ),
        }
    }

    fn commit_static(
        &mut self,
        pending: PendingSurface,
        pixels: Vec<u8>,
        anim_frames: Option<Vec<AnimatedFrame>>,
        buf_size: usize,
        stride: u32,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let has_anim = anim_frames.is_some();
        let pool_size = if has_anim { buf_size * 2 } else { buf_size };

        let mut pool = RawPool::new(pool_size, &self.shm).context("failed to create shm pool")?;
        pool.mmap()[..buf_size].copy_from_slice(&pixels);

        let (buffer_a, buffer_b) = Self::alloc_buffers(
            &mut pool,
            pending.width,
            pending.height,
            stride,
            buf_size,
            has_anim,
            queue_handle,
        );

        let wl_surface = pending.layer_surface.wl_surface();
        wl_surface.attach(Some(&buffer_a), 0, 0);
        wl_surface.damage_buffer(
            0,
            0,
            pending.width.cast_signed(),
            pending.height.cast_signed(),
        );

        let frame_callback = has_anim.then(|| wl_surface.frame(queue_handle, wl_surface.clone()));

        pending.layer_surface.commit();

        self.committed.push(CommittedSurface {
            layer_surface: pending.layer_surface,
            pool,
            buffer_a,
            buffer_b,
            front: 0,
            transition: None,
            frame_callback,
            animation: anim_frames.map(AnimationState::new),
            width: pending.width,
            height: pending.height,
        });

        Ok(())
    }

    fn commit_with_transition(
        &mut self,
        pending: PendingSurface,
        pixels: Vec<u8>,
        anim_frames: Option<Vec<AnimatedFrame>>,
        transition_cfg: &TransitionConfig,
        buf_size: usize,
        stride: u32,
        queue_handle: &QueueHandle<Self>,
    ) -> Result<()> {
        let mut pool =
            RawPool::new(buf_size * 2, &self.shm).context("failed to create shm pool")?;

        let (buffer_a, buffer_b) = Self::alloc_buffers(
            &mut pool,
            pending.width,
            pending.height,
            stride,
            buf_size,
            true,
            queue_handle,
        );

        let transition = Transition::new(transition_cfg, (pending.width, pending.height), pixels);

        tracing::info!(w = pending.width, h = pending.height, "transition starting");

        let wl_surface = pending.layer_surface.wl_surface();
        wl_surface.attach(Some(&buffer_a), 0, 0);
        wl_surface.damage_buffer(
            0,
            0,
            pending.width.cast_signed(),
            pending.height.cast_signed(),
        );

        let callback = wl_surface.frame(queue_handle, wl_surface.clone());
        pending.layer_surface.commit();

        self.committed.push(CommittedSurface {
            layer_surface: pending.layer_surface,
            pool,
            buffer_a,
            buffer_b,
            front: 0,
            transition: Some(transition),
            frame_callback: Some(callback),
            animation: anim_frames.map(AnimationState::new),
            width: pending.width,
            height: pending.height,
        });

        Ok(())
    }

    fn alloc_buffers(
        pool: &mut RawPool,
        width: u32,
        height: u32,
        stride: u32,
        buf_size: usize,
        double: bool,
        queue_handle: &QueueHandle<Self>,
    ) -> (WlBuffer, WlBuffer) {
        let mut mk = |offset: i32| {
            pool.create_buffer::<WaylandState, ()>(
                offset,
                width.cast_signed(),
                height.cast_signed(),
                stride.cast_signed(),
                Xrgb8888,
                (),
                queue_handle,
            )
        };

        let a = mk(0);
        let b = if double {
            mk(buf_size as i32)
        } else {
            a.clone()
        };
        (a, b)
    }

    pub(super) fn advance_transition(
        &mut self,
        surface: &WlSurface,
        queue_handle: &QueueHandle<Self>,
    ) {
        let Some(cs) = self
            .committed
            .iter_mut()
            .find(|c| c.layer_surface.wl_surface() == surface)
        else {
            tracing::warn!("frame callback for unknown surface");
            return;
        };

        cs.advance_transition(queue_handle);
    }
}
