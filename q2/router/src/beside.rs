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

use q2_keys::{remap_windows_key, OsFamily, PhysKey, FAKE_ALTGR_CTRL_SCAN};

use crate::capture::{Capture, Motion};
use crate::hotkey::{is_modifier, Hotkey, CTRL};
use crate::Guards;

const LOCAL: &str = "local";
/// Until the peer reports a physical size, its display is assumed to be 96 dpi.
const MM_PER_PX: f64 = 25.4 / 96.0;

/// Ctrl+Alt+Shift+Esc: the way out when control is stuck on a peer (R-INPUT-5).
const HOME_HOTKEY: Hotkey = Hotkey {
    ctrl: true,
    alt: true,
    shift: true,
    win: false,
    vk: 0x1B,
};

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

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub peer: String,
    pub size_px: (i32, i32),
    /// The peer's real physical size. `None` until it has told us, and then
    /// the display is assumed to be 96 dpi.
    pub size_mm: Option<(f64, f64)>,
    /// The peer's operating system, which decides how modifier keys are mapped.
    pub os: OsFamily,
}

struct Captured {
    peer: String,
    monitor: MonitorId,
    capture: Capture,
    center: (i32, i32),
    keys: BTreeSet<(u32, u32, bool)>,
    buttons: BTreeSet<Button>,
    /// Keys that were already down on this computer when the capture began.
    /// Their release is not hidden from this computer.
    pre_held: BTreeSet<u32>,
    /// The peer's operating system.
    os: OsFamily,
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
    guards: Guards,
    home: Hotkey,
    jump: Option<Hotkey>,
    fullscreen_app: Option<String>,
    /// Physical keys that are down on this computer, as seen while not captured.
    held_local: BTreeSet<(u32, u32, bool)>,
    held_buttons: BTreeSet<Button>,
    last_pos: (i32, i32),
    /// Keys of a shortcut that was acted on. Their repeats and release are hidden.
    swallow_up: BTreeSet<u32>,
    /// Physical input has arrived on this computer since `take_physical`.
    physical: bool,
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
            guards: Guards::default(),
            home: HOME_HOTKEY,
            jump: None,
            fullscreen_app: None,
            held_local: BTreeSet::new(),
            held_buttons: BTreeSet::new(),
            last_pos: (0, 0),
            swallow_up: BTreeSet::new(),
            physical: false,
        }
    }

    pub fn is_captured(&self) -> bool {
        matches!(self.state, State::Captured(_))
    }

    /// The peer this computer is sending its input to, if any.
    pub fn captured_peer(&self) -> Option<&str> {
        match &self.state {
            State::Captured(c) => Some(&c.peer),
            State::Local => None,
        }
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

    /// Switch guards (R-INPUT-4). `require_modifier` means Ctrl must be held.
    pub fn set_guards(&mut self, guards: Guards) {
        self.guards = guards;
    }

    /// `home` returns control to this computer; `jump` takes it to the peer,
    /// at the centre of the peer's display (R-INPUT-5).
    pub fn set_hotkeys(&mut self, home: Option<Hotkey>, jump: Option<Hotkey>) {
        self.home = home.unwrap_or(HOME_HOTKEY);
        self.jump = jump;
    }

    /// The executable name of the foreground app when it is fullscreen.
    pub fn set_fullscreen_app(&mut self, app: Option<String>) {
        self.fullscreen_app = app;
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

    /// True once if the person has used this computer's own mouse or keyboard
    /// while it was not sending its input elsewhere (R-DEV-2). The caller tells
    /// the other devices, so that whichever computer is driving this one lets go.
    pub fn take_physical(&mut self) -> bool {
        std::mem::take(&mut self.physical)
    }

    pub fn handle(&mut self, input: Input) -> Step {
        if matches!(self.state, State::Local) {
            self.physical = true;
        }
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
                physical_mm: match target.size_mm {
                    Some((width, height)) if width > 0.0 && height > 0.0 => {
                        SizeMm { width, height }
                    }
                    _ => SizeMm {
                        width: w as f64 * MM_PER_PX,
                        height: h as f64 * MM_PER_PX,
                    },
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
        match input {
            Input::Move { x, y } => {
                self.last_pos = (x, y);
                self.edge_input(x, y)
            }
            Input::Key {
                vk,
                scan,
                extended,
                down,
            } => self.local_key(vk, scan, extended, down),
            Input::Button { button, down } => {
                if down {
                    self.held_buttons.insert(button);
                } else {
                    self.held_buttons.remove(&button);
                }
                Step::default()
            }
            _ => Step::default(),
        }
    }

    fn held_vks(&self) -> BTreeSet<u32> {
        self.held_local.iter().map(|(vk, _, _)| *vk).collect()
    }

    fn local_key(&mut self, vk: u32, scan: u32, extended: bool, down: bool) -> Step {
        if scan == FAKE_ALTGR_CTRL_SCAN {
            return Step::default();
        }
        if !down {
            self.held_local.remove(&(vk, scan, extended));
            let hidden = self.swallow_up.remove(&vk);
            return Step {
                swallow: hidden,
                out: Vec::new(),
            };
        }
        self.held_local.insert((vk, scan, extended));
        if self.swallow_up.contains(&vk) {
            return Step {
                swallow: true,
                out: Vec::new(),
            };
        }
        let held = self.held_vks();
        let jump = self.jump.is_some_and(|h| h.matches(vk, &held));
        if jump && self.ready {
            if let Some(step) = self.jump_capture() {
                self.swallow_up.insert(vk);
                return step;
            }
        }
        Step::default()
    }

    /// Take control to the peer without crossing an edge.
    fn jump_capture(&mut self) -> Option<Step> {
        let target = self.target.clone()?;
        let (x, y) = self.last_pos;
        let panel = self
            .panels
            .iter()
            .find(|p| x >= p.x && x < p.x + p.width as i32 && y >= p.y && y < p.y + p.height as i32)
            .or_else(|| self.panels.first())?;
        let center = (
            panel.x + panel.width as i32 / 2,
            panel.y + panel.height as i32 / 2,
        );
        let entry = (target.size_px.0 / 2, target.size_px.1 / 2);
        // The peer sits to the right, so its way back is its left edge.
        let capture = Capture::enter(target.size_px, entry, Edge::Left);
        let (ex, ey) = capture.position();
        self.state = State::Captured(Captured {
            peer: target.peer.clone(),
            monitor: target_id(&target.peer),
            capture,
            center,
            keys: BTreeSet::new(),
            buttons: BTreeSet::new(),
            pre_held: self.held_vks(),
            os: target.os,
        });
        Some(Step {
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
        })
    }

    fn edge_blocked(&self, heading: Edge, on_edge: (f64, f64), size: SizeMm) -> bool {
        let g = &self.guards;
        let (along, length) = match heading {
            Edge::Left | Edge::Right => (on_edge.1, size.height),
            Edge::Top | Edge::Bottom => (on_edge.0, size.width),
        };
        if g.corner_dead_zone_mm > 0.0
            && (along < g.corner_dead_zone_mm || along > length - g.corner_dead_zone_mm)
        {
            return true;
        }
        if g.require_modifier && !self.held_vks().iter().any(|vk| CTRL.contains(vk)) {
            return true;
        }
        if g.block_fullscreen {
            if let Some(app) = &self.fullscreen_app {
                let app = app.to_lowercase();
                let ignored = g
                    .fullscreen_ignore
                    .iter()
                    .any(|name| name.trim().to_lowercase() == app);
                if !ignored {
                    return true;
                }
            }
        }
        false
    }

    fn edge_input(&mut self, x: i32, y: i32) -> Step {
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
            if self.edge_blocked(heading, on_edge, size) {
                return Step::default();
            }
            let entry = (hit.dest_px.0.round() as i32, hit.dest_px.1.round() as i32);
            let capture = Capture::enter(target.size_px, entry, heading.opposite());
            let (ex, ey) = capture.position();
            let center = (
                panel.x + panel.width as i32 / 2,
                panel.y + panel.height as i32 / 2,
            );
            let mut captured = Captured {
                peer: target.peer.clone(),
                monitor: hit.dest_monitor,
                capture,
                center,
                keys: BTreeSet::new(),
                buttons: BTreeSet::new(),
                pre_held: self.held_vks(),
                os: target.os,
            };
            let mut out = vec![
                Out::Warp {
                    x: center.0,
                    y: center.1,
                },
                Out::Enter {
                    peer: target.peer,
                    x: ex,
                    y: ey,
                },
            ];
            // Modifiers held on this computer keep working on the peer.
            for &(vk, scan, extended) in &self.held_local {
                if is_modifier(vk) {
                    captured.keys.insert((vk, scan, extended));
                    out.push(key_out(target.os, vk, scan, extended, true));
                }
            }
            // Mouse buttons held on this computer (e.g. during a drag) continue on the peer.
            for &button in &self.held_buttons {
                captured.buttons.insert(button);
                out.push(Out::Button { button, down: true });
            }
            self.state = State::Captured(captured);
            return Step { swallow: true, out };
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
                if scan == FAKE_ALTGR_CTRL_SCAN {
                    // Windows makes up a left Ctrl for AltGr. The peer gets AltGr alone.
                    self.state = State::Captured(c);
                    return Step {
                        swallow: true,
                        out,
                    };
                }
                let mut held: BTreeSet<u32> = c.keys.iter().map(|(vk, _, _)| *vk).collect();
                held.extend(c.pre_held.iter().copied());
                if down && self.home.matches(vk, &held) {
                    let warp = Out::Warp {
                        x: c.center.0,
                        y: c.center.1,
                    };
                    let mut out = leave_outs(c);
                    out.push(warp);
                    self.swallow_up.insert(vk);
                    return Step { swallow: true, out };
                }
                if self.swallow_up.contains(&vk) {
                    // A shortcut that was acted on: hide its repeats and release.
                    if !down {
                        self.swallow_up.remove(&vk);
                        c.pre_held.remove(&vk);
                        self.held_local.retain(|(held_vk, _, _)| *held_vk != vk);
                    }
                } else if !down && c.pre_held.remove(&vk) {
                    // Pressed before the capture: this computer needs the release.
                    self.held_local.retain(|(held_vk, _, _)| *held_vk != vk);
                    if c.keys.remove(&(vk, scan, extended)) {
                        out.push(key_out(c.os, vk, scan, extended, false));
                    }
                    self.state = State::Captured(c);
                    return Step { swallow: false, out };
                } else {
                    if down {
                        c.keys.insert((vk, scan, extended));
                    } else {
                        c.keys.remove(&(vk, scan, extended));
                    }
                    out.push(key_out(c.os, vk, scan, extended, down));
                }
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

/// The physical size of the peer's display that has `size_px` pixels, out of
/// the monitors the peer reported. Prefers the one at the origin, which is the
/// display the session shows. When no monitor has exactly those pixels (a
/// Retina display can count points where the session counts pixels), the one at
/// the origin with the same shape is used. `None` when nothing fits or a size
/// is missing.
pub fn physical_size_of(panels: &[OsPanel], size_px: (i32, i32)) -> Option<(f64, f64)> {
    let has_size = |p: &&OsPanel| p.width_mm > 0.0 && p.height_mm > 0.0;
    let same_size = |p: &&OsPanel| {
        has_size(p) && p.width as i32 == size_px.0 && p.height as i32 == size_px.1
    };
    let at_origin = |p: &&OsPanel| p.x == 0 && p.y == 0;
    let exact = panels
        .iter()
        .filter(same_size)
        .find(at_origin)
        .or_else(|| panels.iter().find(same_size));
    let found = match exact {
        Some(found) => found,
        None => {
            let shape = size_px.0 as f64 / size_px.1.max(1) as f64;
            panels.iter().filter(has_size).filter(at_origin).find(|p| {
                let own = p.width as f64 / (p.height.max(1)) as f64;
                (own - shape).abs() / shape < 0.005
            })?
        }
    };
    Some((found.width_mm, found.height_mm))
}

/// A key event for the peer, with the modifier mapping of R-KEYS-2 applied.
fn key_out(os: OsFamily, vk: u32, scan: u32, extended: bool, down: bool) -> Out {
    let key = remap_windows_key(os, PhysKey { vk, scan, extended });
    Out::Key {
        vk: key.vk,
        scan: key.scan,
        extended: key.extended,
        down,
    }
}

fn target_id(peer: &str) -> MonitorId {
    MonitorId {
        device: DeviceId(peer.to_owned()),
        local_index: 0,
    }
}

/// Release everything that is held on the target, then say control is back.
fn leave_outs(c: Captured) -> Vec<Out> {
    let mut out = Vec::new();
    for (vk, scan, extended) in c.keys {
        out.push(key_out(c.os, vk, scan, extended, false));
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
    use crate::Guards;

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
            size_mm: None,
            os: OsFamily::Windows,
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
    fn drag_with_button_held_forwards_button_down_and_releases_on_leave() {
        let mut b = ready();
        b.handle(Input::Button {
            button: Button::Left,
            down: true,
        });
        let step = b.handle(Input::Move { x: 1919, y: 540 });
        assert!(step.swallow);
        assert!(b.is_captured());
        assert!(step.out.contains(&Out::Button {
            button: Button::Left,
            down: true,
        }));
        let outs = b.release();
        assert!(outs.contains(&Out::Button {
            button: Button::Left,
            down: false,
        }));
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
            size_mm: None,
            os: OsFamily::Windows,
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
            vk: 0x1B,
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

    fn key(vk: u32, down: bool) -> Input {
        Input::Key {
            vk,
            scan: vk,
            extended: false,
            down,
        }
    }

    fn guarded(guards: Guards) -> Beside {
        let mut b = ready();
        b.set_guards(guards);
        b
    }

    #[test]
    fn corner_dead_zone_blocks_near_the_corner_only() {
        let guards = Guards {
            corner_dead_zone_mm: 20.0,
            ..Guards::default()
        };
        // 1080 px is 285.75 mm: 20 mm is about 76 px at 3.78 px/mm.
        let mut b = guarded(guards.clone());
        assert_eq!(b.handle(Input::Move { x: 1919, y: 30 }), Step::default());
        assert_eq!(b.handle(Input::Move { x: 1919, y: 1060 }), Step::default());
        assert!(!b.is_captured());
        let mut b = guarded(guards);
        assert!(b.handle(Input::Move { x: 1919, y: 540 }).swallow);
    }

    #[test]
    fn require_modifier_needs_ctrl_held_and_hands_it_to_the_peer() {
        let guards = Guards {
            require_modifier: true,
            ..Guards::default()
        };
        let mut b = guarded(guards);
        assert_eq!(b.handle(Input::Move { x: 1919, y: 540 }), Step::default());
        assert_eq!(b.handle(key(0xA2, true)), Step::default());
        let step = b.handle(Input::Move { x: 1919, y: 540 });
        assert!(step.swallow);
        assert!(step.out.contains(&Out::Key {
            vk: 0xA2,
            scan: 0xA2,
            extended: false,
            down: true
        }));
        // Releasing Ctrl later reaches this computer and releases it on the peer.
        let up = b.handle(key(0xA2, false));
        assert!(!up.swallow);
        assert_eq!(
            up.out,
            vec![Out::Key {
                vk: 0xA2,
                scan: 0xA2,
                extended: false,
                down: false
            }]
        );
    }

    #[test]
    fn fullscreen_guard_blocks_unless_the_app_is_ignored() {
        let guards = Guards {
            block_fullscreen: true,
            fullscreen_ignore: ["Vlc.exe".to_owned()].into_iter().collect(),
            ..Guards::default()
        };
        let mut b = guarded(guards.clone());
        b.set_fullscreen_app(Some("game.exe".to_owned()));
        assert_eq!(b.handle(Input::Move { x: 1919, y: 540 }), Step::default());
        b.set_fullscreen_app(Some("VLC.EXE".to_owned()));
        assert!(b.handle(Input::Move { x: 1919, y: 540 }).swallow);
        let mut b = guarded(guards);
        b.set_fullscreen_app(None);
        assert!(b.handle(Input::Move { x: 1919, y: 540 }).swallow);
    }

    #[test]
    fn jump_hotkey_takes_control_to_the_peer_centre_and_hides_the_key() {
        let mut b = ready();
        b.set_hotkeys(None, Hotkey::parse("ctrl+alt+f1"));
        b.handle(Input::Move { x: 300, y: 300 });
        b.handle(key(0xA2, true));
        b.handle(key(0xA4, true));
        let step = b.handle(key(0x70, true));
        assert!(step.swallow);
        assert!(b.is_captured());
        assert_eq!(step.out[0], Out::Warp { x: 960, y: 540 });
        assert_eq!(
            step.out[1],
            Out::Enter {
                peer: PEER.to_owned(),
                x: 960,
                y: 540
            }
        );
        // Key repeat and release of the shortcut are hidden and not forwarded.
        let repeat = b.handle(key(0x70, true));
        assert_eq!(
            repeat,
            Step {
                swallow: true,
                out: vec![]
            }
        );
        let up = b.handle(key(0x70, false));
        assert_eq!(
            up,
            Step {
                swallow: true,
                out: vec![]
            }
        );
        // The modifiers were held before: their release is left to this computer.
        let ctrl_up = b.handle(key(0xA2, false));
        assert_eq!(
            ctrl_up,
            Step {
                swallow: false,
                out: vec![]
            }
        );
    }

    #[test]
    fn jump_hotkey_needs_a_ready_peer_and_the_exact_modifiers() {
        let mut b = ready();
        b.set_hotkeys(None, Hotkey::parse("ctrl+alt+f1"));
        b.handle(key(0xA2, true));
        assert_eq!(b.handle(key(0x70, true)), Step::default());
        assert!(!b.is_captured());
        let mut b = ready();
        b.set_hotkeys(None, Hotkey::parse("ctrl+alt+f1"));
        b.set_ready(false);
        b.handle(key(0xA2, true));
        b.handle(key(0xA4, true));
        assert_eq!(b.handle(key(0x70, true)), Step::default());
    }

    #[test]
    fn a_configured_home_hotkey_replaces_the_default() {
        let mut b = ready();
        b.set_hotkeys(Hotkey::parse("ctrl+f12"), None);
        b.handle(Input::Move { x: 1919, y: 540 });
        for vk in [0xA2u32, 0xA4, 0xA0] {
            b.handle(key(vk, true));
        }
        assert!(b
            .handle(key(0x1B, true))
            .out
            .iter()
            .any(|o| matches!(o, Out::Key { vk: 0x1B, .. })));
        assert!(b.is_captured());
        let mut b = ready();
        b.set_hotkeys(Hotkey::parse("ctrl+f12"), None);
        b.handle(Input::Move { x: 1919, y: 540 });
        b.handle(key(0xA2, true));
        let step = b.handle(key(0x7B, true));
        assert!(!b.is_captured());
        assert!(step.out.iter().any(|o| matches!(o, Out::Leave { .. })));
    }

    #[test]
    fn keys_held_before_capture_are_released_locally_not_swallowed() {
        let mut b = ready();
        b.handle(key(0x41, true));
        let step = b.handle(Input::Move { x: 1919, y: 540 });
        // A plain letter is not handed to the peer.
        assert!(!step.out.iter().any(|o| matches!(o, Out::Key { .. })));
        let up = b.handle(key(0x41, false));
        assert_eq!(
            up,
            Step {
                swallow: false,
                out: vec![]
            }
        );
    }

    #[test]
    fn a_reported_physical_size_decides_the_landing_height() {
        // A 27 inch 1440p monitor (597 x 336 mm) next to the 285.75 mm tall one.
        let mut b = Beside::new();
        b.set_local_panels(vec![panel()]);
        b.set_target(Some(Target {
            peer: PEER.to_owned(),
            size_px: (2560, 1440),
            size_mm: Some((597.0, 336.0)),
            os: OsFamily::Windows,
        }));
        b.set_ready(true);
        // 100 mm below the top of the local monitor is 100 mm below the top of
        // the peer: 100 / 336 * 1440 = 428.6 px.
        let step = b.handle(Input::Move { x: 1919, y: 378 });
        match &step.out[1] {
            Out::Enter { y, .. } => assert!((*y - 429).abs() <= 3, "y was {y}"),
            other => panic!("expected Enter, got {other:?}"),
        }
    }

    #[test]
    fn the_reported_monitor_with_the_matching_pixels_gives_the_size() {
        let a = OsPanel {
            index: 0,
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            width_mm: 597.0,
            height_mm: 336.0,
            scale: 1.0,
        };
        let b = OsPanel {
            index: 1,
            x: 2560,
            width_mm: 521.0,
            height_mm: 293.0,
            width: 1920,
            height: 1080,
            ..a.clone()
        };
        let c = OsPanel {
            index: 2,
            x: 0,
            width_mm: 400.0,
            height_mm: 225.0,
            width: 1920,
            height: 1080,
            ..a.clone()
        };
        let all = [a.clone(), b.clone()];
        assert_eq!(physical_size_of(&all, (2560, 1440)), Some((597.0, 336.0)));
        assert_eq!(physical_size_of(&all, (1920, 1080)), Some((521.0, 293.0)));
        assert_eq!(physical_size_of(&all, (800, 600)), None);
        // A Retina display counts points (1440 x 900) where the session counts
        // pixels (2880 x 1800): same shape at the origin still matches.
        let retina = OsPanel {
            width: 1440,
            height: 900,
            width_mm: 286.0,
            height_mm: 179.0,
            ..a.clone()
        };
        assert_eq!(
            physical_size_of(&[retina.clone()], (2880, 1800)),
            Some((286.0, 179.0))
        );
        // A different shape does not.
        assert_eq!(physical_size_of(&[retina], (2560, 1440)), None);
        // Two with the same pixels: the one at the origin wins.
        let both = [b.clone(), c.clone()];
        assert_eq!(physical_size_of(&both, (1920, 1080)), Some((400.0, 225.0)));
        // A monitor that reports no size is not used.
        let blind = OsPanel {
            width_mm: 0.0,
            ..a
        };
        assert_eq!(physical_size_of(&[blind], (2560, 1440)), None);
    }

    fn mac_peer() -> Beside {
        let mut b = Beside::new();
        b.set_local_panels(vec![panel()]);
        b.set_target(Some(Target {
            peer: PEER.to_owned(),
            size_px: (1920, 1080),
            size_mm: None,
            os: OsFamily::Macos,
        }));
        b.set_ready(true);
        b.handle(Input::Move { x: 1919, y: 540 });
        assert!(b.is_captured());
        b
    }

    fn phys(vk: u32, scan: u32, extended: bool, down: bool) -> Input {
        Input::Key {
            vk,
            scan,
            extended,
            down,
        }
    }

    #[test]
    fn a_mac_peer_gets_cmd_for_ctrl_and_control_for_win_and_release_matches() {
        let mut b = mac_peer();
        let step = b.handle(phys(0xA2, 0x1D, false, true));
        assert_eq!(
            step.out,
            vec![Out::Key {
                vk: 0x5B,
                scan: 0x5B,
                extended: true,
                down: true
            }]
        );
        let step = b.handle(phys(0x5B, 0x5B, true, true));
        assert_eq!(
            step.out,
            vec![Out::Key {
                vk: 0xA2,
                scan: 0x1D,
                extended: false,
                down: true
            }]
        );
        // Alt (Option) is passed on unchanged.
        let step = b.handle(phys(0xA4, 0x38, false, true));
        assert_eq!(
            step.out,
            vec![Out::Key {
                vk: 0xA4,
                scan: 0x38,
                extended: false,
                down: true
            }]
        );
        // Leaving releases the mapped keys, not the physical ones.
        let leave = b.release();
        assert!(leave.contains(&Out::Key {
            vk: 0x5B,
            scan: 0x5B,
            extended: true,
            down: false
        }));
        assert!(leave.contains(&Out::Key {
            vk: 0xA2,
            scan: 0x1D,
            extended: false,
            down: false
        }));
        assert!(!leave.contains(&Out::Key {
            vk: 0xA2,
            scan: 0x1D,
            extended: true,
            down: false
        }));
    }

    #[test]
    fn a_windows_peer_gets_the_keys_unchanged() {
        let mut b = captured();
        let step = b.handle(phys(0xA2, 0x1D, false, true));
        assert_eq!(
            step.out,
            vec![Out::Key {
                vk: 0xA2,
                scan: 0x1D,
                extended: false,
                down: true
            }]
        );
    }

    #[test]
    fn the_ctrl_windows_makes_up_for_altgr_is_never_sent() {
        let mut b = mac_peer();
        let fake = b.handle(phys(0xA2, 0x21D, false, true));
        assert_eq!(
            fake,
            Step {
                swallow: true,
                out: vec![]
            }
        );
        // The right Alt itself (AltGr) reaches the peer as the right Option.
        let altgr = b.handle(phys(0xA5, 0x38, true, true));
        assert_eq!(
            altgr.out,
            vec![Out::Key {
                vk: 0xA5,
                scan: 0x38,
                extended: true,
                down: true
            }]
        );
        // Releasing the made-up Ctrl is hidden too, and nothing is left held for it.
        let fake_up = b.handle(phys(0xA2, 0x21D, false, false));
        assert_eq!(fake_up.out, vec![]);
        let leave = b.release();
        assert_eq!(
            leave
                .iter()
                .filter(|o| matches!(o, Out::Key { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn the_made_up_ctrl_does_not_count_as_ctrl_for_the_ctrl_guard() {
        let guards = Guards {
            require_modifier: true,
            ..Guards::default()
        };
        let mut b = guarded(guards);
        b.handle(phys(0xA2, 0x21D, false, true));
        assert_eq!(b.handle(Input::Move { x: 1919, y: 540 }), Step::default());
    }

    #[test]
    fn using_this_computer_while_it_is_not_sending_input_is_reported_once() {
        let mut b = ready();
        assert!(!b.take_physical());
        b.handle(Input::Move { x: 300, y: 300 });
        b.handle(key(0x41, true));
        assert!(b.take_physical());
        assert!(!b.take_physical());
        b.handle(Input::Button {
            button: Button::Left,
            down: true,
        });
        assert!(b.take_physical());
    }

    #[test]
    fn input_that_is_being_sent_to_a_peer_does_not_count() {
        let mut b = captured();
        b.take_physical();
        b.handle(Input::Move { x: 970, y: 545 });
        b.handle(key(0x41, true));
        assert!(!b.take_physical());
    }

    #[test]
    fn the_captured_peer_is_known_only_while_capturing() {
        let mut b = ready();
        assert_eq!(b.captured_peer(), None);
        b.handle(Input::Move { x: 1919, y: 540 });
        assert_eq!(b.captured_peer(), Some(PEER));
        b.release();
        assert_eq!(b.captured_peer(), None);
    }
}
