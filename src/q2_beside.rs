//! Beside placement on the controlling computer (R-PLACE-1, R-INPUT-1,
//! R-INPUT-2, R-SVC-3).
//!
//! Glue between three parts: the low-level hooks (`q2_hook`), the pure state
//! machine in `q2-router`, and the headless peer session (`q2_session`). The
//! hook thread asks the state machine what to do with each physical input
//! event and carries out the answer on the peer's session. A control thread
//! keeps one session open to the first paired Tailscale peer that is online,
//! tells the state machine when that session is usable, and drops a session
//! that stays unusable (docs/DECISIONS.md D-014).

#![allow(dead_code)]

#[cfg(all(feature = "flutter", not(any(target_os = "android", target_os = "ios"))))]
mod imp {
    use std::sync::Mutex;

    use q2_router::{Beside, Button, Out};

    use crate::common::input::{
        MOUSE_BUTTON_BACK, MOUSE_BUTTON_FORWARD, MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT,
        MOUSE_BUTTON_WHEEL, MOUSE_TYPE_DOWN, MOUSE_TYPE_MOVE, MOUSE_TYPE_UP, MOUSE_TYPE_WHEEL,
    };
    use crate::flutter::FlutterSession;
    use crate::q2_session;

    const WHEEL_NOTCH: i32 = 120;

    static STATE: Mutex<Option<Beside>> = Mutex::new(None);
    /// The peer that currently receives input, set by `Enter` and cleared by `Leave`.
    static ACTIVE: Mutex<Option<String>> = Mutex::new(None);
    /// Wheel movement that has not yet added up to a whole notch: (vertical, horizontal).
    static WHEEL_REST: Mutex<(i32, i32)> = Mutex::new((0, 0));

    pub(super) fn with_beside<R>(f: impl FnOnce(&mut Beside) -> R) -> Option<R> {
        let mut slot = STATE.lock().ok()?;
        Some(f(slot.get_or_insert_with(Beside::new)))
    }

    fn with_active(f: impl FnOnce(&FlutterSession)) {
        let peer = ACTIVE.lock().ok().and_then(|p| p.clone());
        let Some(peer) = peer else {
            return;
        };
        if let Some(session) = q2_session::session_for(&peer) {
            f(&session);
        }
    }

    fn button_bit(button: Button) -> i32 {
        match button {
            Button::Left => MOUSE_BUTTON_LEFT,
            Button::Right => MOUSE_BUTTON_RIGHT,
            Button::Middle => MOUSE_BUTTON_WHEEL,
            Button::Back => MOUSE_BUTTON_BACK,
            Button::Forward => MOUSE_BUTTON_FORWARD,
        }
    }

    /// Whole notches out of `delta`, keeping the remainder for the next event.
    fn take_notches(rest: &mut i32, delta: i32) -> i32 {
        let total = rest.saturating_add(delta);
        *rest = total % WHEEL_NOTCH;
        total / WHEEL_NOTCH
    }

    pub(super) fn execute(outs: Vec<Out>) {
        for out in outs {
            match out {
                Out::Warp { x, y } => warp(x, y),
                Out::Enter { peer, x, y } => {
                    if let Ok(mut active) = ACTIVE.lock() {
                        *active = Some(peer.clone());
                    }
                    if let Ok(mut rest) = WHEEL_REST.lock() {
                        *rest = (0, 0);
                    }
                    if let Some(session) = q2_session::session_for(&peer) {
                        session.send_mouse(MOUSE_TYPE_MOVE, x, y, false, false, false, false);
                    }
                }
                Out::MoveTo { x, y } => with_active(|s| {
                    s.send_mouse(MOUSE_TYPE_MOVE, x, y, false, false, false, false)
                }),
                Out::Button { button, down } => with_active(|s| {
                    let kind = if down { MOUSE_TYPE_DOWN } else { MOUSE_TYPE_UP };
                    let mask = (button_bit(button) << 3) | kind;
                    s.send_mouse(mask, 0, 0, false, false, false, false);
                }),
                Out::Wheel { vertical, delta } => {
                    let notches = match WHEEL_REST.lock() {
                        Ok(mut rest) => {
                            if vertical {
                                take_notches(&mut rest.0, delta)
                            } else {
                                take_notches(&mut rest.1, delta)
                            }
                        }
                        Err(_) => 0,
                    };
                    if notches != 0 {
                        // The protocol counts up as positive y and left as positive x.
                        let (x, y) = if vertical { (0, notches) } else { (-notches, 0) };
                        with_active(|s| {
                            s.send_mouse(MOUSE_TYPE_WHEEL, x, y, false, false, false, false)
                        });
                    }
                }
                Out::Key {
                    vk,
                    scan,
                    extended,
                    down,
                } => with_active(|s| send_key(s, vk, scan, extended, down)),
                Out::Leave { .. } => {
                    if let Ok(mut active) = ACTIVE.lock() {
                        *active = None;
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    fn warp(x: i32, y: i32) {
        crate::platform::windows::set_cursor_pos(x, y);
    }

    #[cfg(not(windows))]
    fn warp(_x: i32, _y: i32) {}

    /// Send a physical key the way RustDesk's own client does in map mode, so
    /// scan codes are translated for the peer's operating system.
    #[cfg(windows)]
    fn send_key(session: &FlutterSession, vk: u32, scan: u32, extended: bool, down: bool) {
        use rdev::{Event, EventType};
        use std::time::SystemTime;

        // Same rule as rdev's own Windows hook: right shift and num lock have
        // their own scan codes and carry no extended prefix.
        let position_code = match scan {
            0x36 | 0x45 => scan,
            _ if extended => (0xE0 << 8) | scan,
            _ => scan,
        };
        let key = rdev::win_key_from_keycode(vk);
        let event_type = if down {
            EventType::KeyPress(key)
        } else {
            EventType::KeyRelease(key)
        };
        let event = Event {
            time: SystemTime::now(),
            unicode: None,
            event_type,
            platform_code: vk,
            position_code,
            usb_hid: 0,
            extra_data: 0,
        };
        crate::keyboard::client::process_event_with_session("map", &event, None, &*session);
    }

    #[cfg(not(windows))]
    fn send_key(_session: &FlutterSession, _vk: u32, _scan: u32, _extended: bool, _down: bool) {}

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn small_wheel_steps_add_up_to_notches() {
            let mut rest = 0;
            assert_eq!(take_notches(&mut rest, 40), 0);
            assert_eq!(take_notches(&mut rest, 40), 0);
            assert_eq!(take_notches(&mut rest, 40), 1);
            assert_eq!(rest, 0);
            assert_eq!(take_notches(&mut rest, -240), -2);
            assert_eq!(take_notches(&mut rest, -60), 0);
            assert_eq!(take_notches(&mut rest, -60), -1);
        }
    }
}

/// Start Beside on this computer. Only the main window process does anything.
pub fn start() {
    #[cfg(all(
        windows,
        feature = "flutter",
        not(any(target_os = "android", target_os = "ios"))
    ))]
    windows::start();
}

/// Keeps one session open to a paired peer and tells the state machine when it
/// is usable. Not Windows-specific, so it is also compiled and checked on Linux.
#[cfg(all(feature = "flutter", not(any(target_os = "android", target_os = "ios"))))]
mod control {
    use std::time::{Duration, Instant};

    use hbb_common::{config::LanPeers, log};
    use q2_router::Target;

    use super::imp::{execute, with_beside};
    use crate::q2_session;

    pub(super) const TICK: Duration = Duration::from_secs(1);
    /// A session that stays unusable this long is closed and opened again.
    const STALE: Duration = Duration::from_secs(30);
    const DEFAULT_SIZE: (i32, i32) = (1920, 1080);

    pub(super) struct Link {
        peer: String,
        size: (i32, i32),
        ready: bool,
        since: Instant,
    }

    /// The first paired Tailscale peer that is online.
    fn pick_peer() -> Option<String> {
        LanPeers::load()
            .peers
            .into_iter()
            .find(|p| p.online && crate::q2_peers::is_paired_peer(&p.id))
            .map(|p| p.id)
    }

    fn display_size(peer: &str) -> Option<(i32, i32)> {
        let session = q2_session::session_for(peer)?;
        let lc = session.lc.read().ok()?;
        let info = lc.peer_info.as_ref()?;
        let display = info
            .displays
            .iter()
            .find(|d| d.x == 0 && d.y == 0)
            .or_else(|| info.displays.first())?;
        Some((display.width, display.height))
    }

    fn forget(peer: &str) {
        if let Some(outs) = with_beside(|b| {
            b.set_target(None);
            b.set_ready(false)
        }) {
            execute(outs);
        }
        q2_session::disconnect(peer);
    }

    pub(super) fn drive(link: &mut Option<Link>) {
        let wanted = pick_peer();
        if link.as_ref().map(|l| &l.peer) != wanted.as_ref() {
            if let Some(old) = link.take() {
                forget(&old.peer);
            }
            if let Some(peer) = wanted {
                match q2_session::connect(&peer) {
                    Ok(_) => {
                        *link = Some(Link {
                            peer,
                            size: DEFAULT_SIZE,
                            ready: false,
                            since: Instant::now(),
                        })
                    }
                    Err(err) => log::warn!("q2 beside: could not open a session to {peer}: {err}"),
                }
            }
        }
        let Some(l) = link.as_mut() else {
            return;
        };
        let ready = q2_session::is_ready(&l.peer);
        if ready {
            if let Some(size) = display_size(&l.peer) {
                l.size = size;
            }
        }
        if ready != l.ready {
            l.ready = ready;
            l.since = Instant::now();
        }
        if !ready && l.since.elapsed() > STALE {
            if let Some(old) = link.take() {
                log::info!("q2 beside: session to {} stayed down, reconnecting", old.peer);
                forget(&old.peer);
            }
            return;
        }
        let target = Target {
            peer: l.peer.clone(),
            size_px: l.size,
        };
        if let Some(outs) = with_beside(|b| {
            b.set_target(Some(target));
            b.set_ready(ready)
        }) {
            execute(outs);
        }
    }
}

#[cfg(all(
    windows,
    feature = "flutter",
    not(any(target_os = "android", target_os = "ios"))
))]
mod windows {
    use std::thread;

    use hbb_common::log;

    use super::control::{drive, Link, TICK};
    use super::imp::{execute, with_beside};

    pub fn start() {
        static START: std::sync::Once = std::sync::Once::new();
        START.call_once(start_once);
    }

    fn start_once() {
        if !crate::q2_local::is_main_window() {
            return;
        }
        let installed = crate::q2_hook::install(Box::new(|input| {
            let step = with_beside(|b| b.handle(input)).unwrap_or_default();
            let swallow = step.swallow;
            execute(step.out);
            swallow
        }));
        if !installed {
            log::error!("q2 beside: the input hooks did not start");
            return;
        }
        let spawned = thread::Builder::new()
            .name("q2-beside".to_owned())
            .spawn(control_loop);
        if let Err(err) = spawned {
            log::error!("q2 beside: control thread did not start: {err}");
        }
    }

    fn control_loop() {
        let mut panels = Vec::new();
        let mut link: Option<Link> = None;
        loop {
            thread::sleep(TICK);
            let now = crate::q2_local::read_panels();
            if now != panels {
                panels = now.clone();
                with_beside(|b| b.set_local_panels(now));
            }
            drive(&mut link);
        }
    }
}
