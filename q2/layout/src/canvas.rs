use crate::types::{Canvas, Edge, EdgeLink, Monitor, MonitorId, RectMm};
use crate::DEFAULT_GAP_MM;

pub const EDGE_EPS_MM: f64 = 1.5;

#[derive(Clone, Debug, PartialEq)]
pub struct Crossing {
    pub dest_monitor: MonitorId,
    pub dest_local_mm: (f64, f64),
    pub dest_px: (f64, f64),
}

fn along_local(edge: Edge, local_mm: (f64, f64)) -> f64 {
    if edge.is_vertical() {
        local_mm.1
    } else {
        local_mm.0
    }
}

fn on_edge(size: crate::SizeMm, local_mm: (f64, f64), heading: Edge, eps: f64) -> bool {
    match heading {
        Edge::Left => local_mm.0 <= eps,
        Edge::Right => local_mm.0 >= size.width - eps,
        Edge::Top => local_mm.1 <= eps,
        Edge::Bottom => local_mm.1 >= size.height - eps,
    }
}

/// Map a canvas point onto the destination, just inside `to_edge`.
fn land(dest: &Monitor, canvas_mm: (f64, f64), to_edge: Edge, inset: f64) -> (f64, f64) {
    let size = dest.effective_mm();
    let mut local = dest.from_canvas(canvas_mm);
    match to_edge {
        Edge::Left => local.0 = inset,
        Edge::Right => local.0 = size.width - inset,
        Edge::Top => local.1 = inset,
        Edge::Bottom => local.1 = size.height - inset,
    }
    local.0 = local.0.clamp(inset, (size.width - inset).max(inset));
    local.1 = local.1.clamp(inset, (size.height - inset).max(inset));
    local
}

/// Physical crossing: canvas X is kept on a horizontal hop, canvas Y on a vertical hop.
/// Only a linked overlapping segment is an exit (R-LAYOUT-3, R-LAYOUT-5).
pub fn try_cross(
    canvas: &Canvas,
    from: &MonitorId,
    local_mm: (f64, f64),
    heading: Edge,
    eps: f64,
) -> Option<Crossing> {
    let mon = canvas.monitor(from)?;
    if !on_edge(mon.effective_mm(), local_mm, heading, eps) {
        return None;
    }
    let along = along_local(heading, local_mm);
    let link = canvas.links.iter().find(|l| {
        l.from == *from
            && l.from_edge == heading
            && along + eps >= l.from_along0
            && along - eps <= l.from_along1
    })?;
    let dest = canvas.monitor(&link.to)?;
    let canvas_mm = mon.to_canvas(local_mm);
    let dest_local = land(dest, canvas_mm, link.to_edge, eps.max(0.5));
    Some(Crossing {
        dest_monitor: dest.id.clone(),
        dest_local_mm: dest_local,
        dest_px: dest.local_mm_to_px(dest_local),
    })
}

/// Link every pair of monitor edges that face each other within `gap_mm`
/// and overlap in the other axis.
pub fn auto_link(canvas: &mut Canvas, gap_mm: f64) {
    canvas.links.clear();
    let gap = if gap_mm > 0.0 { gap_mm } else { DEFAULT_GAP_MM };
    let n = canvas.monitors.len();
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            maybe_link(canvas, i, j, gap);
        }
    }
}

fn maybe_link(canvas: &mut Canvas, i: usize, j: usize, gap: f64) {
    let a = &canvas.monitors[i];
    let b = &canvas.monitors[j];
    let ar = a.rect();
    let br = b.rect();

    // A.Right faces B.Left
    if (br.x - ar.right()).abs() <= gap {
        if let Some((y0, y1)) = RectMm::overlap_1d(ar.y, ar.bottom(), br.y, br.bottom()) {
            push_pair(
                canvas,
                i,
                Edge::Right,
                j,
                Edge::Left,
                y0,
                y1,
            );
        }
    }
    // A.Bottom faces B.Top
    if (br.y - ar.bottom()).abs() <= gap {
        if let Some((x0, x1)) = RectMm::overlap_1d(ar.x, ar.right(), br.x, br.right()) {
            push_pair(canvas, i, Edge::Bottom, j, Edge::Top, x0, x1);
        }
    }
}

fn push_pair(
    canvas: &mut Canvas,
    i: usize,
    i_edge: Edge,
    j: usize,
    j_edge: Edge,
    canvas0: f64,
    canvas1: f64,
) {
    let (from_a0, from_a1, from_b0, from_b1) = {
        let a = &canvas.monitors[i];
        let b = &canvas.monitors[j];
        if i_edge.is_vertical() {
            (
                canvas0 - a.origin_mm.1,
                canvas1 - a.origin_mm.1,
                canvas0 - b.origin_mm.1,
                canvas1 - b.origin_mm.1,
            )
        } else {
            (
                canvas0 - a.origin_mm.0,
                canvas1 - a.origin_mm.0,
                canvas0 - b.origin_mm.0,
                canvas1 - b.origin_mm.0,
            )
        }
    };
    let a_id = canvas.monitors[i].id.clone();
    let b_id = canvas.monitors[j].id.clone();
    canvas.links.push(EdgeLink {
        from: a_id.clone(),
        from_edge: i_edge,
        from_along0: from_a0.min(from_a1),
        from_along1: from_a0.max(from_a1),
        to: b_id.clone(),
        to_edge: j_edge,
    });
    canvas.links.push(EdgeLink {
        from: b_id,
        from_edge: j_edge,
        from_along0: from_b0.min(from_b1),
        from_along1: from_b0.max(from_b1),
        to: a_id,
        to_edge: i_edge,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeviceId, MonitorId, SizeMm, SizePx};

    fn mon(device: &str, idx: u32, x: f64, y: f64, w: f64, h: f64, px_w: u32, px_h: u32) -> Monitor {
        Monitor {
            id: MonitorId {
                device: DeviceId(device.into()),
                local_index: idx,
            },
            origin_mm: (x, y),
            physical_mm: SizeMm {
                width: w,
                height: h,
            },
            pixels: SizePx {
                width: px_w,
                height: px_h,
            },
            scale: 1.0,
            override_physical_mm: None,
        }
    }

    fn two_side_by_side() -> Canvas {
        let mut c = Canvas::new();
        // 24" 16:9-ish: 530 × 300 mm, 1920×1080
        c.monitors.push(mon("win1", 0, 0.0, 0.0, 530.0, 300.0, 1920, 1080));
        c.monitors.push(mon("win1", 1, 530.0, 40.0, 530.0, 300.0, 1920, 1080));
        auto_link(&mut c, 2.0);
        c
    }

    #[test]
    fn px_mm_roundtrip() {
        let m = mon("win1", 0, 0.0, 0.0, 530.0, 300.0, 1920, 1080);
        let mm = m.px_to_local_mm((960.0, 540.0));
        let back = m.local_mm_to_px(mm);
        assert!((back.0 - 960.0).abs() < 0.01);
        assert!((back.1 - 540.0).abs() < 0.01);
    }

    #[test]
    fn crossing_keeps_canvas_y() {
        let c = two_side_by_side();
        let from = c.monitors[0].id.clone();
        // 100 mm from the top of the left monitor → canvas y = 100
        let hit = try_cross(&c, &from, (529.0, 100.0), Edge::Right, EDGE_EPS_MM).unwrap();
        assert_eq!(hit.dest_monitor, c.monitors[1].id);
        let dest = c.monitor(&hit.dest_monitor).unwrap();
        let canvas = dest.to_canvas(hit.dest_local_mm);
        assert!((canvas.1 - 100.0).abs() < 1.0, "canvas y {}", canvas.1);
        // dest origin y is 40, so local y ≈ 60
        assert!((hit.dest_local_mm.1 - 60.0).abs() < 1.0);
    }

    #[test]
    fn unmatched_edge_is_a_wall() {
        let c = two_side_by_side();
        let from = c.monitors[0].id.clone();
        // Left monitor's left edge has no neighbour.
        assert!(try_cross(&c, &from, (0.5, 100.0), Edge::Left, EDGE_EPS_MM).is_none());
        // Top of the overlap-less band on the right monitor (y local 10 → canvas 50,
        // which is still on the left monitor's height, so this *is* linked).
        // A point on the left monitor below the right monitor's bottom (canvas y > 340)
        // is outside both; use the left monitor's top-right above the neighbour (y=10
        // is still overlapping). Neighbour starts at canvas y=40, so local y=10 on the
        // left is canvas 10: no overlap.
        assert!(try_cross(&c, &from, (529.0, 10.0), Edge::Right, EDGE_EPS_MM).is_none());
    }

    #[test]
    fn dpi_does_not_change_physical_landing() {
        let mut c = Canvas::new();
        c.monitors.push(mon("win1", 0, 0.0, 0.0, 530.0, 300.0, 1920, 1080));
        let mut retina = mon("mac", 0, 530.0, 0.0, 530.0, 300.0, 1512, 982);
        retina.scale = 2.0;
        c.monitors.push(retina);
        auto_link(&mut c, 2.0);
        let from = c.monitors[0].id.clone();
        let hit = try_cross(&c, &from, (529.0, 150.0), Edge::Right, EDGE_EPS_MM).unwrap();
        assert!((hit.dest_local_mm.1 - 150.0).abs() < 1.0);
    }
}
