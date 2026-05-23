//! # MDB Desktop Configuration
//!
//! Theme, keybindings, and paths for the desktop environment.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Desktop configuration loaded from `~/.config/mdb-os/desktop.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    pub theme: ThemeConfig,
    pub keybindings: KeybindingConfig,
    pub panel: PanelConfig,
    pub paths: PathConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Desktop background color [R, G, B, A] as floats 0.0–1.0.
    pub background_color: [f32; 4],
    /// Optional wallpaper image path.
    pub wallpaper: Option<PathBuf>,
    /// Panel background color.
    pub panel_color: [f32; 4],
    /// Panel text color.
    pub panel_text_color: [f32; 4],
    /// Window border color (focused).
    pub border_focused: [f32; 4],
    /// Window border color (unfocused).
    pub border_unfocused: [f32; 4],
    /// Border width in pixels.
    pub border_width: u32,
    /// Corner radius for windows.
    pub corner_radius: u32,
    /// Font size for panel text.
    pub font_size: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    /// Modifier key: "Super", "Alt", "Ctrl".
    pub modifier: String,
    /// Key to open terminal.
    pub terminal: String,
    /// Key to open launcher.
    pub launcher: String,
    /// Key to close focused window.
    pub close_window: String,
    /// Key to toggle fullscreen.
    pub fullscreen: String,
    /// Key to switch to next workspace.
    pub workspace_next: String,
    /// Key to switch to previous workspace.
    pub workspace_prev: String,
    /// Key to open file manager.
    pub file_manager: String,
    /// Key to lock screen.
    pub lock_screen: String,
    /// Key to log out.
    pub logout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// Panel height in pixels.
    pub height: u32,
    /// Panel position: "top" or "bottom".
    pub position: String,
    /// Show clock.
    pub show_clock: bool,
    /// Show MDB status (fold stats, dimensional usage).
    pub show_mdb_status: bool,
    /// Show workspace indicators.
    pub show_workspaces: bool,
    /// Number of workspaces.
    pub workspace_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Path to MDBFS mount.
    pub mdbfs_mount: PathBuf,
    /// Path to MDBFS backing store.
    pub mdbfs_store: PathBuf,
    /// Default terminal emulator command.
    pub terminal_cmd: String,
    /// Default file manager command.
    pub file_manager_cmd: String,
    /// Default launcher command.
    pub launcher_cmd: String,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig {
                // Deep dark blue — MDB brand color
                background_color: [0.05, 0.05, 0.12, 1.0],
                wallpaper: None,
                // Semi-transparent dark panel
                panel_color: [0.08, 0.08, 0.16, 0.92],
                panel_text_color: [0.9, 0.92, 0.96, 1.0],
                // Cyan glow for focused windows
                border_focused: [0.0, 0.85, 0.95, 1.0],
                border_unfocused: [0.25, 0.25, 0.35, 0.6],
                border_width: 2,
                corner_radius: 8,
                font_size: 14.0,
            },
            keybindings: KeybindingConfig {
                modifier: "Super".to_string(),
                terminal: "Return".to_string(),
                launcher: "d".to_string(),
                close_window: "q".to_string(),
                fullscreen: "f".to_string(),
                workspace_next: "Right".to_string(),
                workspace_prev: "Left".to_string(),
                file_manager: "e".to_string(),
                lock_screen: "l".to_string(),
                logout: "Escape".to_string(),
            },
            panel: PanelConfig {
                height: 32,
                position: "top".to_string(),
                show_clock: true,
                show_mdb_status: true,
                show_workspaces: true,
                workspace_count: 4,
            },
            paths: PathConfig {
                mdbfs_mount: PathBuf::from("/home/user/mdb"),
                mdbfs_store: PathBuf::from("/var/lib/mdbfs"),
                terminal_cmd: "foot".to_string(),
                file_manager_cmd: "mdb-files".to_string(),
                launcher_cmd: "fuzzel".to_string(),
            },
        }
    }
}

impl DesktopConfig {
    /// Load config from the standard path, falling back to defaults.
    pub fn load() -> Self {
        let config_path = dirs_config_path().join("desktop.toml");
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(contents) => match toml::from_str(&contents) {
                    Ok(cfg) => return cfg,
                    Err(e) => {
                        log::warn!("Failed to parse config {}: {}", config_path.display(), e);
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read config {}: {}", config_path.display(), e);
                }
            }
        }
        Self::default()
    }

    /// Save the current config to disk.
    pub fn save(&self) -> std::io::Result<()> {
        let config_dir = dirs_config_path();
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("desktop.toml");
        let contents = toml::to_string_pretty(self).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e)
        })?;
        std::fs::write(config_path, contents)
    }
}

fn dirs_config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("mdb-os")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("mdb-os")
    } else {
        PathBuf::from("/etc/mdb-os")
    }
}
