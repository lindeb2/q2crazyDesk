//! Millimetre canvas, physical edge crossing, Inside regions, and profiles.
//!
//! Crossing maths follow LittleBigMouse (GPL-3.0),
//! https://github.com/mgth/LittleBigMouse `rust/crates`, studied 2026-10-24:
//! monitors live in real millimetres, and a point 100 mm from the top of one
//! screen arrives 100 mm from the top of its neighbour. Rewritten; the Win32
//! hook is not copied.

mod canvas;
mod os_layout;
mod profile;
mod types;

pub use canvas::{auto_link, try_cross, Crossing, EDGE_EPS_MM};
pub use os_layout::{canvas_from_os_panels, crossing_correction, OsPanel};
pub use profile::{select_profile, Profile};
pub use types::{
    Canvas, DeviceId, Edge, EdgeLink, InsideMode, Monitor, MonitorId, Placement, RectMm, SizeMm,
    SizePx,
};

pub const DEFAULT_GAP_MM: f64 = 8.0;
