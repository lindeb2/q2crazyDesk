//! Product name, Tailscale-only direct access, and peer refresh (R-NET-3, R-NET-4).

use std::sync::Once;
use std::thread;
use std::time::Duration;

use hbb_common::config::Config;

pub const PRODUCT_NAME: &str = "q2crazyDesk";

pub fn apply() {
    *hbb_common::config::APP_NAME.write().unwrap() = PRODUCT_NAME.to_owned();
    // Direct IP is unencrypted. Only paired Tailscale addresses are accepted (R-NET-3, R-DEV-3).
    crate::q2_peers::apply_whitelist();
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
