//! Beside placement, controller side: capture this computer's input at a linked
//! edge, send it to a peer, and take it back when the cursor leaves (R-PLACE-1,
//! R-INPUT-1, R-INPUT-2, R-INPUT-5, R-LAYOUT-3).
//!
//! This is pure logic. The Windows glue turns hook events into [`Input`] and
//! carries out the returned [`Out`] values: it moves the local cursor, and
//! sends mouse and key events on the peer's session.
//!
//! While captured the real cursor is held at the centre of the monitor it left
//! from. Every mouse move reports where the cursor would have gone, so the
//! distance from that centre is the motion delta. A virtual cursor
//! ([`Capture`]) follows the deltas on the target's display, and absolute
//! target pixels are sent, so the target never has to guess about clamping.

use std::collections::BTreeSet;

use q2_layout::{
    auto_link, canvas_from_os_panels, try_cross, Canvas, DeviceId, Edge, Monitor, MonitorId,
    OsPanel, SizeMm, SizePx, DEFAULT_GAP_MM, EDGE_EPS_MM,
};

use crate::capture::{Capture, Motion};

const LOCAL: &str = "local";
/// Until the peer reports a physical size, its display is assumed to be 96 dpi.
const MM_PER_PX: f64 = 25.4 / 96.0;

const VK_ESCAPE: u32 = 0x1B;
const CTRL: [u32; 3] = [0x11, 0xA2, 0xA3];
const ALT: [u32; 3] = [0x12, 0xA4, 0xA5];
const SHIFT: [u32; 3] = [0x10, 0xA0, 0xA1];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Button {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// A physical input event on this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// Where the cursor is about to be, in screen pixels.
    Move { x: i32, y: i32 },
    Button { button: Button, down: bool },
    /// 120 per notch, positive is up or right.
    Wheel { vertical: bool, delta: i32 },
    Key {
        vk: u32,
        scan: u32,
        extended: bool,
        down: bool,
    },
}

/// Something for the glue to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Out {
    /// Put the local cursor here (screen pixels).
    Warp { x: i32, y: i32 },
    /// Control moves to `peer`; its cursor appears at this target pixel.
    Enter { peer: String, x: i32, y: i32 },
    /// Absolute target pixel.
    MoveTo { x: i32, y: i32 },
    Button { button: Button, down: bool },
    Wheel { vertical: bool, delta: i32 },
    Key {
        vk: u32,
        scan: u32,
        extended: bool,
        down: bool,
    },
    /// Control is back on this computer.
    Leave { peer: String },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
    /// Hide the event from the rest of this computer.
    pub swallow: bool,
    pub out: Vec<Out>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub peer: String,
    pub size_px: (i32, i32),
}

struct Captured {
    peer: String,
    monitor: MonitorId,
    capture: Capture,
    center: (i32, i32),
    keys: BTreeSet<(u32, u32, bool)>,
    buttons: BTreeSet<Button>,
}

enum State {
    Local,
    Captured(Captured),
}

pub struct Beside {
    panels: Vec<OsPanel>,
    target: Option<Target>,
    ready: bool,
    canvas: Canvas,
    state: State,
}

impl Default for Beside {
    fn default() -> Self {
        Self::new()
    }
}

impl Beside {
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            target: None,
            ready: false,
            canvas: Canvas::new(),
            state: State::Local,
        }
    }

    pub fn is_captured(&self) -> bool {
        matches!(self.state, State::Captured(_))
    }

    /// This computer's monitors, as the OS arranges them.
    pub fn set_local_panels(&mut self, panels: Vec<OsPanel>) {
        self.panels = panels;
        self.rebuild();
    }

    /// The peer that sits to the right of the rightmost local monitor.
    pub fn set_target(&mut self, target: Option<Target>) {
        self.target = target;
        self.rebuild();
    }

    /// Only a peer whose session is logged in can capture input.
    pub fn set_ready(&mut self, ready: bool) -> Vec<Out> {
        self.ready = ready;
        if ready {
            return Vec::new();
        }
        self.release()
    }

    /// End a capture without a cursor exit (disconnect, rescue, placement change).
    pub fn release(&mut self) -> Vec<Out> {
        match std::mem::replace(&mut self.state, State::Local) {
            State::Local => Vec::new(),
            State::Captured(c) => {
                let warp = Out::Warp {
                    x: c.center.0,
                    y: c.center.1,
                };
                let mut out = leave_outs(c);
                out.push(warp);
                out
            }
        }
    }

    pub fn handle(&mut self, input: Input) -> Step {
        match std::mem::replace(&mut self.state, State::Local) {
            State::Local => self.local_input(input),
            State::Captured(c) => self.captured_input(c, input),
        }
    }

    fn rebuild(&mut self) {
        let mut canvas = canvas_from_os_panels(LOCAL, &self.panels);
        let rightmost = canvas
            .monitors
            .iter()
            .max_by(|a, b| a.rect().right().total_cmp(&b.rect().right()))
            .map(|m| m.rect());
        if let (Some(target), Some(rect)) = (&self.target, rightmost) {
            let (w, h) = (target.size_px.0.max(1), target.size_px.1.max(1));
            canvas.monitors.push(Monitor {
                id: target_id(&target.peer),
                origin_mm: (rect.right(), rect.y),
                physical_mm: SizeMm {
                    width: w as f64 * MM_PER_PX,
                    height: h as f64 * MM_PER_PX,
                },
                pixels: SizePx {
                    width: w as u32,
                    height: h as u32,
                },
                scale: 1.0,
                override_physical_mm: None,
            });
            auto_link(&mut canvas, DEFAULT_GAP_MM);
        }
        self.canvas = canvas;
    }

    fn local_input(&mut self, input: Input) -> Step {
        let Input::Move { x, y } = input else {
            return Step::default();
        };
        if !self.ready {
            return Step::default();
        }
        let Some(target) = self.target.clone() else {
            return Step::default();
        };
        let Some(panel) = self
            .panels
            .iter()
            .find(|p| x >= p.x && x < p.x + p.width as i32 && y >= p.y && y < p.y + p.height as i32)
        else {
            return Step::default();
        };
        let Some(mon) = self
            .canvas
            .monitors
            .iter()
            .find(|m| m.id.device.0 == LOCAL && m.id.local_index == panel.index)
        else {
            return Step::default();
        };
        let size = mon.effective_mm();
        let at = mon.px_to_local_mm(((x - panel.x) as f64, (y - panel.y) as f64));
        let candidates = [
            (x >= panel.x + panel.width as i32 - 1, Edge::Right, (size.width, at.1)),
            (x <= panel.x, Edge::Left, (0.0, at.1)),
            (y <= panel.y, Edge::Top, (at.0, 0.0)),
            (y >= panel.y + panel.height as i32 - 1, Edge::Bottom, (at.0, size.height)),
        ];
        for (touching, heading, on_edge) in candidates {
            if !touching {
                continue;
            }
            let Some(hit) = try_cross(&self.canvas, &mon.id, on_edge, heading, EDGE_EPS_MM) else {
                continue;
            };
            if hit.dest_monitor != target_id(&target.peer) {
                continue;
            }
            let entry = (hit.dest_px.0.round() as i32, hit.dest_px.1.round() as i32);
            let capture = Capture::enter(target.size_px, entry, heading.opposite());
            let (ex, ey) = capture.position();
            let center = (
                panel.x + panel.width as i32 / 2,
                panel.y + panel.height as i32 / 2,
            );
            self.state = State::Captured(Captured {
                peer: target.peer.clone(),
                monitor: hit.dest_monitor,
                capture,
                center,
                keys: BTreeSet::new(),
                buttons: BTreeSet::new(),
            });
            return Step {
                swallow: true,
                out: vec![
                    Out::Warp {
                        x: center.0,
                        y: center.1,
                    },
                    Out::Enter {
                        peer: target.peer,
                        x: ex,
                        y: ey,
                    },
                ],
            };
        }
        Step::default()
    }

    fn captured_input(&mut self, mut c: Captured, input: Input) -> Step {
        let mut out = Vec::new();
        match input {
            Input::Move { x, y } => {
                let (dx, dy) = (x - c.center.0, y - c.center.1);
                if dx == 0 && dy == 0 {
                    self.state = State::Captured(c);
                    return Step {
                        swallow: true,
                        out,
                    };
                }
                match c.capture.motion(dx, dy) {
                    Motion::Inside { x, y } => out.push(Out::MoveTo { x, y }),
                    Motion::Exit { along } => {
                        let landing = self.landing(&c, along);
                        let mut out = leave_outs(c);
                        out.push(Out::Warp {
                            x: landing.0,
                            y: landing.1,
                        });
                        return Step { swallow: true, out };
                    }
                }
            }
            Input::Button { button, down } => {
                if down {
                    c.buttons.insert(button);
                } else {
                    c.buttons.remove(&button);
                }
                out.push(Out::Button { button, down });
            }
            Input::Wheel { vertical, delta } => out.push(Out::Wheel { vertical, delta }),
            Input::Key {
                vk,
                scan,
                extended,
                down,
            } => {
                if down && vk == VK_ESCAPE && rescue_held(&c.keys) {
                    let warp = Out::Warp {
                        x: c.center.0,
                        y: c.center.1,
                    };
                    let mut out = leave_outs(c);
                    out.push(warp);
                    return Step { swallow: true, out };
                }
                if down {
                    c.keys.insert((vk, scan, extended));
                } else {
                    c.keys.remove(&(vk, scan, extended));
                }
                out.push(Out::Key {
                    vk,
                    scan,
                    extended,
                    down,
                });
            }
        }
        self.state = State::Captured(c);
        Step { swallow: true, out }
    }

    /// Where on this computer's screen the cursor comes back, in pixels.
    fn landing(&self, c: &Captured, along_px: i32) -> (i32, i32) {
        let fallback = c.center;
        let Some(mon) = self.canvas.monitor(&c.monitor) else {
            return fallback;
        };
        let exit = c.capture.exit_edge();
        let size = mon.effective_mm();
        let local = mon.px_to_local_mm(match exit {
            Edge::Left | Edge::Right => (0.0, along_px as f64),
            Edge::Top | Edge::Bottom => (along_px as f64, 0.0),
        });
        let on_edge = match exit {
            Edge::Left => (0.0, local.1),
            Edge::Right => (size.width, local.1),
            Edge::Top => (local.0, 0.0),
            Edge::Bottom => (local.0, size.height),
        };
        let Some(hit) = try_cross(&self.canvas, &c.monitor, on_edge, exit, EDGE_EPS_MM) else {
            return fallback;
        };
        let Some(panel) = self
            .panels
            .iter()
            .find(|p| p.index == hit.dest_monitor.local_index)
        else {
            return fallback;
        };
        // The layout already lands the cursor inside the monitor, away from
        // the edge pixel that would capture it again.
        let x = panel.x + hit.dest_px.0.round() as i32;
        let y = panel.y + hit.dest_px.1.round() as i32;
        (
            x.clamp(panel.x, panel.x + panel.width as i32 - 1),
            y.clamp(panel.y, panel.y + panel.height as i32 - 1),
        )
    }
}

fn target_id(peer: &str) -> MonitorId {
    MonitorId {
        device: DeviceId(peer.to_owned()),
        local_index: 0,
    }
}

fn rescue_held(keys: &BTreeSet<(u32, u32, bool)>) -> bool {
    let held = |group: &[u32; 3]| keys.iter().any(|(vk, _, _)| group.contains(vk));
    held(&CTRL) && held(&ALT) && held(&SHIFT)
}

/// Release everything that is held on the target, then say control is back.
fn leave_outs(c: Captured) -> Vec<Out> {
    let mut out = Vec::new();
    for (vk, scan, extended) in c.keys {
        out.push(Out::Key {
            vk,
            scan,
            extended,
            down: false,
        });
    }
    for button in c.buttons {
        out.push(Out::Button {
            button,
            down: false,
        });
    }
    out.push(Out::Leave { peer: c.peer });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const PEER: &str = "100.64.0.2";

    fn panel() -> OsPanel {
        OsPanel {
            index: 0,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            width_mm: 508.0,
            height_mm: 285.75,
            scale: 1.0,
        }
    }

    fn ready() -> Beside {
        let mut b = Beside::new();
        b.set_local_panels(vec![panel()]);
        b.set_target(Some(Target {
            peer: PEER.to_owned(),
            size_px: (1920, 1080),
        }));
        b.set_ready(true);
        b
    }

    fn captured() -> Beside {
        let mut b = ready();
        let step = b.handle(Input::Move { x: 1919, y: 540 });
        assert!(step.swallow);
        b
    }

    #[test]
    fn touching_the_linked_edge_captures_at_the_same_physical_height() {
        let mut b = ready();
        let step = b.handle(Input::Move { x: 1919, y: 540 });
        assert!(step.swallow);
        assert!(b.is_captured());
        assert_eq!(step.out[0], Out::Warp { x: 960, y: 540 });
        match &step.out[1] {
            Out::Enter { peer, x, y } => {
                assert_eq!(peer, PEER);
                assert!((0..=8).contains(x), "x was {x}");
                assert!((*y - 540).abs() <= 2, "y was {y}");
            }
            other => panic!("expected Enter, got {other:?}"),
        }
    }

    #[test]
    fn a_different_height_monitor_gets_the_physical_landing() {
        let mut b = Beside::new();
        b.set_local_panels(vec![panel()]);
        b.set_target(Some(Target {
            peer: PEER.to_owned(),
            size_px: (1280, 1440),
        }));
        b.set_ready(true);
        // 100 mm from the top of the local monitor is 378 px at 3.78 px/mm.
        let step = b.handle(Input::Move { x: 1919, y: 378 });
        match &step.out[1] {
            Out::Enter { y, .. } => assert!((*y - 378).abs() <= 3, "y was {y}"),
            other => panic!("expected Enter, got {other:?}"),
        }
    }

    #[test]
    fn nothing_happens_away_from_the_edge_a_wall_or_when_not_ready() {
        let mut b = ready();
        assert_eq!(b.handle(Input::Move { x: 900, y: 500 }), Step::default());
        assert_eq!(b.handle(Input::Move { x: 0, y: 500 }), Step::default());
        assert!(!b.is_captured());
        let mut b = ready();
        b.set_ready(false);
        assert_eq!(b.handle(Input::Move { x: 1919, y: 500 }), Step::default());
        let mut b = Beside::new();
        b.set_local_panels(vec![panel()]);
        b.set_ready(true);
        assert_eq!(b.handle(Input::Move { x: 1919, y: 500 }), Step::default());
    }

    #[test]
    fn local_buttons_and_keys_are_not_touched_when_not_captured() {
        let mut b = ready();
        let key = Input::Key {
            vk: 0x41,
            scan: 0x1E,
            extended: false,
            down: true,
        };
        assert_eq!(b.handle(key), Step::default());
    }

    #[test]
    fn motion_is_sent_as_an_absolute_target_pixel() {
        let mut b = ready();
        let enter = b.handle(Input::Move { x: 1919, y: 540 });
        let (ex, ey) = match &enter.out[1] {
            Out::Enter { x, y, .. } => (*x, *y),
            other => panic!("expected Enter, got {other:?}"),
        };
        let step = b.handle(Input::Move { x: 960 + 10, y: 540 + 5 });
        assert!(step.swallow);
        match step.out.as_slice() {
            [Out::MoveTo { x, y }] => {
                assert_eq!(*x, ex + 10);
                assert_eq!(*y, ey + 5);
            }
            other => panic!("unexpected {other:?}"),
        }
        // The warp back to the centre produces a move with no delta.
        let step = b.handle(Input::Move { x: 960, y: 540 });
        assert_eq!(
            step,
            Step {
                swallow: true,
                out: vec![]
            }
        );
    }

    #[test]
    fn pushing_back_through_the_edge_returns_at_the_same_physical_height() {
        let mut b = captured();
        b.handle(Input::Key {
            vk: 0x41,
            scan: 0x1E,
            extended: false,
            down: true,
        });
        b.handle(Input::Button {
            button: Button::Left,
            down: true,
        });
        let step = b.handle(Input::Move { x: 960 - 30, y: 540 });
        assert!(step.swallow);
        assert!(!b.is_captured());
        let n = step.out.len();
        assert_eq!(
            step.out[0],
            Out::Key {
                vk: 0x41,
                scan: 0x1E,
                extended: false,
                down: false
            }
        );
        assert_eq!(
            step.out[1],
            Out::Button {
                button: Button::Left,
                down: false
            }
        );
        assert_eq!(
            step.out[2],
            Out::Leave {
                peer: PEER.to_owned()
            }
        );
        match &step.out[n - 1] {
            Out::Warp { x, y } => {
                assert!((*x - 1914).abs() <= 3, "x was {x}");
                assert!((*y - 540).abs() <= 3, "y was {y}");
            }
            other => panic!("expected Warp, got {other:?}"),
        }
    }

    #[test]
    fn wheel_and_keys_are_forwarded_and_swallowed() {
        let mut b = captured();
        let step = b.handle(Input::Wheel {
            vertical: true,
            delta: -120,
        });
        assert!(step.swallow);
        assert_eq!(
            step.out,
            vec![Out::Wheel {
                vertical: true,
                delta: -120
            }]
        );
        let step = b.handle(Input::Key {
            vk: 0x41,
            scan: 0x1E,
            extended: false,
            down: true,
        });
        assert!(step.swallow);
        assert_eq!(step.out.len(), 1);
    }

    #[test]
    fn ctrl_alt_shift_escape_is_the_rescue_and_releases_what_is_held() {
        let mut b = captured();
        for vk in [0xA2u32, 0xA4, 0xA0] {
            b.handle(Input::Key {
                vk,
                scan: 0,
                extended: false,
                down: true,
            });
        }
        let step = b.handle(Input::Key {
            vk: VK_ESCAPE,
            scan: 1,
            extended: false,
            down: true,
        });
        assert!(step.swallow);
        assert!(!b.is_captured());
        let releases = step
            .out
            .iter()
            .filter(|o| matches!(o, Out::Key { down: false, .. }))
            .count();
        assert_eq!(releases, 3);
        assert!(step
            .out
            .iter()
            .any(|o| matches!(o, Out::Leave { peer } if peer == PEER)));
        assert!(matches!(step.out.last(), Some(Out::Warp { x: 960, y: 540 })));
    }

    #[test]
    fn losing_the_session_ends_the_capture() {
        let mut b = captured();
        let out = b.set_ready(false);
        assert!(!b.is_captured());
        assert!(out
            .iter()
            .any(|o| matches!(o, Out::Leave { peer } if peer == PEER)));
        assert!(b.release().is_empty());
    }
}
