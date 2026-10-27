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

use hbb_common::{config::Config, log};
use q2_layout::OsPanel;
use q2_sync::service::{self, Allow, Provider};

use crate::q2_peers::PAIRED_KEY;

pub fn start() {
    static START: Once = Once::new();
    START.call_once(|| {
        if !hosts_service() {
            return;
        }
        let allow: Allow = Arc::new(|ip| match ip {
            IpAddr::V4(v4) => q2_net::is_paired_ip(&Config::get_option(PAIRED_KEY), v4),
            IpAddr::V6(_) => false,
        });
        let provider: Provider = Arc::new(panels);
        let bind = SocketAddr::from(([0, 0, 0, 0], service::PORT));
        // The service keeps running in its own threads for the life of the process.
        if let Err(err) = service::serve(bind, allow, provider) {
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
fn panels() -> Vec<OsPanel> {
    crate::q2_local::read_panels()
}

/// Only Windows reports monitor sizes so far. Peers fall back to 96 dpi.
#[cfg(not(windows))]
fn panels() -> Vec<OsPanel> {
    Vec::new()
}
