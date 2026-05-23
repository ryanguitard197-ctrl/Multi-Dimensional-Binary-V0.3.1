//! # Compositor State
//!
//! Central state struct for the MDB Desktop compositor.
//! Holds all Wayland protocol states, window tracking, and MDB integration.

use crate::config::DesktopConfig;
use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    desktop::{Space, Window},
    input::{keyboard::XkbConfig, SeatState, Seat, SeatHandler},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display, DisplayHandle,
            protocol::wl_surface::WlSurface,
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState,
            with_states,
        },
        output::OutputManagerState,
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
        shm::{ShmHandler, ShmState},
        selection::data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            set_data_device_focus,
        },
        socket::ListeningSocketSource,
    },
    utils::{Serial, SERIAL_COUNTER},
};
use std::sync::Arc;

/// Per-client state stored by the Wayland display.
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// Central compositor state.
pub struct MdbDesktop {
    // === Wayland protocol states ===
    pub display_handle: DisplayHandle,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,

    // === Desktop state ===
    pub space: Space<Window>,
    pub loop_signal: LoopSignal,
    pub config: DesktopConfig,

    // === Window management ===
    pub focused_window: Option<Window>,
    pub current_workspace: u32,
    pub pending_windows: Vec<ToplevelSurface>,

    // === MDB integration ===
    pub mdbfs_mounted: bool,
    pub mdb_fold_count: u64,
    pub mdb_total_dimensional_bytes: u64,
}

impl MdbDesktop {
    pub fn new(
        display: &Display<Self>,
        event_loop: &EventLoop<'static, Self>,
        config: DesktopConfig,
    ) -> Self {
        let display_handle = display.handle();
        let loop_signal = event_loop.get_signal();

        // Initialize Wayland protocol handlers
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        // Create the default seat (keyboard + pointer)
        let mut seat = seat_state.new_wl_seat(&display_handle, "mdb-seat-0");

        // Configure keyboard with default US layout
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to add keyboard to seat");
        seat.add_pointer();

        let space = Space::default();

        Self {
            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
            space,
            loop_signal,
            config,
            focused_window: None,
            current_workspace: 0,
            pending_windows: Vec::new(),
            mdbfs_mounted: false,
            mdb_fold_count: 0,
            mdb_total_dimensional_bytes: 0,
        }
    }

    /// Set keyboard focus to a window.
    pub fn set_focus(&mut self, window: Option<&Window>) {
        let surface = window.and_then(|w| {
            w.toplevel().map(|t| t.wl_surface().clone())
        });

        let serial = SERIAL_COUNTER.next_serial();
        let seat = &self.seat;

        // Update keyboard focus
        if let Some(keyboard) = seat.get_keyboard() {
            keyboard.set_focus(self, surface.as_ref(), serial);
        }

        // Update data device focus for copy/paste
        if let Some(surface) = &surface {
            let client = self.display_handle.get_client(surface.id())
                .ok();
            set_data_device_focus(&self.display_handle, &seat, client);
        }

        self.focused_window = window.cloned();
    }

    /// Place a new window in the space at a reasonable position.
    pub fn place_window(&mut self, window: &Window) {
        // Simple cascading placement
        let count = self.space.elements().count() as i32;
        let x = 60 + (count * 30);
        let y = 60 + (count * 30) + self.config.panel.height as i32;

        self.space.map_element(window.clone(), (x, y), false);
    }

    /// Switch to a workspace by index.
    pub fn switch_workspace(&mut self, idx: u32) {
        if idx < self.config.panel.workspace_count {
            self.current_workspace = idx;
            log::info!("Switched to workspace {}", idx);
        }
    }

    /// Check if MDBFS is mounted and update stats.
    pub fn refresh_mdb_status(&mut self) {
        let mount_path = &self.config.paths.mdbfs_mount;
        self.mdbfs_mounted = mount_path.exists() && mount_path.is_dir();

        // Read fold stats from the backing store
        let store_path = &self.config.paths.mdbfs_store;
        if store_path.join("data").exists() {
            if let Ok(entries) = std::fs::read_dir(store_path.join("data")) {
                let mut count = 0u64;
                let mut bytes = 0u64;
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        count += 1;
                        bytes += meta.len();
                    }
                }
                self.mdb_fold_count = count;
                self.mdb_total_dimensional_bytes = bytes;
            }
        }
    }
}

// ============================================================
// Protocol handler implementations
// ============================================================

impl CompositorHandler for MdbDesktop {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a ClientId) -> &'a CompositorClientState {
        // Not directly available in all smithay versions — this is scaffolding.
        // The actual implementation depends on the ClientData stored per-client.
        unimplemented!("client_compositor_state: wire up via ClientState")
    }

    fn commit(&mut self, surface: &WlSurface) {
        // Forward buffer commits to the space for rendering
        self.space.elements().for_each(|window| {
            window.toplevel().map(|tl| {
                if tl.wl_surface() == surface {
                    window.on_commit();
                }
            });
        });
    }
}

impl BufferHandler for MdbDesktop {
    fn buffer_destroyed(&mut self, _buffer: &smithay::wayland::buffer::Buffer) {}
}

impl ShmHandler for MdbDesktop {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl XdgShellHandler for MdbDesktop {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface.clone());
        self.place_window(&window);
        self.set_focus(Some(&window));

        log::info!("New toplevel window mapped");
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // Popups (menus, tooltips) — position relative to parent
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        // Remove from space, update focus
        let wl_surface = surface.wl_surface();
        if let Some(window) = self.space.elements().find(|w| {
            w.toplevel().map(|t| t.wl_surface() == wl_surface).unwrap_or(false)
        }).cloned() {
            self.space.unmap_elem(&window);

            // Focus the next window if we closed the focused one
            if self.focused_window.as_ref() == Some(&window) {
                let next = self.space.elements().next().cloned();
                self.set_focus(next.as_ref());
            }
        }
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, _serial: Serial) {
    }
}

impl SeatHandler for MdbDesktop {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: smithay::input::pointer::CursorImageStatus) {}
}

impl DataDeviceHandler for MdbDesktop {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for MdbDesktop {}
impl ServerDndGrabHandler for MdbDesktop {}

// Delegate macros — wire protocol dispatching to our handler impls
delegate_compositor!(MdbDesktop);
delegate_shm!(MdbDesktop);
delegate_xdg_shell!(MdbDesktop);
delegate_seat!(MdbDesktop);
delegate_data_device!(MdbDesktop);
delegate_output!(MdbDesktop);
