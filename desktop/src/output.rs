//! # Output (Monitor) Management
//!
//! Handles multi-monitor setup, hotplugging, and per-output configuration.
//!
//! Each physical display is an "output" in Wayland terminology. The MDB
//! Desktop supports:
//!
//! - **Hotplug detection** — monitors can be connected/disconnected at runtime
//! - **Per-output workspaces** — each monitor can have its own workspace set
//! - **Output arrangement** — configurable positioning (left-of, right-of, above, below, mirror)
//! - **Scale factors** — per-output HiDPI scaling
//! - **Mode selection** — resolution and refresh rate per output
//!
//! ## MDB Integration
//!
//! Each output is assigned a dimensional coordinate range. Windows on different
//! monitors have different D3 (spatial) coordinates, giving the scheduler
//! awareness of physical screen locality.

use smithay::utils::{Logical, Physical, Point, Rectangle, Size, Transform};
use std::collections::HashMap;

/// Unique identifier for an output.
pub type OutputId = u64;

/// Arrangement position relative to another output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputPosition {
    /// To the left of the reference output.
    LeftOf(OutputId),
    /// To the right of the reference output.
    RightOf(OutputId),
    /// Above the reference output.
    Above(OutputId),
    /// Below the reference output.
    Below(OutputId),
    /// Mirror (clone) the reference output.
    Mirror(OutputId),
    /// Absolute position.
    Absolute(i32, i32),
    /// Primary output (first connected).
    Primary,
}

/// A display mode (resolution + refresh rate).
#[derive(Debug, Clone, PartialEq)]
pub struct OutputMode {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Refresh rate in millihertz (e.g. 60000 = 60 Hz).
    pub refresh_mhz: u32,
    /// Whether this is the preferred/native mode.
    pub preferred: bool,
}

impl OutputMode {
    /// Refresh rate in Hz.
    pub fn refresh_hz(&self) -> f64 {
        self.refresh_mhz as f64 / 1000.0
    }

    /// Physical size.
    pub fn size(&self) -> Size<u32, Physical> {
        Size::from((self.width, self.height))
    }
}

/// Configuration for a single output.
#[derive(Debug, Clone)]
pub struct OutputConfig {
    /// Output name (e.g. "HDMI-A-1", "eDP-1").
    pub name: String,
    /// Physical make/model.
    pub make: String,
    pub model: String,
    /// Available modes.
    pub available_modes: Vec<OutputMode>,
    /// Selected mode index.
    pub selected_mode: usize,
    /// Scale factor (1.0 = normal, 2.0 = HiDPI).
    pub scale: f64,
    /// Transform (rotation).
    pub transform: OutputTransform,
    /// Position relative to other outputs.
    pub position: OutputPosition,
    /// Whether this output is enabled.
    pub enabled: bool,
    /// Physical size in mm (for DPI computation).
    pub physical_width_mm: u32,
    pub physical_height_mm: u32,
}

/// Output rotation/flip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTransform {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    Flipped,
    FlippedRotate90,
    FlippedRotate180,
    FlippedRotate270,
}

impl OutputTransform {
    /// Convert to smithay Transform.
    pub fn to_smithay(&self) -> Transform {
        match self {
            Self::Normal => Transform::Normal,
            Self::Rotate90 => Transform::_90,
            Self::Rotate180 => Transform::_180,
            Self::Rotate270 => Transform::_270,
            Self::Flipped => Transform::Flipped,
            Self::FlippedRotate90 => Transform::Flipped90,
            Self::FlippedRotate180 => Transform::Flipped180,
            Self::FlippedRotate270 => Transform::Flipped270,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            make: String::new(),
            model: String::new(),
            available_modes: Vec::new(),
            selected_mode: 0,
            scale: 1.0,
            transform: OutputTransform::Normal,
            position: OutputPosition::Primary,
            enabled: true,
            physical_width_mm: 0,
            physical_height_mm: 0,
        }
    }
}

impl OutputConfig {
    /// Get the currently selected mode.
    pub fn current_mode(&self) -> Option<&OutputMode> {
        self.available_modes.get(self.selected_mode)
    }

    /// Compute logical size (physical size / scale).
    pub fn logical_size(&self) -> Size<i32, Logical> {
        if let Some(mode) = self.current_mode() {
            Size::from((
                (mode.width as f64 / self.scale) as i32,
                (mode.height as f64 / self.scale) as i32,
            ))
        } else {
            Size::from((1920, 1080)) // fallback
        }
    }

    /// Compute physical DPI from physical size and mode.
    pub fn dpi(&self) -> f64 {
        if let Some(mode) = self.current_mode() {
            if self.physical_width_mm > 0 {
                let inches = self.physical_width_mm as f64 / 25.4;
                mode.width as f64 / inches
            } else {
                96.0 // fallback
            }
        } else {
            96.0
        }
    }
}

/// Manages all connected outputs.
pub struct OutputManager {
    /// All known outputs.
    outputs: HashMap<OutputId, OutputConfig>,
    /// Next output ID.
    next_id: OutputId,
    /// Computed absolute positions (after layout).
    positions: HashMap<OutputId, Point<i32, Logical>>,
    /// The primary output ID.
    primary: Option<OutputId>,
    /// Panel height in pixels (reserved for the panel).
    panel_height: i32,
}

impl OutputManager {
    pub fn new(panel_height: u32) -> Self {
        Self {
            outputs: HashMap::new(),
            next_id: 1,
            positions: HashMap::new(),
            primary: None,
            panel_height: panel_height as i32,
        }
    }

    /// Add a new output. Returns its ID.
    pub fn add_output(&mut self, config: OutputConfig) -> OutputId {
        let id = self.next_id;
        self.next_id += 1;

        let is_primary = matches!(config.position, OutputPosition::Primary);
        self.outputs.insert(id, config);

        if is_primary || self.primary.is_none() {
            self.primary = Some(id);
        }

        self.recompute_layout();
        log::info!("Output {} added ({})", id, self.outputs[&id].name);
        id
    }

    /// Remove an output (disconnected).
    pub fn remove_output(&mut self, id: OutputId) {
        if let Some(config) = self.outputs.remove(&id) {
            log::info!("Output {} removed ({})", id, config.name);
            self.positions.remove(&id);
            if self.primary == Some(id) {
                self.primary = self.outputs.keys().next().copied();
            }
            self.recompute_layout();
        }
    }

    /// Update an output's configuration.
    pub fn update_output(&mut self, id: OutputId, config: OutputConfig) {
        self.outputs.insert(id, config);
        self.recompute_layout();
    }

    /// Get an output by ID.
    pub fn get(&self, id: OutputId) -> Option<&OutputConfig> {
        self.outputs.get(&id)
    }

    /// Get the primary output ID.
    pub fn primary_id(&self) -> Option<OutputId> {
        self.primary
    }

    /// Get the primary output config.
    pub fn primary(&self) -> Option<&OutputConfig> {
        self.primary.and_then(|id| self.outputs.get(&id))
    }

    /// Get the absolute position of an output.
    pub fn position(&self, id: OutputId) -> Point<i32, Logical> {
        self.positions.get(&id).copied().unwrap_or_default()
    }

    /// Get the full rectangle (position + logical size) of an output.
    pub fn geometry(&self, id: OutputId) -> Rectangle<i32, Logical> {
        let pos = self.position(id);
        let size = self.outputs.get(&id)
            .map(|c| c.logical_size())
            .unwrap_or_else(|| Size::from((1920, 1080)));
        Rectangle::from_loc_and_size(pos, size)
    }

    /// Get the usable area of an output (minus panel).
    pub fn usable_area(&self, id: OutputId) -> Rectangle<i32, Logical> {
        let geo = self.geometry(id);
        Rectangle::from_loc_and_size(
            (geo.loc.x, geo.loc.y + self.panel_height),
            (geo.size.w, geo.size.h - self.panel_height),
        )
    }

    /// Get the total bounding box of all outputs.
    pub fn total_geometry(&self) -> Rectangle<i32, Logical> {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for id in self.outputs.keys() {
            let geo = self.geometry(*id);
            min_x = min_x.min(geo.loc.x);
            min_y = min_y.min(geo.loc.y);
            max_x = max_x.max(geo.loc.x + geo.size.w);
            max_y = max_y.max(geo.loc.y + geo.size.h);
        }

        if min_x == i32::MAX {
            // No outputs
            Rectangle::from_loc_and_size((0, 0), (1920, 1080))
        } else {
            Rectangle::from_loc_and_size(
                (min_x, min_y),
                (max_x - min_x, max_y - min_y),
            )
        }
    }

    /// Find which output a point is on.
    pub fn output_at(&self, point: Point<i32, Logical>) -> Option<OutputId> {
        for (id, _) in &self.outputs {
            let geo = self.geometry(*id);
            if geo.contains(point) {
                return Some(*id);
            }
        }
        // If point is outside all outputs, find nearest
        self.primary
    }

    /// List all output IDs.
    pub fn output_ids(&self) -> Vec<OutputId> {
        self.outputs.keys().copied().collect()
    }

    /// Number of connected outputs.
    pub fn count(&self) -> usize {
        self.outputs.len()
    }

    /// Recompute absolute positions based on the arrangement config.
    fn recompute_layout(&mut self) {
        self.positions.clear();

        // Place primary at origin
        if let Some(primary_id) = self.primary {
            self.positions.insert(primary_id, Point::from((0, 0)));

            // Place remaining outputs relative to their references
            let ids: Vec<OutputId> = self.outputs.keys().copied().collect();
            for id in ids {
                if id == primary_id {
                    continue;
                }
                if let Some(config) = self.outputs.get(&id) {
                    let pos = self.compute_position(id, &config.position);
                    self.positions.insert(id, pos);
                }
            }
        }
    }

    fn compute_position(&self, _id: OutputId, position: &OutputPosition) -> Point<i32, Logical> {
        match position {
            OutputPosition::RightOf(ref_id) => {
                let ref_geo = self.geometry(*ref_id);
                Point::from((ref_geo.loc.x + ref_geo.size.w, ref_geo.loc.y))
            }
            OutputPosition::LeftOf(ref_id) => {
                let ref_geo = self.geometry(*ref_id);
                let my_size = self.outputs.get(&_id)
                    .map(|c| c.logical_size())
                    .unwrap_or_else(|| Size::from((1920, 1080)));
                Point::from((ref_geo.loc.x - my_size.w, ref_geo.loc.y))
            }
            OutputPosition::Above(ref_id) => {
                let ref_geo = self.geometry(*ref_id);
                let my_size = self.outputs.get(&_id)
                    .map(|c| c.logical_size())
                    .unwrap_or_else(|| Size::from((1920, 1080)));
                Point::from((ref_geo.loc.x, ref_geo.loc.y - my_size.h))
            }
            OutputPosition::Below(ref_id) => {
                let ref_geo = self.geometry(*ref_id);
                Point::from((ref_geo.loc.x, ref_geo.loc.y + ref_geo.size.h))
            }
            OutputPosition::Mirror(ref_id) => {
                self.position(*ref_id)
            }
            OutputPosition::Absolute(x, y) => Point::from((*x, *y)),
            OutputPosition::Primary => Point::from((0, 0)),
        }
    }
}
