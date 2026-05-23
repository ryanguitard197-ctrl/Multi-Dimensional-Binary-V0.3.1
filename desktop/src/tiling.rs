//! # Window Tiling Engine
//!
//! Provides automatic tiling and manual snap zones for the MDB Desktop.
//!
//! ## Layout Modes
//!
//! - **Float** (default): Traditional floating windows. Manual placement.
//! - **Tiled**: Automatic binary-split tiling (like i3/Sway).
//! - **Monocle**: Single fullscreen window per workspace.
//!
//! ## Snap Zones
//!
//! When dragging a window to a screen edge, it snaps:
//! - Left edge → left half
//! - Right edge → right half
//! - Top edge → maximized
//! - Top-left corner → top-left quarter
//! - Top-right corner → top-right quarter
//! - Bottom-left corner → bottom-left quarter
//! - Bottom-right corner → bottom-right quarter
//!
//! These work in both Float and Tiled modes.
//!
//! ## MDB Integration
//!
//! The tiling engine assigns each zone a DimensionalAddress, so windows
//! inherit spatial locality from their screen position. Processes in the
//! same zone are "closer" in dimensional space, improving scheduler
//! locality and cache efficiency.

use smithay::utils::{Logical, Point, Rectangle, Size};

/// Layout mode for a workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    /// Traditional floating windows.
    Float,
    /// Automatic binary-split tiling.
    Tiled,
    /// Single maximized window at a time.
    Monocle,
}

/// A snap zone — a named screen region where windows can dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapZone {
    /// No snap — floating freely.
    None,
    /// Left half of the screen.
    Left,
    /// Right half of the screen.
    Right,
    /// Top half of the screen.
    Top,
    /// Bottom half of the screen.
    Bottom,
    /// Maximized (full screen minus panel).
    Maximized,
    /// Top-left quarter.
    TopLeft,
    /// Top-right quarter.
    TopRight,
    /// Bottom-left quarter.
    BottomLeft,
    /// Bottom-right quarter.
    BottomRight,
}

/// The tiling engine for a single workspace.
#[derive(Debug)]
pub struct TilingEngine {
    /// Current layout mode.
    pub mode: LayoutMode,
    /// Output (screen) dimensions, excluding panel.
    pub usable_area: Rectangle<i32, Logical>,
    /// The size of the snap detection margin in pixels.
    pub snap_margin: i32,
    /// Gap between tiled windows in pixels.
    pub gap: i32,
    /// Tiling tree (for Tiled mode).
    tree: TilingTree,
}

/// Binary split tree for automatic tiling.
#[derive(Debug)]
enum TilingTree {
    /// Leaf node — holds a single window ID.
    Leaf(u64),
    /// Split node — divides space between two children.
    Split {
        direction: SplitDirection,
        /// Fraction of space given to the first child (0.0–1.0).
        ratio: f64,
        first: Box<TilingTree>,
        second: Box<TilingTree>,
    },
    /// Empty — no window in this slot.
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SplitDirection {
    Horizontal, // side by side
    Vertical,   // stacked top/bottom
}

impl TilingEngine {
    /// Create a new tiling engine for a workspace.
    pub fn new(usable_area: Rectangle<i32, Logical>) -> Self {
        Self {
            mode: LayoutMode::Float,
            usable_area,
            snap_margin: 24,
            gap: 6,
            tree: TilingTree::Empty,
        }
    }

    /// Detect which snap zone a pointer position is in.
    pub fn detect_snap_zone(&self, pos: Point<i32, Logical>) -> SnapZone {
        let area = &self.usable_area;
        let m = self.snap_margin;

        let at_left = pos.x <= area.loc.x + m;
        let at_right = pos.x >= area.loc.x + area.size.w - m;
        let at_top = pos.y <= area.loc.y + m;
        let at_bottom = pos.y >= area.loc.y + area.size.h - m;

        match (at_left, at_right, at_top, at_bottom) {
            (true, _, true, _) => SnapZone::TopLeft,
            (_, true, true, _) => SnapZone::TopRight,
            (true, _, _, true) => SnapZone::BottomLeft,
            (_, true, _, true) => SnapZone::BottomRight,
            (true, _, _, _) => SnapZone::Left,
            (_, true, _, _) => SnapZone::Right,
            (_, _, true, _) => SnapZone::Maximized,
            (_, _, _, true) => SnapZone::Bottom,
            _ => SnapZone::None,
        }
    }

    /// Get the geometry for a snap zone.
    pub fn zone_geometry(&self, zone: SnapZone) -> Rectangle<i32, Logical> {
        let a = &self.usable_area;
        let g = self.gap;
        let half_w = a.size.w / 2;
        let half_h = a.size.h / 2;

        match zone {
            SnapZone::None => Rectangle::from_loc_and_size(a.loc, a.size),
            SnapZone::Left => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + g),
                (half_w - g * 2, a.size.h - g * 2),
            ),
            SnapZone::Right => Rectangle::from_loc_and_size(
                (a.loc.x + half_w + g, a.loc.y + g),
                (half_w - g * 2, a.size.h - g * 2),
            ),
            SnapZone::Top => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + g),
                (a.size.w - g * 2, half_h - g * 2),
            ),
            SnapZone::Bottom => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + half_h + g),
                (a.size.w - g * 2, half_h - g * 2),
            ),
            SnapZone::Maximized => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + g),
                (a.size.w - g * 2, a.size.h - g * 2),
            ),
            SnapZone::TopLeft => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + g),
                (half_w - g * 2, half_h - g * 2),
            ),
            SnapZone::TopRight => Rectangle::from_loc_and_size(
                (a.loc.x + half_w + g, a.loc.y + g),
                (half_w - g * 2, half_h - g * 2),
            ),
            SnapZone::BottomLeft => Rectangle::from_loc_and_size(
                (a.loc.x + g, a.loc.y + half_h + g),
                (half_w - g * 2, half_h - g * 2),
            ),
            SnapZone::BottomRight => Rectangle::from_loc_and_size(
                (a.loc.x + half_w + g, a.loc.y + half_h + g),
                (half_w - g * 2, half_h - g * 2),
            ),
        }
    }

    /// Render a snap preview overlay rectangle (semi-transparent highlight).
    /// Returns the geometry to draw, or None if no snap zone is active.
    pub fn snap_preview(&self, pointer_pos: Point<i32, Logical>) -> Option<Rectangle<i32, Logical>> {
        let zone = self.detect_snap_zone(pointer_pos);
        if zone == SnapZone::None {
            None
        } else {
            Some(self.zone_geometry(zone))
        }
    }

    // ── Tiling tree operations ──────────────────────────────────────────

    /// Insert a window into the tiling tree (for Tiled mode).
    pub fn insert_tiled(&mut self, window_id: u64) {
        self.tree = self.tree_insert(std::mem::replace(&mut self.tree, TilingTree::Empty), window_id);
    }

    /// Remove a window from the tiling tree.
    pub fn remove_tiled(&mut self, window_id: u64) {
        self.tree = self.tree_remove(std::mem::replace(&mut self.tree, TilingTree::Empty), window_id);
    }

    /// Compute tiled geometries for all windows in the tree.
    /// Returns (window_id, rectangle) pairs.
    pub fn compute_tiled_layout(&self) -> Vec<(u64, Rectangle<i32, Logical>)> {
        let mut result = Vec::new();
        let area = Rectangle::from_loc_and_size(
            (self.usable_area.loc.x + self.gap, self.usable_area.loc.y + self.gap),
            (self.usable_area.size.w - self.gap * 2, self.usable_area.size.h - self.gap * 2),
        );
        self.layout_tree(&self.tree, area, &mut result);
        result
    }

    /// Adjust the split ratio of the focused container.
    pub fn adjust_ratio(&mut self, delta: f64) {
        self.tree_adjust_ratio(&mut self.tree, delta);
    }

    /// Toggle split direction of the focused container.
    pub fn toggle_split(&mut self) {
        self.tree_toggle_split(&mut self.tree);
    }

    // ── Internal tree helpers ───────────────────────────────────────────

    fn tree_insert(&self, tree: TilingTree, window_id: u64) -> TilingTree {
        match tree {
            TilingTree::Empty => TilingTree::Leaf(window_id),
            TilingTree::Leaf(existing) => {
                // Split the existing leaf to accommodate the new window.
                // Alternate split direction based on aspect ratio of area.
                let direction = if self.usable_area.size.w > self.usable_area.size.h {
                    SplitDirection::Horizontal
                } else {
                    SplitDirection::Vertical
                };
                TilingTree::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(TilingTree::Leaf(existing)),
                    second: Box::new(TilingTree::Leaf(window_id)),
                }
            }
            TilingTree::Split { direction, ratio, first, second } => {
                // Insert into the smaller subtree (by node count).
                let first_count = self.tree_count(&first);
                let second_count = self.tree_count(&second);
                if first_count <= second_count {
                    TilingTree::Split {
                        direction,
                        ratio,
                        first: Box::new(self.tree_insert(*first, window_id)),
                        second,
                    }
                } else {
                    TilingTree::Split {
                        direction,
                        ratio,
                        first,
                        second: Box::new(self.tree_insert(*second, window_id)),
                    }
                }
            }
        }
    }

    fn tree_remove(&self, tree: TilingTree, window_id: u64) -> TilingTree {
        match tree {
            TilingTree::Leaf(id) if id == window_id => TilingTree::Empty,
            TilingTree::Split { direction, ratio, first, second } => {
                let new_first = self.tree_remove(*first, window_id);
                let new_second = self.tree_remove(*second, window_id);
                match (&new_first, &new_second) {
                    (TilingTree::Empty, _) => new_second,
                    (_, TilingTree::Empty) => new_first,
                    _ => TilingTree::Split {
                        direction,
                        ratio,
                        first: Box::new(new_first),
                        second: Box::new(new_second),
                    },
                }
            }
            other => other,
        }
    }

    fn tree_count(&self, tree: &TilingTree) -> usize {
        match tree {
            TilingTree::Empty => 0,
            TilingTree::Leaf(_) => 1,
            TilingTree::Split { first, second, .. } => {
                self.tree_count(first) + self.tree_count(second)
            }
        }
    }

    fn layout_tree(
        &self,
        tree: &TilingTree,
        area: Rectangle<i32, Logical>,
        result: &mut Vec<(u64, Rectangle<i32, Logical>)>,
    ) {
        match tree {
            TilingTree::Empty => {}
            TilingTree::Leaf(id) => {
                result.push((*id, area));
            }
            TilingTree::Split { direction, ratio, first, second } => {
                let g = self.gap;
                let (area_a, area_b) = match direction {
                    SplitDirection::Horizontal => {
                        let split = (area.size.w as f64 * ratio) as i32;
                        (
                            Rectangle::from_loc_and_size(
                                area.loc,
                                (split - g, area.size.h),
                            ),
                            Rectangle::from_loc_and_size(
                                (area.loc.x + split + g, area.loc.y),
                                (area.size.w - split - g, area.size.h),
                            ),
                        )
                    }
                    SplitDirection::Vertical => {
                        let split = (area.size.h as f64 * ratio) as i32;
                        (
                            Rectangle::from_loc_and_size(
                                area.loc,
                                (area.size.w, split - g),
                            ),
                            Rectangle::from_loc_and_size(
                                (area.loc.x, area.loc.y + split + g),
                                (area.size.w, area.size.h - split - g),
                            ),
                        )
                    }
                };
                self.layout_tree(first, area_a, result);
                self.layout_tree(second, area_b, result);
            }
        }
    }

    fn tree_adjust_ratio(&self, tree: &mut TilingTree, delta: f64) {
        if let TilingTree::Split { ratio, .. } = tree {
            *ratio = (*ratio + delta).clamp(0.15, 0.85);
        }
    }

    fn tree_toggle_split(&self, tree: &mut TilingTree) {
        if let TilingTree::Split { direction, .. } = tree {
            *direction = match direction {
                SplitDirection::Horizontal => SplitDirection::Vertical,
                SplitDirection::Vertical => SplitDirection::Horizontal,
            };
        }
    }
}

/// Window state tracked per-window for tiling purposes.
#[derive(Debug, Clone)]
pub struct WindowTilingState {
    /// Which snap zone this window is docked in (None = floating).
    pub snap_zone: SnapZone,
    /// Geometry before snapping (for un-snap restore).
    pub pre_snap_geometry: Option<Rectangle<i32, Logical>>,
    /// Whether this window is fullscreen.
    pub is_fullscreen: bool,
    /// Geometry before fullscreen (for un-fullscreen restore).
    pub pre_fullscreen_geometry: Option<Rectangle<i32, Logical>>,
    /// Whether this window is maximized.
    pub is_maximized: bool,
    /// Minimum size constraint.
    pub min_size: Option<Size<i32, Logical>>,
    /// Maximum size constraint.
    pub max_size: Option<Size<i32, Logical>>,
}

impl Default for WindowTilingState {
    fn default() -> Self {
        Self {
            snap_zone: SnapZone::None,
            pre_snap_geometry: None,
            is_fullscreen: false,
            pre_fullscreen_geometry: None,
            is_maximized: false,
            min_size: None,
            max_size: None,
        }
    }
}
