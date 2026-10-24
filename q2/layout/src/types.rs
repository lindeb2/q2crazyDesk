use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonitorId {
    pub device: DeviceId,
    pub local_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

impl Edge {
    pub fn opposite(self) -> Self {
        match self {
            Edge::Left => Edge::Right,
            Edge::Right => Edge::Left,
            Edge::Top => Edge::Bottom,
            Edge::Bottom => Edge::Top,
        }
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Edge::Left | Edge::Right)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SizeMm {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SizePx {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectMm {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectMm {
    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }

    pub fn contains(self, p: (f64, f64), eps: f64) -> bool {
        p.0 >= self.x - eps
            && p.0 <= self.right() + eps
            && p.1 >= self.y - eps
            && p.1 <= self.bottom() + eps
    }

    pub fn overlap_1d(a0: f64, a1: f64, b0: f64, b1: f64) -> Option<(f64, f64)> {
        let lo = a0.max(b0);
        let hi = a1.min(b1);
        if hi > lo {
            Some((lo, hi))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Monitor {
    pub id: MonitorId,
    /// Top-left of the panel on the shared canvas, in millimetres.
    pub origin_mm: (f64, f64),
    pub physical_mm: SizeMm,
    pub pixels: SizePx,
    /// Windows per-monitor scale or macOS backing-scale (1.0, 1.25, 2.0, …).
    pub scale: f64,
    pub override_physical_mm: Option<SizeMm>,
}

impl Monitor {
    pub fn effective_mm(&self) -> SizeMm {
        self.override_physical_mm.unwrap_or(self.physical_mm)
    }

    pub fn rect(&self) -> RectMm {
        let s = self.effective_mm();
        RectMm {
            x: self.origin_mm.0,
            y: self.origin_mm.1,
            w: s.width,
            h: s.height,
        }
    }

    pub fn to_canvas(&self, local_mm: (f64, f64)) -> (f64, f64) {
        (
            self.origin_mm.0 + local_mm.0,
            self.origin_mm.1 + local_mm.1,
        )
    }

    pub fn from_canvas(&self, canvas_mm: (f64, f64)) -> (f64, f64) {
        (
            canvas_mm.0 - self.origin_mm.0,
            canvas_mm.1 - self.origin_mm.1,
        )
    }

    /// Logical pixels (what the OS reports after scaling) → local millimetres.
    pub fn px_to_local_mm(&self, px: (f64, f64)) -> (f64, f64) {
        let s = self.effective_mm();
        let w = self.pixels.width.max(1) as f64;
        let h = self.pixels.height.max(1) as f64;
        (px.0 / w * s.width, px.1 / h * s.height)
    }

    pub fn local_mm_to_px(&self, mm: (f64, f64)) -> (f64, f64) {
        let s = self.effective_mm();
        let w = self.pixels.width.max(1) as f64;
        let h = self.pixels.height.max(1) as f64;
        (
            mm.0 / s.width.max(1e-6) * w,
            mm.1 / s.height.max(1e-6) * h,
        )
    }
}

/// Along-edge range is local millimetres: from the top on a vertical edge,
/// from the left on a horizontal edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeLink {
    pub from: MonitorId,
    pub from_edge: Edge,
    pub from_along0: f64,
    pub from_along1: f64,
    pub to: MonitorId,
    pub to_edge: Edge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsideMode {
    Windowed,
    Fullscreen { host_monitor_index: u32 },
    ChosenMonitor { host_monitor_index: u32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Placement {
    Beside,
    Inside {
        host: DeviceId,
        mode: InsideMode,
        /// Windowed Inside region in the host monitor's local millimetres.
        window_mm: Option<RectMm>,
    },
    Lent {
        borrower: DeviceId,
        monitor: MonitorId,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Canvas {
    pub monitors: Vec<Monitor>,
    pub placements: BTreeMap<DeviceId, Placement>,
    pub links: Vec<EdgeLink>,
    pub version: u64,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            monitors: Vec::new(),
            placements: BTreeMap::new(),
            links: Vec::new(),
            version: 0,
        }
    }

    pub fn monitor(&self, id: &MonitorId) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.id == *id)
    }

    pub fn monitors_of<'a>(&'a self, device: &'a DeviceId) -> impl Iterator<Item = &'a Monitor> {
        self.monitors.iter().filter(move |m| m.id.device == *device)
    }

    pub fn placement(&self, device: &DeviceId) -> Placement {
        self.placements
            .get(device)
            .cloned()
            .unwrap_or(Placement::Beside)
    }
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}
