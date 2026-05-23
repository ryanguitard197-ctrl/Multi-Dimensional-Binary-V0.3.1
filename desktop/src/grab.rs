//! # Interactive Grabs
//!
//! Pointer grabs for moving and resizing windows.
//!
//! When the user holds Super+click (or drags a title bar), the compositor
//! enters a "grab" state that intercepts all pointer events until the
//! button is released.
//!
//! ## Move Grab
//!
//! Tracks the delta between the initial click position and the window's
//! top-left corner. Each pointer motion event updates the window position.
//! On release, checks for snap zones (edge docking).
//!
//! ## Resize Grab
//!
//! Initiated by Super+Right-click or by clicking a window edge/corner.
//! The resize edge is determined by where the click landed relative to
//! the window geometry. During the grab, the window geometry is
//! recalculated on each motion event, respecting min/max size hints.

use smithay::utils::{Logical, Point, Rectangle, Size};

/// Which edges are being resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeEdges {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl ResizeEdges {
    /// Detect which edges to resize based on click position within window.
    pub fn from_click(
        click: Point<i32, Logical>,
        window_geo: Rectangle<i32, Logical>,
        border: i32,
    ) -> Self {
        let rel_x = click.x - window_geo.loc.x;
        let rel_y = click.y - window_geo.loc.y;

        Self {
            top: rel_y < border,
            bottom: rel_y > window_geo.size.h - border,
            left: rel_x < border,
            right: rel_x > window_geo.size.w - border,
        }
    }

    /// Returns true if no edge is selected (click was in the interior).
    pub fn is_empty(&self) -> bool {
        !self.top && !self.bottom && !self.left && !self.right
    }

    /// Convert to an XDG resize edge value for the Wayland protocol.
    pub fn to_xdg_edge(&self) -> u32 {
        // XDG resize edge bits: top=1, bottom=2, left=4, right=8
        let mut edge = 0u32;
        if self.top { edge |= 1; }
        if self.bottom { edge |= 2; }
        if self.left { edge |= 4; }
        if self.right { edge |= 8; }
        edge
    }
}

/// State for an active move grab.
#[derive(Debug)]
pub struct MoveGrab {
    /// Window ID being moved.
    pub window_id: u64,
    /// Offset from pointer to window top-left at grab start.
    pub offset: Point<i32, Logical>,
    /// Window geometry at grab start (for undo on Escape).
    pub initial_geometry: Rectangle<i32, Logical>,
    /// Whether the window was snapped before the move started.
    pub was_snapped: bool,
    /// If dragging from a snapped state, the un-snap threshold in pixels.
    pub unsnap_threshold: i32,
    /// Whether we've exceeded the unsnap threshold.
    pub unsnapped: bool,
}

impl MoveGrab {
    pub fn new(
        window_id: u64,
        pointer: Point<i32, Logical>,
        window_geo: Rectangle<i32, Logical>,
        was_snapped: bool,
    ) -> Self {
        Self {
            window_id,
            offset: Point::from((
                pointer.x - window_geo.loc.x,
                pointer.y - window_geo.loc.y,
            )),
            initial_geometry: window_geo,
            was_snapped,
            unsnap_threshold: 20,
            unsnapped: !was_snapped,
        }
    }

    /// Compute the new window position for the current pointer position.
    pub fn compute_position(&mut self, pointer: Point<i32, Logical>) -> Point<i32, Logical> {
        if !self.unsnapped {
            // Check if we've moved far enough to unsnap
            let dx = (pointer.x - self.initial_geometry.loc.x - self.offset.x).abs();
            let dy = (pointer.y - self.initial_geometry.loc.y - self.offset.y).abs();
            if dx > self.unsnap_threshold || dy > self.unsnap_threshold {
                self.unsnapped = true;
                // Center the window on the cursor when unsnapping
                let half_w = self.initial_geometry.size.w / 2;
                self.offset = Point::from((half_w, 12)); // near top center of window
            }
        }

        Point::from((pointer.x - self.offset.x, pointer.y - self.offset.y))
    }

    /// Cancel the grab and return the original geometry.
    pub fn cancel(&self) -> Rectangle<i32, Logical> {
        self.initial_geometry
    }
}

/// State for an active resize grab.
#[derive(Debug)]
pub struct ResizeGrab {
    /// Window ID being resized.
    pub window_id: u64,
    /// Which edges are being resized.
    pub edges: ResizeEdges,
    /// Pointer position at grab start.
    pub start_pointer: Point<i32, Logical>,
    /// Window geometry at grab start.
    pub initial_geometry: Rectangle<i32, Logical>,
    /// Minimum window size.
    pub min_size: Size<i32, Logical>,
    /// Maximum window size.
    pub max_size: Size<i32, Logical>,
}

impl ResizeGrab {
    pub fn new(
        window_id: u64,
        pointer: Point<i32, Logical>,
        window_geo: Rectangle<i32, Logical>,
        edges: ResizeEdges,
        min_size: Option<Size<i32, Logical>>,
        max_size: Option<Size<i32, Logical>>,
    ) -> Self {
        Self {
            window_id,
            edges,
            start_pointer: pointer,
            initial_geometry: window_geo,
            min_size: min_size.unwrap_or_else(|| Size::from((120, 80))),
            max_size: max_size.unwrap_or_else(|| Size::from((8192, 8192))),
        }
    }

    /// Compute the new window geometry for the current pointer position.
    pub fn compute_geometry(&self, pointer: Point<i32, Logical>) -> Rectangle<i32, Logical> {
        let dx = pointer.x - self.start_pointer.x;
        let dy = pointer.y - self.start_pointer.y;

        let mut x = self.initial_geometry.loc.x;
        let mut y = self.initial_geometry.loc.y;
        let mut w = self.initial_geometry.size.w;
        let mut h = self.initial_geometry.size.h;

        if self.edges.right {
            w = (w + dx).clamp(self.min_size.w, self.max_size.w);
        }
        if self.edges.left {
            let new_w = (w - dx).clamp(self.min_size.w, self.max_size.w);
            x += w - new_w;
            w = new_w;
        }
        if self.edges.bottom {
            h = (h + dy).clamp(self.min_size.h, self.max_size.h);
        }
        if self.edges.top {
            let new_h = (h - dy).clamp(self.min_size.h, self.max_size.h);
            y += h - new_h;
            h = new_h;
        }

        Rectangle::from_loc_and_size((x, y), (w, h))
    }

    /// Cancel the grab and return the original geometry.
    pub fn cancel(&self) -> Rectangle<i32, Logical> {
        self.initial_geometry
    }
}

/// The active grab state for the compositor.
#[derive(Debug)]
pub enum ActiveGrab {
    /// No grab active — normal pointer operation.
    None,
    /// Moving a window.
    Move(MoveGrab),
    /// Resizing a window.
    Resize(ResizeGrab),
}

impl Default for ActiveGrab {
    fn default() -> Self {
        ActiveGrab::None
    }
}

impl ActiveGrab {
    /// Returns true if a grab is active.
    pub fn is_active(&self) -> bool {
        !matches!(self, ActiveGrab::None)
    }

    /// Get the window ID being grabbed (if any).
    pub fn window_id(&self) -> Option<u64> {
        match self {
            ActiveGrab::None => None,
            ActiveGrab::Move(g) => Some(g.window_id),
            ActiveGrab::Resize(g) => Some(g.window_id),
        }
    }
}
