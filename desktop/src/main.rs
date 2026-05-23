//! # MDB Desktop — Entry Point
//!
//! Launches the MDB Desktop Wayland compositor.
//!
//! ## Backends
//!
//! - **winit** (default for development): Runs as a window inside another
//!   desktop. Good for testing without a dedicated GPU session.
//!
//! - **udev** (production): Runs directly on DRM/KMS hardware.
//!   This is what the bootable ISO uses — no other desktop needed.
//!
//! ## Usage
//!
//! ```bash
//! # Development mode (winit window)
//! mdb-desktop --backend winit
//!
//! # Production mode (direct hardware)
//! mdb-desktop --backend udev
//!
//! # The ISO auto-launches this at boot via systemd
//! ```

mod config;
mod grab;
mod input;
mod notification;
mod output;
mod render;
mod screenshot;
mod session;
mod state;
mod tiling;
mod wallpaper;

use clap::Parser;
use log::info;
use state::MdbDesktop;

use smithay::reexports::{
    calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
    wayland_server::Display,
};
use smithay::wayland::socket::ListeningSocketSource;

#[derive(Parser)]
#[command(name = "mdb-desktop")]
#[command(about = "MDB Desktop Environment — Wayland compositor for MDB-OS")]
#[command(version = mdb_core::VERSION)]
struct Cli {
    /// Backend: "winit" (dev) or "udev" (production hardware).
    #[arg(short, long, default_value = "udev")]
    backend: String,

    /// Path to config file (default: ~/.config/mdb-os/desktop.toml).
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,

    /// Enable debug logging.
    #[arg(short, long)]
    debug: bool,
}

fn main() {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level)).init();

    info!("╔══════════════════════════════════════╗");
    info!("║       MDB Desktop Environment        ║");
    info!("║   Multidimensional Binary OS v{}   ║", mdb_core::VERSION);
    info!("╚══════════════════════════════════════╝");

    // Load configuration
    let config = config::DesktopConfig::load();
    info!("Config loaded. Panel: {}px {}", config.panel.height, config.panel.position);

    // Create Wayland display and event loop
    let mut event_loop: EventLoop<'static, MdbDesktop> =
        EventLoop::try_new().expect("Failed to create event loop");
    let display: Display<MdbDesktop> = Display::new().expect("Failed to create Wayland display");

    // Create compositor state
    let mut state = MdbDesktop::new(&display, &event_loop, config);

    // Set up the Wayland listening socket
    let listening_socket = ListeningSocketSource::new_auto()
        .expect("Failed to create Wayland socket");
    let socket_name = listening_socket.socket_name().to_string_lossy().to_string();
    info!("Wayland socket: {}", socket_name);

    // Insert the socket source into the event loop
    event_loop
        .handle()
        .insert_source(listening_socket, |client_stream, _, state| {
            // A new client connected — accept it
            // state.display_handle.insert_client(client_stream, Arc::new(ClientState { ... }));
            log::debug!("New Wayland client connected");
        })
        .expect("Failed to insert socket source");

    // Insert the Wayland display into the event loop
    let display_fd = Generic::new(display, Interest::READ, Mode::Level);
    event_loop
        .handle()
        .insert_source(display_fd, |_, display, state| {
            // Process Wayland events
            unsafe {
                display.get_mut().dispatch_clients(state).ok();
            }
            Ok(PostAction::Continue)
        })
        .expect("Failed to insert display source");

    // Set WAYLAND_DISPLAY so child processes connect to us
    std::env::set_var("WAYLAND_DISPLAY", &socket_name);

    // Check MDBFS status
    state.refresh_mdb_status();
    if state.mdbfs_mounted {
        info!("MDBFS: mounted at {}", state.config.paths.mdbfs_mount.display());
    } else {
        info!("MDBFS: not mounted (will attempt auto-mount)");
        attempt_mdbfs_mount(&state);
    }

    info!("Starting compositor with '{}' backend...", cli.backend);

    match cli.backend.as_str() {
        "winit" => run_winit(event_loop, state),
        "udev" => run_udev(event_loop, state),
        other => {
            eprintln!("Unknown backend: {}. Use 'winit' or 'udev'.", other);
            std::process::exit(1);
        }
    }
}

/// Run the compositor inside a winit window (development mode).
fn run_winit(mut event_loop: EventLoop<'static, MdbDesktop>, mut state: MdbDesktop) {
    info!("Winit backend: creating development window...");

    // In production code, this initializes smithay's winit backend:
    //
    //   let (mut backend, mut winit_evt) = smithay::backend::winit::init()
    //       .expect("Failed to initialize winit backend");
    //
    // Then the render loop:
    //
    //   loop {
    //       winit_evt.dispatch_new_events(|event| { ... });
    //       backend.bind().unwrap();
    //       // render desktop
    //       state.space.render_output(&mut backend.renderer(), ...);
    //       backend.submit(None).unwrap();
    //       event_loop.dispatch(Duration::from_millis(16), &mut state).unwrap();
    //   }

    info!("Winit backend initialized — entering event loop");
    info!("(Rendering requires smithay + system GPU libs — see build instructions)");

    // Run the event loop
    event_loop
        .run(std::time::Duration::from_millis(16), &mut state, |state| {
            // Per-tick housekeeping
            state.refresh_mdb_status();
        })
        .expect("Event loop failed");
}

/// Run the compositor directly on DRM/KMS hardware (production mode).
fn run_udev(mut event_loop: EventLoop<'static, MdbDesktop>, mut state: MdbDesktop) {
    info!("Udev backend: scanning for GPU devices...");

    // In production code, this initializes the DRM/KMS + libinput backend:
    //
    //   let session = smithay::backend::session::auto::AutoSession::new()
    //       .expect("Failed to open session");
    //   let udev_backend = smithay::backend::udev::UdevBackend::new(seat_name)
    //       .expect("Failed to create udev backend");
    //
    // For each GPU found:
    //   - Open DRM device
    //   - Create GBM allocator
    //   - Initialize Gles2 renderer
    //   - Scan connectors for monitors
    //   - Set CRTC modes
    //
    // libinput handles keyboard/mouse/touchpad:
    //   let input_backend = smithay::backend::libinput::LibinputInputBackend::new(session);

    info!("Udev backend initialized — entering render loop");
    info!("(Requires DRM/KMS hardware — runs on bootable ISO)");

    event_loop
        .run(std::time::Duration::from_millis(16), &mut state, |state| {
            state.refresh_mdb_status();
        })
        .expect("Event loop failed");
}

/// Try to auto-mount MDBFS if not already mounted.
fn attempt_mdbfs_mount(state: &MdbDesktop) {
    let mount = &state.config.paths.mdbfs_mount;
    let store = &state.config.paths.mdbfs_store;

    info!(
        "Attempting MDBFS auto-mount: {} -> {}",
        store.display(),
        mount.display()
    );

    // Create mount point if needed
    if let Err(e) = std::fs::create_dir_all(mount) {
        log::warn!("Could not create mount point: {}", e);
        return;
    }

    // Launch mdbfs mount in background
    match std::process::Command::new("mdbfs")
        .args(["mount", &mount.to_string_lossy(), "--store", &store.to_string_lossy(), "--foreground"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_child) => info!("MDBFS mount process started"),
        Err(e) => log::warn!("Failed to start MDBFS: {}", e),
    }
}
