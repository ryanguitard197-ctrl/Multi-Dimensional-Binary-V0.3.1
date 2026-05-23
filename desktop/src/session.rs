//! # Session Management
//!
//! Handles compositor lifecycle: login, lock screen, sleep, logout,
//! and clean shutdown.
//!
//! ## Lock Screen
//!
//! When the session is locked:
//! - All output rendering switches to the lock surface
//! - Input events are consumed by the lock screen (password entry)
//! - Wayland clients cannot interact
//! - MDB processes continue running (folded state is preserved)
//!
//! ## Session Save/Restore
//!
//! On clean shutdown, the session state (open windows, workspaces,
//! window positions) is serialized and stored in MDBFS. On next login,
//! the session is restored. MDB folded processes survive across sessions.
//!
//! ## Idle Management
//!
//! Tracks user activity and triggers:
//! - Screen dimming after idle_dim_timeout
//! - Screen off (DPMS) after idle_off_timeout
//! - Lock after idle_lock_timeout (if enabled)

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Session is active — user is interacting.
    Active,
    /// User is idle (no input for a while).
    Idle,
    /// Screen is dimmed.
    Dimmed,
    /// Screen is off (DPMS standby).
    ScreenOff,
    /// Session is locked.
    Locked,
    /// Session is shutting down.
    ShuttingDown,
}

/// Idle timeouts configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleConfig {
    /// Seconds of inactivity before dimming.
    pub dim_timeout_secs: u64,
    /// Seconds of inactivity before screen off.
    pub off_timeout_secs: u64,
    /// Seconds of inactivity before auto-lock.
    pub lock_timeout_secs: u64,
    /// Whether to auto-lock on idle.
    pub auto_lock: bool,
    /// Whether to auto-lock on lid close (laptop).
    pub lock_on_lid_close: bool,
    /// Screen brightness when dimmed (0.0–1.0).
    pub dim_brightness: f64,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            dim_timeout_secs: 300,      // 5 minutes
            off_timeout_secs: 600,      // 10 minutes
            lock_timeout_secs: 900,     // 15 minutes
            auto_lock: true,
            lock_on_lid_close: true,
            dim_brightness: 0.3,
        }
    }
}

/// Persistent session data (saved to MDBFS for session restore).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Workspace count at save time.
    pub workspace_count: u32,
    /// Active workspace.
    pub active_workspace: u32,
    /// Window layout data.
    pub windows: Vec<SavedWindow>,
    /// Timestamp when saved.
    pub saved_at_epoch: u64,
}

/// A window's position/state for session restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedWindow {
    /// App ID or command that launched this window.
    pub app_id: String,
    /// Window title at save time.
    pub title: String,
    /// Workspace index.
    pub workspace: u32,
    /// Position X.
    pub x: i32,
    /// Position Y.
    pub y: i32,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Whether it was maximized.
    pub maximized: bool,
    /// Whether it was fullscreen.
    pub fullscreen: bool,
    /// Z-order (stacking position).
    pub z_order: u32,
}

/// The session manager.
pub struct SessionManager {
    /// Current session state.
    pub state: SessionState,
    /// When the last user input occurred.
    pub last_input: Instant,
    /// Idle configuration.
    pub idle_config: IdleConfig,
    /// Path to session save file.
    pub save_path: PathBuf,
    /// Whether the session has been modified since last save.
    pub dirty: bool,
    /// Lock screen: whether password verification is in progress.
    pub lock_verifying: bool,
    /// Lock screen: accumulated password input (cleared on submit/cancel).
    pub lock_password_buffer: String,
    /// Lock screen: number of failed attempts.
    pub lock_failed_attempts: u32,
    /// Lock screen: lockout until (if too many failed attempts).
    pub lock_lockout_until: Option<Instant>,
}

impl SessionManager {
    pub fn new(save_path: PathBuf) -> Self {
        Self {
            state: SessionState::Active,
            last_input: Instant::now(),
            idle_config: IdleConfig::default(),
            save_path,
            dirty: false,
            lock_verifying: false,
            lock_password_buffer: String::new(),
            lock_failed_attempts: 0,
            lock_lockout_until: None,
        }
    }

    /// Record user input (resets idle timer).
    pub fn record_input(&mut self) {
        self.last_input = Instant::now();

        match self.state {
            SessionState::Dimmed => {
                self.state = SessionState::Active;
                log::debug!("Session: undimmed");
            }
            SessionState::ScreenOff => {
                self.state = SessionState::Active;
                log::debug!("Session: screen on");
            }
            SessionState::Idle => {
                self.state = SessionState::Active;
            }
            _ => {}
        }
    }

    /// Tick — check idle timeouts and update state.
    pub fn tick(&mut self) {
        if self.state == SessionState::Locked || self.state == SessionState::ShuttingDown {
            return;
        }

        let idle_secs = self.last_input.elapsed().as_secs();

        if self.idle_config.auto_lock && idle_secs >= self.idle_config.lock_timeout_secs {
            if self.state != SessionState::Locked {
                self.lock();
            }
        } else if idle_secs >= self.idle_config.off_timeout_secs {
            if self.state != SessionState::ScreenOff {
                self.state = SessionState::ScreenOff;
                log::info!("Session: screen off (idle {}s)", idle_secs);
            }
        } else if idle_secs >= self.idle_config.dim_timeout_secs {
            if self.state != SessionState::Dimmed {
                self.state = SessionState::Dimmed;
                log::info!("Session: dimmed (idle {}s)", idle_secs);
            }
        } else if idle_secs > 30 {
            if self.state == SessionState::Active {
                self.state = SessionState::Idle;
            }
        }
    }

    /// Lock the session.
    pub fn lock(&mut self) {
        self.state = SessionState::Locked;
        self.lock_password_buffer.clear();
        self.lock_verifying = false;
        log::info!("Session locked");
    }

    /// Attempt to unlock with a password.
    /// Returns true if unlocked successfully.
    pub fn try_unlock(&mut self, password: &str) -> bool {
        // Check lockout
        if let Some(until) = self.lock_lockout_until {
            if Instant::now() < until {
                log::warn!("Session: unlock attempt during lockout");
                return false;
            }
            self.lock_lockout_until = None;
        }

        // Verify password against PAM/system auth.
        // In a real implementation, this calls into libpam.
        // For now, we use a placeholder that checks the MDB-OS user credential.
        let verified = verify_password(password);

        if verified {
            self.state = SessionState::Active;
            self.lock_failed_attempts = 0;
            self.lock_password_buffer.clear();
            self.last_input = Instant::now();
            log::info!("Session unlocked");
            true
        } else {
            self.lock_failed_attempts += 1;
            self.lock_password_buffer.clear();
            log::warn!("Session: unlock failed (attempt {})", self.lock_failed_attempts);

            // Lockout after 5 failed attempts
            if self.lock_failed_attempts >= 5 {
                let lockout = Duration::from_secs(30 * (self.lock_failed_attempts as u64 - 4));
                self.lock_lockout_until = Some(Instant::now() + lockout);
                log::warn!("Session: locked out for {}s", lockout.as_secs());
            }

            false
        }
    }

    /// Begin shutdown sequence.
    pub fn begin_shutdown(&mut self) {
        self.state = SessionState::ShuttingDown;
        log::info!("Session: shutting down");
    }

    /// Save session state to disk.
    pub fn save_session(&self, data: &SessionData) -> Result<(), String> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        if let Some(parent) = self.save_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create session dir: {}", e))?;
        }

        std::fs::write(&self.save_path, json)
            .map_err(|e| format!("Failed to write session file: {}", e))?;

        log::info!("Session saved to {}", self.save_path.display());
        Ok(())
    }

    /// Load session state from disk.
    pub fn load_session(&self) -> Option<SessionData> {
        let json = std::fs::read_to_string(&self.save_path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// Whether the session is currently locked.
    pub fn is_locked(&self) -> bool {
        self.state == SessionState::Locked
    }

    /// Whether the session is shutting down.
    pub fn is_shutting_down(&self) -> bool {
        self.state == SessionState::ShuttingDown
    }

    /// Get the current screen brightness multiplier (1.0 = full, dim_brightness when dimmed).
    pub fn brightness(&self) -> f64 {
        match self.state {
            SessionState::Dimmed => self.idle_config.dim_brightness,
            SessionState::ScreenOff => 0.0,
            _ => 1.0,
        }
    }
}

/// Verify a password against the system.
///
/// In production, this uses PAM (Pluggable Authentication Modules):
/// ```rust,ignore
/// use pam::Client;
/// let mut client = Client::with_password("mdb-desktop").unwrap();
/// client.conversation_mut().set_credentials(username, password);
/// client.authenticate().is_ok()
/// ```
///
/// For now, returns false (always fails) — actual PAM integration
/// requires libpam-dev and runtime PAM configuration.
fn verify_password(_password: &str) -> bool {
    // TODO: integrate with PAM for real authentication
    // This is intentionally not a hardcoded password — security matters.
    log::debug!("Password verification: PAM integration pending");
    false
}
