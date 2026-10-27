//! Place the OS monitor layout onto the millimetre canvas (R-LAYOUT-4).
//!
//! Windows already moves the cursor across monitors in pixels. When two panels
//! differ in size or vertical offset, that pixel row is not the same physical
//! height. `crossing_correction` is the pixel position that keeps the height.

use crate::{auto_link, try_cross, Canvas, DeviceId, Edge, Monitor, MonitorId, SizeMm, SizePx, EDGE_EPS_MM};

/// One monitor as the OS reports it, in the same pixel space as the cursor.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OsPanel {
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub width_mm: f64,
    pub height_mm: f64,
    pub scale: f64,
}

/// Build a canvas whose millimetre positions follow the OS pixel arrangement.
pub fn canvas_from_os_panels(device: &str, panels: &[OsPanel]) -> Canvas {
    let mut canvas = Canvas::new();
    let Some(anchor) = anchor_panel(panels) else {
        return canvas;
    };
    let ax = anchor.width_mm / anchor.width.max(1) as f64;
    let ay = anchor.height_mm / anchor.height.max(1) as f64;
    let device_id = DeviceId(device.to_owned());
    for panel in panels {
        canvas.monitors.push(Monitor {
            id: MonitorId {
                device: device_id.clone(),
                local_index: panel.index,
            },
            origin_mm: (
                (panel.x - anchor.x) as f64 * ax,
                (panel.y - anchor.y) as f64 * ay,
            ),
            physical_mm: SizeMm {
                width: panel.width_mm,
                height: panel.height_mm,
            },
            pixels: SizePx {
                width: panel.width,
                height: panel.height,
            },
            scale: panel.scale,
            override_physical_mm: None,
        });
    }
    auto_link(&mut canvas, crate::DEFAULT_GAP_MM);
    canvas
}

/// Where the cursor should land on `to_index`, in screen pixels.
///
/// `None` when the edge is a wall, or when the OS landing is already within
/// 4 px of the physical landing.
pub fn crossing_correction(
    canvas: &Canvas,
    panels: &[OsPanel],
    from_index: u32,
    from_px: (i32, i32),
    to_index: u32,
    to_px: (i32, i32),
) -> Option<(i32, i32)> {
    if from_index == to_index {
        return None;
    }
    let from = panels.iter().find(|p| p.index == from_index)?;
    let to = panels.iter().find(|p| p.index == to_index)?;
    let from_mon = canvas.monitors.iter().find(|m| m.id.local_index == from_index)?;
    let heading = facing_edge(from, to);
    let local_px = (
        (from_px.0 - from.x) as f64,
        (from_px.1 - from.y) as f64,
    );
    let local_mm = from_mon.px_to_local_mm(local_px);
    let size = from_mon.effective_mm();
    let on_edge_mm = match heading {
        Edge::Left => (0.0, local_mm.1),
        Edge::Right => (size.width, local_mm.1),
        Edge::Top => (local_mm.0, 0.0),
        Edge::Bottom => (local_mm.0, size.height),
    };
    let hit = try_cross(canvas, &from_mon.id, on_edge_mm, heading, EDGE_EPS_MM)?;
    if hit.dest_monitor.local_index != to_index {
        return None;
    }
    let along_x = (to.x as f64 + hit.dest_px.0).round() as i32;
    let along_y = (to.y as f64 + hit.dest_px.1).round() as i32;
    // Keep Windows' distance past the edge. Only the position along the edge
    // is the physical height (or width) we came from.
    let (x, y) = match heading {
        Edge::Left | Edge::Right => (
            to_px.0,
            along_y.clamp(to.y + 1, to.y + to.height as i32 - 2),
        ),
        Edge::Top | Edge::Bottom => (
            along_x.clamp(to.x + 1, to.x + to.width as i32 - 2),
            to_px.1,
        ),
    };
    if (x - to_px.0).abs() < 4 && (y - to_px.1).abs() < 4 {
        return None;
    }
    Some((x, y))
}

fn anchor_panel(panels: &[OsPanel]) -> Option<&OsPanel> {
    panels.iter().find(|p| contains_origin(p)).or_else(|| {
        panels.iter().min_by_key(|p| (p.y, p.x))
    })
}

fn contains_origin(p: &OsPanel) -> bool {
    p.x <= 0 && p.y <= 0 && p.x + p.width as i32 > 0 && p.y + p.height as i32 > 0
}

fn facing_edge(from: &OsPanel, to: &OsPanel) -> Edge {
    let dx = (to.x as f64 + to.width as f64 / 2.0) - (from.x as f64 + from.width as f64 / 2.0);
    let dy = (to.y as f64 + to.height as f64 / 2.0) - (from.y as f64 + from.height as f64 / 2.0);
    if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            Edge::Right
        } else {
            Edge::Left
        }
    } else if dy >= 0.0 {
        Edge::Bottom
    } else {
        Edge::Top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(index: u32, x: i32, y: i32, w_mm: f64, h_mm: f64) -> OsPanel {
        OsPanel {
            index,
            x,
            y,
            width: 1920,
            height: 1080,
            width_mm: w_mm,
            height_mm: h_mm,
            scale: 1.0,
        }
    }

    #[test]
    fn same_density_keeps_the_pixel_row() {
        let panels = vec![panel(0, 0, 0, 530.0, 300.0), panel(1, 1920, -200, 530.0, 300.0)];
        let canvas = canvas_from_os_panels("win1", &panels);
        // Leave the left monitor 40 px inside the right edge, at screen y 100.
        let got = crossing_correction(&canvas, &panels, 0, (1880, 100), 1, (1922, 100));
        assert!(got.is_none(), "same density should not move the cursor: {got:?}");
    }

    #[test]
    fn different_height_keeps_millimetres() {
        // Left panel is physically taller, so the same pixel row is not the same height.
        let panels = vec![panel(0, 0, 0, 600.0, 340.0), panel(1, 1920, 0, 400.0, 225.0)];
        let canvas = canvas_from_os_panels("win1", &panels);
        let (x, y) = crossing_correction(&canvas, &panels, 0, (1880, 540), 1, (1922, 540)).unwrap();
        assert!(x > 1920 && x < 1920 + 1920);
        // 540/1080 of 340 mm is 170 mm. On the shorter panel that is 170/225 of 1080 px.
        let expect_y = 170.0 / 225.0 * 1080.0;
        assert!((y as f64 - expect_y).abs() < 3.0, "y {y} expected {expect_y}");
    }
}
