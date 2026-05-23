//! # Input Handling
//!
//! Keyboard shortcuts, pointer/mouse handling, and gesture processing
//! for the MDB Desktop compositor.

use crate::state::MdbDesktop;
use smithay::input::keyboard::{FilterResult, KeysymHandle, ModifiersState};
use log::info;
use std::process::Command;

/// Actions triggered by keyboard shortcuts.
#[derive(Debug, Clone)]
pub enum KeyAction {
    /// Launch terminal emulator.
    Terminal,
    /// Open application launcher.
    Launcher,
    /// Open MDB file manager.
    FileManager,
    /// Close the focused window.
    CloseWindow,
    /// Toggle fullscreen on focused window.
    ToggleFullscreen,
    /// Switch to next workspace.
    WorkspaceNext,
    /// Switch to previous workspace.
    WorkspacePrev,
    /// Switch to a specific workspace (0-indexed).
    WorkspaceSwitch(u32),
    /// Lock the screen.
    LockScreen,
    /// Log out / quit compositor.
    Quit,
    /// Move focused window (begin interactive move).
    MoveWindow,
    /// Resize focused window (begin interactive resize).
    ResizeWindow,
    /// No action — forward key to focused client.
    Forward,
}

/// Process a key press and return the action.
pub fn process_key(
    state: &MdbDesktop,
    modifiers: &ModifiersState,
    keysym: u32,
) -> KeyAction {
    let super_held = modifiers.logo;
    let alt_held = modifiers.alt;
    let ctrl_held = modifiers.ctrl;
    let shift_held = modifiers.shift;

    // Super (Logo) key shortcuts — primary modifier
    if super_held {
        match keysym {
            // xkb keysym values
            0xff0d => return KeyAction::Terminal,        // Return
            0x0064 => return KeyAction::Launcher,        // d
            0x0065 => return KeyAction::FileManager,     // e
            0x0071 => return KeyAction::CloseWindow,     // q
            0x0066 => return KeyAction::ToggleFullscreen, // f
            0xff53 => return KeyAction::WorkspaceNext,   // Right
            0xff51 => return KeyAction::WorkspacePrev,   // Left
            0x006c => return KeyAction::LockScreen,      // l

            // Super+1..9 → switch to workspace 0..8
            k @ 0x0031..=0x0039 => {
                return KeyAction::WorkspaceSwitch((k - 0x0031) as u32);
            }

            _ => {}
        }

        // Super+Shift shortcuts
        if shift_held {
            match keysym {
                0x0071 => return KeyAction::Quit, // Super+Shift+Q → logout
                _ => {}
            }
        }
    }

    // Alt+F4 → close window (for familiarity)
    if alt_held && keysym == 0xffc1 {
        return KeyAction::CloseWindow;
    }

    KeyAction::Forward
}

/// Execute a key action.
pub fn execute_action(state: &mut MdbDesktop, action: KeyAction) {
    match action {
        KeyAction::Terminal => {
            let cmd = &state.config.paths.terminal_cmd;
            info!("Launching terminal: {}", cmd);
            spawn_detached(cmd);
        }
        KeyAction::Launcher => {
            let cmd = &state.config.paths.launcher_cmd;
            info!("Launching app launcher: {}", cmd);
            spawn_detached(cmd);
        }
        KeyAction::FileManager => {
            let cmd = &state.config.paths.file_manager_cmd;
            info!("Launching file manager: {}", cmd);
            spawn_detached(cmd);
        }
        KeyAction::CloseWindow => {
            if let Some(window) = &state.focused_window {
                info!("Closing focused window");
                if let Some(toplevel) = window.toplevel() {
                    toplevel.send_close();
                }
            }
        }
        KeyAction::ToggleFullscreen => {
            // Toggle fullscreen state on the focused window
            info!("Toggle fullscreen");
        }
        KeyAction::WorkspaceNext => {
            let next = (state.current_workspace + 1) % state.config.panel.workspace_count;
            state.switch_workspace(next);
        }
        KeyAction::WorkspacePrev => {
            let count = state.config.panel.workspace_count;
            let prev = (state.current_workspace + count - 1) % count;
            state.switch_workspace(prev);
        }
        KeyAction::WorkspaceSwitch(idx) => {
            state.switch_workspace(idx);
        }
        KeyAction::LockScreen => {
            info!("Locking screen");
            spawn_detached("swaylock");
        }
        KeyAction::Quit => {
            info!("Quitting compositor");
            state.loop_signal.stop();
        }
        KeyAction::MoveWindow | KeyAction::ResizeWindow => {
            // Interactive move/resize — handled by pointer grab
        }
        KeyAction::Forward => {
            // Nothing — key forwarded to client
        }
    }
}

/// Spawn a process detached from the compositor.
fn spawn_detached(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match Command::new(parts[0])
        .args(&parts[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => info!("Spawned: {}", cmd),
        Err(e) => log::error!("Failed to spawn {}: {}", cmd, e),
    }
}
