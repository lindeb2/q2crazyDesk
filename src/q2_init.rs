//! Product name, Tailscale-only direct access, and peer refresh (R-NET-3, R-NET-4).

use std::sync::Once;
use std::thread;
use std::time::Duration;

use hbb_common::config::Config;

pub const PRODUCT_NAME: &str = "q2crazyDesk";
pub const TAILSCALE_CIDR: &str = "100.64.0.0/10";

pub fn apply() {
    *hbb_common::config::APP_NAME.write().unwrap() = PRODUCT_NAME.to_owned();
    if Config::get_option("whitelist").is_empty() {
        Config::set_option("whitelist".to_owned(), TAILSCALE_CIDR.to_owned());
    }
    // Direct IP is how peers connect. It is unencrypted, so the whitelist above
    // keeps it inside Tailscale (R-NET-3).
    Config::set_option("direct-server".to_owned(), "Y".to_owned());
    static START: Once = Once::new();
    START.call_once(|| {
        thread::Builder::new()
            .name("q2-tailscale".to_owned())
            .spawn(|| loop {
                crate::q2_peers::refresh();
                thread::sleep(Duration::from_secs(15));
            })
            .ok();
    });
}
