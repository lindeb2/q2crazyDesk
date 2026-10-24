//! Product name and Tailscale-only IP default (R-NET-3). Additive q2 hook.

use hbb_common::config::Config;

pub const PRODUCT_NAME: &str = "q2crazyDesk";
pub const TAILSCALE_CIDR: &str = "100.64.0.0/10";

pub fn apply() {
    *hbb_common::config::APP_NAME.write().unwrap() = PRODUCT_NAME.to_owned();
    if Config::get_option("whitelist").is_empty() {
        Config::set_option("whitelist".to_owned(), TAILSCALE_CIDR.to_owned());
    }
}
