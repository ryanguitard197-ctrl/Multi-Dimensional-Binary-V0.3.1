//! # Screenshot Tool
//!
//! Built-in screenshot capture for MDB Desktop.
//!
//! ## Modes
//!
//! - **Full screen** — Capture all outputs (stitched together for multi-monitor).
//! - **Single output** — Capture one specific monitor.
//! - **Window** — Capture the focused window only.
//! - **Region** — Interactive rectangular selection.
//!
//! ## Output
//!
//! Screenshots are saved as PNG to `~/Pictures/Screenshots/` (or MDBFS if mounted).
//! The clipboard is also updated so you can paste immediately.
//!
//! ## Keybindings
//!
//! - `Print Screen` → Full screen
//! - `Alt+Print Screen` → Focused window
//! - `Super+Shift+S` → Interactive region select

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Screenshot capture mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Capture all outputs.
    FullScreen,
    /// Capture a single output.
    SingleOutput(u64),
    /// Capture the focused window.
    FocusedWindow,
    /// Interactive region selection.
    Region,
}

/// A rectangular region for capture.
#[derive(Debug, Clone, Copy)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Interactive region selection state.
#[derive(Debug)]
pub enum RegionSelectState {
    /// Not selecting.
    Inactive,
    /// Waiting for first click (start point).
    WaitingStart,
    /// Dragging (start point set, tracking end point).
    Dragging {
        start_x: i32,
        start_y: i32,
        current_x: i32,
        current_y: i32,
    },
    /// Selection complete.
    Complete(CaptureRegion),
}

/// Captured screenshot data.
pub struct ScreenshotData {
    /// RGBA pixel data.
    pub pixels: Vec<u8>,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl ScreenshotData {
    /// Encode as PNG bytes.
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        let mut buf = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut buf, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| format!("PNG header error: {}", e))?;
            writer
                .write_image_data(&self.pixels)
                .map_err(|e| format!("PNG write error: {}", e))?;
        }
        Ok(buf)
    }

    /// Save to a file. Creates parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        let png_data = self.to_png()?;
        std::fs::write(path, png_data)
            .map_err(|e| format!("Failed to write screenshot: {}", e))?;
        log::info!("Screenshot saved: {}", path.display());
        Ok(())
    }
}

/// The screenshot tool.
pub struct ScreenshotTool {
    /// Directory to save screenshots.
    pub save_dir: PathBuf,
    /// Current region selection state.
    pub region_state: RegionSelectState,
    /// Whether to copy to clipboard after capture.
    pub copy_to_clipboard: bool,
    /// Whether to send a notification after capture.
    pub notify: bool,
}

impl ScreenshotTool {
    pub fn new() -> Self {
        let save_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join("Pictures").join("Screenshots")
        } else {
            PathBuf::from("/tmp/mdb-screenshots")
        };

        Self {
            save_dir,
            region_state: RegionSelectState::Inactive,
            copy_to_clipboard: true,
            notify: true,
        }
    }

    /// Generate a filename for a new screenshot.
    pub fn next_filename(&self) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.save_dir.join(format!("mdb-screenshot-{}.png", timestamp))
    }

    /// Start an interactive region selection.
    pub fn start_region_select(&mut self) {
        self.region_state = RegionSelectState::WaitingStart;
        log::info!("Region select: click and drag to select area");
    }

    /// Handle a pointer press during region selection.
    pub fn region_press(&mut self, x: i32, y: i32) {
        if matches!(self.region_state, RegionSelectState::WaitingStart) {
            self.region_state = RegionSelectState::Dragging {
                start_x: x,
                start_y: y,
                current_x: x,
                current_y: y,
            };
        }
    }

    /// Handle a pointer motion during region selection.
    pub fn region_motion(&mut self, x: i32, y: i32) {
        if let RegionSelectState::Dragging { current_x, current_y, .. } = &mut self.region_state {
            *current_x = x;
            *current_y = y;
        }
    }

    /// Handle a pointer release during region selection.
    pub fn region_release(&mut self) {
        if let RegionSelectState::Dragging { start_x, start_y, current_x, current_y } = self.region_state {
            let x = start_x.min(current_x);
            let y = start_y.min(current_y);
            let w = (start_x - current_x).unsigned_abs();
            let h = (start_y - current_y).unsigned_abs();

            if w > 5 && h > 5 {
                self.region_state = RegionSelectState::Complete(CaptureRegion {
                    x, y, width: w, height: h,
                });
                log::info!("Region selected: {}x{} at ({}, {})", w, h, x, y);
            } else {
                // Too small — cancel
                self.region_state = RegionSelectState::Inactive;
                log::info!("Region select cancelled (too small)");
            }
        }
    }

    /// Cancel region selection.
    pub fn cancel_region(&mut self) {
        self.region_state = RegionSelectState::Inactive;
    }

    /// Get the current selection rectangle (for overlay rendering).
    pub fn selection_rect(&self) -> Option<CaptureRegion> {
        match &self.region_state {
            RegionSelectState::Dragging { start_x, start_y, current_x, current_y } => {
                let x = (*start_x).min(*current_x);
                let y = (*start_y).min(*current_y);
                let w = (*start_x - *current_x).unsigned_abs();
                let h = (*start_y - *current_y).unsigned_abs();
                Some(CaptureRegion { x, y, width: w, height: h })
            }
            RegionSelectState::Complete(region) => Some(*region),
            _ => None,
        }
    }

    /// Check if a region selection is in progress (for input interception).
    pub fn is_selecting(&self) -> bool {
        matches!(
            self.region_state,
            RegionSelectState::WaitingStart | RegionSelectState::Dragging { .. }
        )
    }
}
