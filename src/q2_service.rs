//! Answers a paired peer's questions about this device, for now the physical
//! size of its monitors (R-LAYOUT-2, R-LAYOUT-3; docs/DECISIONS.md D-015).
//!
//! RustDesk's protocol has no field for millimetres, so every q2 device runs
//! the small service in `q2-sync` on its own port. It answers paired Tailscale
//! addresses only. Both the main window and the `--server` process (the one in
//! the user's session that captures the screen) try to host it; whichever
//! binds the port first keeps it.

use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Once};
use std::time::Duration;

use hbb_common::{config::{Config, LanPeers}, log};
use q2_layout::{Canvas, OsPanel};
use q2_sync::{
    service::{self, Allow, LayoutProvider, OnClaim, OnLayout, OnWake, Provider},
    Versioned,
};

use crate::q2_peers::PAIRED_KEY;

pub const LAYOUT_KEY: &str = "q2-layout-doc";

/// Load the locally stored synced layout document, if any.
pub fn load_layout() -> Option<Versioned<Canvas>> {
    let raw = Config::get_option(LAYOUT_KEY);
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

/// Save the synced layout document locally.
pub fn save_layout(doc: &Versioned<Canvas>) {
    if let Ok(json) = serde_json::to_string(doc) {
        Config::set_option(LAYOUT_KEY.to_owned(), json);
    }
}

/// Update local layout from incoming peer layout (last-writer-wins).
pub fn update_incoming_layout(incoming: Versioned<Canvas>) -> (bool, Option<Versioned<Canvas>>) {
    let mut current = load_layout();
    if let Some(cur) = current.as_mut() {
        let accepted = cur.merge_incoming(incoming);
        if accepted {
            save_layout(cur);
            notify_layout_updated();
        }
        (accepted, Some(cur.clone()))
    } else {
        save_layout(&incoming);
        notify_layout_updated();
        (true, Some(incoming))
    }
}

fn notify_layout_updated() {
    #[cfg(feature = "flutter")]
    crate::flutter_ffi::main_on_layout_updated();
}

/// Broadcast the layout document to all online discovered peers.
pub fn broadcast_layout(doc: &Versioned<Canvas>) {
    save_layout(doc);
    let peers = LanPeers::load().peers;
    for peer in peers {
        if peer.online {
            if let Ok(ip) = peer.id.parse::<IpAddr>() {
                let addr = SocketAddr::new(ip, service::PORT);
                let doc_clone = doc.clone();
                let _ = std::thread::Builder::new()
                    .name("q2-broadcast-layout".to_owned())
                    .spawn(move || {
                        let _ = service::push_layout(addr, Duration::from_millis(1500), doc_clone);
                    });
            }
        }
    }
}

/// Synchronize the layout document with a peer at `addr`.
pub fn sync_layout_with_peer(addr: SocketAddr, timeout: Duration) {
    if let Ok(remote_doc) = service::fetch_layout(addr, timeout) {
        if let Some(remote) = remote_doc {
            update_incoming_layout(remote);
        }
    }
    if let Some(local) = load_layout() {
        let _ = service::push_layout(addr, timeout, local);
    }
}

/// Only paired Tailscale addresses are answered.
pub fn paired_allow() -> Allow {
    Arc::new(|ip| match ip {
        IpAddr::V4(v4) => q2_net::is_paired_ip(&Config::get_option(PAIRED_KEY), v4),
        IpAddr::V6(_) => false,
    })
}

pub fn start() {
    static START: Once = Once::new();
    START.call_once(|| {
        if !hosts_service() {
            return;
        }
        let provider: Provider = Arc::new(panels);
        // Claims of control belong to the main window, which owns the input
        // hooks (`q2_beside`). This port only answers questions.
        let on_claim: OnClaim = Arc::new(|_| false);
        let layout_provider: LayoutProvider = Arc::new(load_layout);
        let on_layout: OnLayout = Arc::new(update_incoming_layout);
        let on_wake: OnWake = Arc::new(|mac| {
            q2_wake::send_magic_packet(q2_wake::MacAddr(mac)).is_ok()
        });
        let bind = SocketAddr::from(([0, 0, 0, 0], service::PORT));
        // The service keeps running in its own threads for the life of the process.
        if let Err(err) = service::serve_with(
            bind,
            paired_allow(),
            provider,
            on_claim,
            layout_provider,
            on_layout,
            on_wake,
        ) {
            log::info!("q2 service not started here (another process may own the port): {err}");
        }
    });
}

#[cfg(windows)]
fn hosts_service() -> bool {
    // The main window (no mode argument) and `--server` host it. The Windows
    // service itself runs in session 0, where there are no monitors to ask.
    let mode = std::env::args().skip(1).find(|arg| arg.starts_with("--"));
    matches!(mode.as_deref(), None | Some("--server"))
}

#[cfg(not(windows))]
fn hosts_service() -> bool {
    true
}

#[cfg(windows)]
pub fn panels() -> Vec<OsPanel> {
    crate::q2_local::read_panels()
}

/// Active displays with the physical size macOS reports for each (from EDID).
/// Sizes are in the same units the session reports: `CGDisplayPixelsWide/High`.
#[cfg(target_os = "macos")]
pub fn panels() -> Vec<OsPanel> {
    use core_graphics::display::CGDisplay;

    let Ok(ids) = CGDisplay::active_displays() else {
        return Vec::new();
    };
    ids.into_iter()
        .enumerate()
        .map(|(index, id)| {
            let display = CGDisplay::new(id);
            let bounds = display.bounds();
            let size = display.screen_size();
            OsPanel {
                index: index as u32,
                x: bounds.origin.x as i32,
                y: bounds.origin.y as i32,
                width: display.pixels_wide() as u32,
                height: display.pixels_high() as u32,
                width_mm: size.width,
                height_mm: size.height,
                scale: 1.0,
            }
        })
        .collect()
}

/// Linux reports no monitor sizes yet. Peers fall back to 96 dpi.
#[cfg(not(any(windows, target_os = "macos")))]
pub fn panels() -> Vec<OsPanel> {
    Vec::new()
}
