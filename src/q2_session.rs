//! Headless peer sessions for Beside placement (R-PLACE-1, R-PLACE-5).
//!
//! Beside needs input, clipboard and audio from a peer, but no picture. This is
//! RustDesk's own client session without a Flutter window. Nothing registers an
//! event stream, and every `push_event` skips a handler that has none. Once the
//! peer's info has arrived, the session drops all of its video subscriptions
//! with `CaptureDisplays { set: [] }`. Inside later subscribes again on the
//! same session, so switching placement does not reconnect (section 8.2).

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use hbb_common::{anyhow::anyhow, log, rendezvous_proto::ConnType, ResultType};

use crate::flutter::{self, sessions, FlutterSession};
use crate::flutter_ffi::SessionID;
use crate::ui_session_interface::io_loop;

/// Peer address (a Tailscale `100.x` address) to the session id that q2 opened.
static BESIDE: Mutex<Option<HashMap<String, SessionID>>> = Mutex::new(None);

const POLL: Duration = Duration::from_millis(100);

fn remember(peer: &str, id: SessionID) -> ResultType<()> {
    let mut map = BESIDE
        .lock()
        .map_err(|_| anyhow!("q2 session table is poisoned"))?;
    map.get_or_insert_with(HashMap::new)
        .insert(peer.to_owned(), id);
    Ok(())
}

fn id_for(peer: &str) -> Option<SessionID> {
    let map = BESIDE.lock().ok()?;
    map.as_ref()?.get(peer).copied()
}

/// The running session to `peer`, if q2 has opened one.
pub fn session_for(peer: &str) -> Option<FlutterSession> {
    sessions::get_session_by_session_id(&id_for(peer)?)
}

/// True once login has finished and the peer's info is known.
pub fn is_ready(peer: &str) -> bool {
    session_for(peer).is_some_and(|s| s.lc.read().map(|lc| lc.peer_info.is_some()).unwrap_or(false))
}

/// Open a session to `peer` (or return the one that is already open) and keep
/// its video off. A paired Tailscale peer logs in without a password.
pub fn connect(peer: &str) -> ResultType<FlutterSession> {
    if let Some(session) = session_for(peer) {
        return Ok(session);
    }
    let session_id = SessionID::new_v4();
    flutter::session_add(
        &session_id,
        peer,
        false,
        false,
        false,
        false,
        false,
        "",
        false,
        String::new(),
        false,
        None,
    )?;
    remember(peer, session_id)?;
    // `insert_session` keeps an existing session to the same peer and only adds
    // a handler to it, so always use what the table holds.
    let session = sessions::get_session_by_session_id(&session_id)
        .ok_or_else(|| anyhow!("session to {peer} vanished"))?;
    // The count includes the handler that was just added.
    if sessions::get_session_count(peer.to_owned(), ConnType::DEFAULT_CONN) != 1 {
        // A window already owns this connection. Leave its video alone.
        log::info!("q2 shares the open session to {peer}");
        return Ok(session);
    }
    let for_loop = (*session).clone();
    thread::Builder::new()
        .name("q2-beside-io".to_owned())
        .spawn(move || {
            let round = match for_loop.connection_round_state.lock() {
                Ok(mut state) => state.new_round(),
                Err(_) => return,
            };
            io_loop(for_loop, round);
        })?;
    keep_video_off(session_id);
    Ok(session)
}

/// Every time the peer's info (re)arrives, which is after each login and each
/// reconnect, drop all video subscriptions. Ends when the session is gone.
fn keep_video_off(session_id: SessionID) {
    let spawned = thread::Builder::new()
        .name("q2-beside-video-off".to_owned())
        .spawn(move || {
            let mut was_ready = false;
            loop {
                thread::sleep(POLL);
                let Some(session) = sessions::get_session_by_session_id(&session_id) else {
                    return;
                };
                let ready = session
                    .lc
                    .read()
                    .map(|lc| lc.peer_info.is_some())
                    .unwrap_or(false);
                if ready && !was_ready {
                    session.capture_displays(vec![], vec![], vec![]);
                }
                was_ready = ready;
            }
        });
    if let Err(err) = spawned {
        log::error!("q2 could not start the video-off watcher: {err}");
    }
}

/// Close the session q2 opened to `peer` and forget it.
pub fn disconnect(peer: &str) {
    let Some(id) = id_for(peer) else {
        return;
    };
    if let Some(session) = sessions::get_session_by_session_id(&id) {
        session.close();
    }
    sessions::remove_session_by_session_id(&id);
    if let Ok(mut map) = BESIDE.lock() {
        if let Some(map) = map.as_mut() {
            map.remove(peer);
        }
    }
}
