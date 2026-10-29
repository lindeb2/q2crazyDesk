//! Who is the controller, where the cursor is, and when it may cross (R-DEV-2, R-INPUT).
//!
//! Jump hotkeys, corner dead zones, and the fullscreen guard follow the behaviour
//! of PowerToys Mouse Without Borders (MIT),
//! https://github.com/microsoft/PowerToys, studied 2026-10-24.

mod beside;
mod capture;
mod hotkey;

use std::collections::{BTreeMap, BTreeSet};

pub use beside::{physical_size_of, Beside, Button, Input, Out, Step, Target};
pub use capture::{Capture, ExitRule, Motion};
pub use hotkey::Hotkey;
pub use q2_keys::{remap_macos_key, remap_windows_key, OsFamily};

use q2_keys::Key;
use q2_layout::{try_cross, Canvas, Crossing, DeviceId, Edge, MonitorId, EDGE_EPS_MM};
use serde::{Deserialize, Serialize};

pub type Tick = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorAt {
    pub monitor: MonitorId,
    pub local_mm: (f64, f64),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guards {
    pub corner_dead_zone_mm: f64,
    pub require_modifier: bool,
    pub block_fullscreen: bool,
    pub fullscreen_ignore: BTreeSet<String>,
}

impl Default for Guards {
    fn default() -> Self {
        Self {
            corner_dead_zone_mm: 0.0,
            require_modifier: false,
            block_fullscreen: false,
            fullscreen_ignore: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    PhysicalInput {
        device: DeviceId,
        tick: Tick,
    },
    EdgeApproach {
        monitor: MonitorId,
        local_mm: (f64, f64),
        heading: Edge,
        modifier_held: bool,
        fullscreen_app: Option<String>,
    },
    Jump(DeviceId),
    Disconnect(DeviceId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    BecomeController(DeviceId),
    Cross(Crossing),
    JumpTo {
        device: DeviceId,
        monitor: MonitorId,
        local_mm: (f64, f64),
    },
    ReleaseKeys(Vec<Key>),
}

pub struct Router {
    pub controller: DeviceId,
    pub cursor: CursorAt,
    pub held_keys: BTreeSet<Key>,
    pub guards: Guards,
    pub jump_hotkeys: BTreeMap<String, DeviceId>,
    last_input_tick: Tick,
}

impl Router {
    pub fn new(controller: DeviceId, cursor: CursorAt) -> Self {
        Self {
            controller,
            cursor,
            held_keys: BTreeSet::new(),
            guards: Guards::default(),
            jump_hotkeys: BTreeMap::new(),
            last_input_tick: 0,
        }
    }

    pub fn hold(&mut self, key: Key) {
        self.held_keys.insert(key);
    }

    pub fn handle(&mut self, canvas: &Canvas, event: Event) -> Vec<Action> {
        match event {
            Event::PhysicalInput { device, tick } => self.on_physical(device, tick),
            Event::EdgeApproach {
                monitor,
                local_mm,
                heading,
                modifier_held,
                fullscreen_app,
            } => self.on_edge(
                canvas,
                monitor,
                local_mm,
                heading,
                modifier_held,
                fullscreen_app,
            ),
            Event::Jump(device) => self.on_jump(canvas, device),
            Event::Disconnect(device) => self.on_disconnect(device),
        }
    }

    fn release_all(&mut self) -> Option<Action> {
        if self.held_keys.is_empty() {
            return None;
        }
        let keys: Vec<Key> = self.held_keys.iter().copied().collect();
        self.held_keys.clear();
        Some(Action::ReleaseKeys(keys))
    }

    fn on_physical(&mut self, device: DeviceId, tick: Tick) -> Vec<Action> {
        self.last_input_tick = tick;
        if device == self.controller {
            return Vec::new();
        }
        let mut out = Vec::new();
        if let Some(rel) = self.release_all() {
            out.push(rel);
        }
        self.controller = device.clone();
        out.push(Action::BecomeController(device));
        out
    }

    fn blocked(
        &self,
        canvas: &Canvas,
        monitor: &MonitorId,
        local_mm: (f64, f64),
        heading: q2_layout::Edge,
        modifier_held: bool,
        fullscreen_app: Option<&str>,
    ) -> bool {
        if self.guards.require_modifier && !modifier_held {
            return true;
        }
        if self.guards.block_fullscreen {
            if let Some(app) = fullscreen_app {
                if !self.guards.fullscreen_ignore.contains(app) {
                    return true;
                }
            }
        }
        let zone = self.guards.corner_dead_zone_mm;
        if zone <= 0.0 {
            return false;
        }
        let Some(mon) = canvas.monitor(monitor) else {
            return false;
        };
        let size = mon.effective_mm();
        let along = if heading.is_vertical() {
            local_mm.1
        } else {
            local_mm.0
        };
        let span = if heading.is_vertical() {
            size.height
        } else {
            size.width
        };
        along < zone || along > span - zone
    }

    fn on_edge(
        &mut self,
        canvas: &Canvas,
        monitor: MonitorId,
        local_mm: (f64, f64),
        heading: Edge,
        modifier_held: bool,
        fullscreen_app: Option<String>,
    ) -> Vec<Action> {
        self.cursor = CursorAt {
            monitor: monitor.clone(),
            local_mm,
        };
        if self.blocked(
            canvas,
            &monitor,
            local_mm,
            heading,
            modifier_held,
            fullscreen_app.as_deref(),
        ) {
            return Vec::new();
        }
        let Some(cross) = try_cross(canvas, &monitor, local_mm, heading, EDGE_EPS_MM) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(rel) = self.release_all() {
            out.push(rel);
        }
        self.cursor = CursorAt {
            monitor: cross.dest_monitor.clone(),
            local_mm: cross.dest_local_mm,
        };
        out.push(Action::Cross(cross));
        out
    }

    fn on_jump(&mut self, canvas: &Canvas, device: DeviceId) -> Vec<Action> {
        let (monitor, local_mm) = {
            let Some(primary) = canvas.monitors_of(&device).min_by_key(|m| m.id.local_index) else {
                return Vec::new();
            };
            let size = primary.effective_mm();
            (primary.id.clone(), (size.width / 2.0, size.height / 2.0))
        };
        let mut out = Vec::new();
        if let Some(rel) = self.release_all() {
            out.push(rel);
        }
        self.controller = device.clone();
        self.cursor = CursorAt {
            monitor: monitor.clone(),
            local_mm,
        };
        out.push(Action::JumpTo {
            device,
            monitor,
            local_mm,
        });
        out
    }

    fn on_disconnect(&mut self, device: DeviceId) -> Vec<Action> {
        if self.cursor.monitor.device != device && self.controller != device {
            return Vec::new();
        }
        let mut out = Vec::new();
        if let Some(rel) = self.release_all() {
            out.push(rel);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use q2_layout::{auto_link, Monitor, SizeMm, SizePx};

    use super::*;

    fn id(s: &str) -> DeviceId {
        DeviceId(s.into())
    }

    fn mon(dev: &str, idx: u32, x: f64, y: f64) -> Monitor {
        Monitor {
            id: MonitorId {
                device: id(dev),
                local_index: idx,
            },
            origin_mm: (x, y),
            physical_mm: SizeMm {
                width: 500.0,
                height: 300.0,
            },
            pixels: SizePx {
                width: 1920,
                height: 1080,
            },
            scale: 1.0,
            override_physical_mm: None,
        }
    }

    fn desk() -> Canvas {
        let mut c = Canvas::new();
        c.monitors.push(mon("win1", 0, 0.0, 0.0));
        c.monitors.push(mon("win2", 0, 500.0, 0.0));
        auto_link(&mut c, 2.0);
        c
    }

    fn router() -> Router {
        let c = desk();
        Router::new(
            id("win1"),
            CursorAt {
                monitor: c.monitors[0].id.clone(),
                local_mm: (250.0, 150.0),
            },
        )
    }

    #[test]
    fn last_input_wins_and_releases_keys() {
        let canvas = desk();
        let mut r = router();
        r.hold(Key::Letter('a'));
        let acts = r.handle(
            &canvas,
            Event::PhysicalInput {
                device: id("mac"),
                tick: 10,
            },
        );
        assert!(acts
            .iter()
            .any(|a| matches!(a, Action::BecomeController(d) if d == &id("mac"))));
        assert!(acts
            .iter()
            .any(|a| matches!(a, Action::ReleaseKeys(k) if k.contains(&Key::Letter('a')))));
        assert_eq!(r.controller, id("mac"));
        assert!(r.held_keys.is_empty());
    }

    #[test]
    fn edge_crosses_and_corner_guard_blocks() {
        let canvas = desk();
        let mut r = router();
        let acts = r.handle(
            &canvas,
            Event::EdgeApproach {
                monitor: canvas.monitors[0].id.clone(),
                local_mm: (499.0, 150.0),
                heading: Edge::Right,
                modifier_held: false,
                fullscreen_app: None,
            },
        );
        assert!(acts.iter().any(|a| matches!(a, Action::Cross(_))));
        assert_eq!(r.cursor.monitor.device, id("win2"));

        r.guards.corner_dead_zone_mm = 20.0;
        r.cursor = CursorAt {
            monitor: canvas.monitors[0].id.clone(),
            local_mm: (499.0, 5.0),
        };
        let blocked = r.handle(
            &canvas,
            Event::EdgeApproach {
                monitor: canvas.monitors[0].id.clone(),
                local_mm: (499.0, 5.0),
                heading: Edge::Right,
                modifier_held: false,
                fullscreen_app: None,
            },
        );
        assert!(blocked.is_empty());
    }

    #[test]
    fn jump_goes_to_primary_centre() {
        let canvas = desk();
        let mut r = router();
        r.hold(Key::Letter('x'));
        let acts = r.handle(&canvas, Event::Jump(id("win2")));
        assert!(acts.iter().any(|a| matches!(
            a,
            Action::JumpTo { device, local_mm, .. }
            if *device == id("win2") && (local_mm.0 - 250.0).abs() < 0.1
        )));
        assert!(r.held_keys.is_empty());
    }
}
