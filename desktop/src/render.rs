//! # Desktop Rendering
//!
//! Renders the MDB desktop: background, panel, window decorations, cursor.
//! Uses smithay's Gles2 renderer for GPU-accelerated compositing.

use crate::state::MdbDesktop;

/// Information rendered on the panel.
pub struct PanelInfo {
    pub current_workspace: u32,
    pub workspace_count: u32,
    pub window_count: usize,
    pub clock_text: String,
    pub mdb_status: String,
}

impl MdbDesktop {
    /// Gather panel info from current state.
    pub fn panel_info(&self) -> PanelInfo {
        let now = chrono_now();
        let mdb_status = if self.mdbfs_mounted {
            format!(
                "MDB: {} folded | {} bytes dimensional",
                self.mdb_fold_count, self.mdb_total_dimensional_bytes
            )
        } else {
            "MDB: unmounted".to_string()
        };

        PanelInfo {
            current_workspace: self.current_workspace,
            workspace_count: self.config.panel.workspace_count,
            window_count: self.space.elements().count(),
            clock_text: now,
            mdb_status,
        }
    }
}

/// Simple clock string (no chrono dependency — uses libc).
fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    format!("{:02}:{:02}", hours, minutes)
}

/// Desktop rendering pipeline.
///
/// Called once per frame by the compositor's render loop.
/// The actual GPU calls depend on the smithay renderer backend (Gles2/Vulkan).
///
/// ## Render order
///
/// 1. Clear to background color (or draw wallpaper texture)
/// 2. Draw each mapped window in bottom-to-top stacking order
/// 3. Draw window borders/decorations for server-side decorated windows
/// 4. Draw the panel overlay (opaque bar at top/bottom)
/// 5. Draw the hardware cursor
///
/// ## Integration with smithay
///
/// In the actual event loop (main.rs), rendering looks like:
///
/// ```rust,ignore
/// // In the winit/udev render callback:
/// renderer.render(output_size, Transform::Normal, |renderer, frame| {
///     // 1. Background
///     frame.clear(state.config.theme.background_color, &[]);
///
///     // 2. Windows — smithay's Space handles this
///     state.space.render_output(renderer, &output, age, background_color)?;
///
///     // 3. Panel — render as a solid rect + text
///     render_panel(renderer, frame, &state.panel_info(), &state.config);
///
///     // 4. Cursor
///     // Handled by smithay's cursor rendering
/// });
/// ```
///
/// The panel rendering uses a pre-rasterized glyph atlas (via `fontdue`)
/// for text, avoiding runtime font shaping overhead.
pub struct RenderPipeline {
    /// Pre-rasterized font glyphs for panel text.
    _glyph_cache: Vec<u8>,
    /// Panel texture (re-rendered when info changes).
    _panel_dirty: bool,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self {
            _glyph_cache: Vec::new(),
            _panel_dirty: true,
        }
    }
}

/// Window decoration style.
pub struct WindowDecoration {
    pub border_color: [f32; 4],
    pub border_width: u32,
    pub title_height: u32,
    pub corner_radius: u32,
}

impl WindowDecoration {
    pub fn focused(config: &crate::config::ThemeConfig) -> Self {
        Self {
            border_color: config.border_focused,
            border_width: config.border_width,
            title_height: 0, // CSD — no server title bar
            corner_radius: config.corner_radius,
        }
    }

    pub fn unfocused(config: &crate::config::ThemeConfig) -> Self {
        Self {
            border_color: config.border_unfocused,
            border_width: config.border_width,
            title_height: 0,
            corner_radius: config.corner_radius,
        }
    }
}
