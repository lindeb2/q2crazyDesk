//! Tailscale peer list and the 100.64.0.0/10 filter (R-NET).

use std::io;
use std::net::Ipv4Addr;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Deserializer};

/// CGNAT / Tailscale range 100.64.0.0/10 (R-NET-3).
pub const TAILSCALE_PREFIX: Ipv4Addr = Ipv4Addr::new(100, 64, 0, 0);
pub const TAILSCALE_PREFIX_BITS: u8 = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub hostname: String,
    pub dns_name: String,
    pub ips: Vec<Ipv4Addr>,
    pub online: bool,
    pub os: String,
    /// The machine that ran `tailscale status`.
    pub this_machine: bool,
}

#[derive(Debug)]
pub struct Status {
    pub backend_state: String,
    pub health: Vec<String>,
    pub peers: Vec<Peer>,
}

/// Message to show when Tailscale is installed but not in a tailnet yet.
pub fn status_message(status: &Status) -> Option<String> {
    if status.backend_state.is_empty() || status.backend_state == "Running" {
        return None;
    }
    status.health.first().cloned().or_else(|| {
        Some(format!(
            "Tailscale is not connected ({})",
            status.backend_state
        ))
    })
}

#[derive(Debug, Deserialize)]
struct StatusJson {
    #[serde(default, rename = "BackendState")]
    backend_state: String,
    #[serde(default, rename = "Health", deserialize_with = "null_default")]
    health: Vec<String>,
    #[serde(default, rename = "Peer", deserialize_with = "null_default")]
    peer: std::collections::HashMap<String, PeerJson>,
    #[serde(default, rename = "Self")]
    self_node: Option<PeerJson>,
}

fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
struct PeerJson {
    #[serde(default, rename = "HostName")]
    host_name: String,
    #[serde(default, rename = "DNSName")]
    dns_name: String,
    #[serde(default, rename = "TailscaleIPs", deserialize_with = "null_default")]
    tailscale_ips: Vec<String>,
    #[serde(default, rename = "Online")]
    online: bool,
    #[serde(default, rename = "OS")]
    os: String,
}

/// IP whitelist for paired peers.
///
/// An empty RustDesk whitelist allows every address. When nothing is paired,
/// return a loopback address so no remote peer matches (R-DEV-3, R-NET-3).
pub fn paired_whitelist(csv: &str) -> String {
    let mut ips: Vec<String> = csv
        .split(',')
        .filter_map(|s| s.trim().parse::<Ipv4Addr>().ok())
        .filter(|ip| is_tailscale_ip(*ip))
        .map(|ip| ip.to_string())
        .collect();
    ips.sort();
    ips.dedup();
    if ips.is_empty() {
        "127.0.0.1".to_owned()
    } else {
        ips.join(",")
    }
}

pub fn is_tailscale_ip(ip: Ipv4Addr) -> bool {
    let ip_u = u32::from(ip);
    let net = u32::from(TAILSCALE_PREFIX);
    let mask = !0u32 << (32 - TAILSCALE_PREFIX_BITS);
    (ip_u & mask) == (net & mask)
}

/// True when `ip` is a Tailscale address listed in a comma-separated pair list.
pub fn is_paired_ip(csv: &str, ip: Ipv4Addr) -> bool {
    if !is_tailscale_ip(ip) {
        return false;
    }
    csv.split(',')
        .filter_map(|s| s.trim().parse::<Ipv4Addr>().ok())
        .any(|paired| paired == ip)
}

pub fn parse_status_json(json: &str) -> Result<Status, serde_json::Error> {
    let status: StatusJson = serde_json::from_str(json)?;
    let mut peers = Vec::new();
    if let Some(me) = status.self_node {
        peers.push(to_peer(me, true));
    }
    for (_, p) in status.peer {
        peers.push(to_peer(p, false));
    }
    Ok(Status {
        backend_state: status.backend_state,
        health: status.health,
        peers,
    })
}

fn to_peer(p: PeerJson, this_machine: bool) -> Peer {
    let ips = p
        .tailscale_ips
        .iter()
        .filter_map(|s| s.parse::<Ipv4Addr>().ok())
        .filter(|ip| is_tailscale_ip(*ip))
        .collect();
    Peer {
        hostname: p.host_name,
        dns_name: p.dns_name.trim_end_matches('.').to_string(),
        ips,
        online: p.online,
        os: platform_label(&p.os),
        this_machine,
    }
}

/// Tailscale reports `macOS` and `windows`. The peer card only treats `Mac OS` as a Mac.
pub fn platform_label(os: &str) -> String {
    match os.trim().to_ascii_lowercase().as_str() {
        "macos" | "mac" | "darwin" | "mac os" => "Mac OS".to_owned(),
        "windows" | "win" | "win32" => "Windows".to_owned(),
        "linux" => "Linux".to_owned(),
        "android" => "Android".to_owned(),
        "ios" => "iOS".to_owned(),
        _ => os.to_owned(),
    }
}

fn tailscale_bin() -> String {
    if let Some(p) = std::env::var_os("Q2_TAILSCALE") {
        return p.to_string_lossy().into_owned();
    }
    for candidate in tailscale_candidates() {
        if Path::new(candidate).exists() {
            return (*candidate).to_owned();
        }
    }
    "tailscale".to_owned()
}

fn tailscale_candidates() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[r"C:\Program Files\Tailscale\tailscale.exe"]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "/usr/local/bin/tailscale",
            "/opt/homebrew/bin/tailscale",
            "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        ]
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        &["/usr/bin/tailscale"]
    }
}

/// First Tailscale IPv4 of a peer, if any.
pub fn connect_ip(peer: &Peer) -> Option<Ipv4Addr> {
    peer.ips.first().copied()
}

#[derive(Debug)]
pub enum NetError {
    TailscaleNotRunning,
    Io(io::Error),
    Json(serde_json::Error),
    NoJson,
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::TailscaleNotRunning => write!(
                f,
                "Tailscale is not running. Install it from https://tailscale.com/download and sign in to your personal tailnet."
            ),
            NetError::Io(e) => write!(f, "tailscale: {e}"),
            NetError::Json(e) => write!(f, "tailscale status --json: {e}"),
            NetError::NoJson => write!(f, "Tailscale returned no status"),
        }
    }
}

fn tailscale_status() -> io::Result<std::process::Output> {
    let mut cmd = Command::new(tailscale_bin());
    cmd.args(["status", "--json"]);
    // Without a terminal, the Mac app binary opens the GUI and prints nothing.
    #[cfg(target_os = "macos")]
    cmd.env("TAILSCALE_BE_CLI", "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW. A console child of this GUI app otherwise flashes a terminal.
        cmd.creation_flags(0x08000000);
    }
    cmd.output()
}

fn json_object(text: &str) -> &str {
    let trimmed = text.trim();
    let Some(start) = trimmed.find('{') else {
        return "";
    };
    let Some(end) = trimmed.rfind('}') else {
        return "";
    };
    if end < start {
        return "";
    }
    &trimmed[start..=end]
}

/// Read peers from `tailscale status --json` (R-NET-4, R-NET-5).
pub fn read_status() -> Result<Status, NetError> {
    let output = match tailscale_status() {
        Ok(o) => o,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(NetError::TailscaleNotRunning);
        }
        Err(e) => return Err(NetError::Io(e)),
    };
    if !output.status.success() {
        return Err(NetError::TailscaleNotRunning);
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let json = json_object(&raw);
    if json.is_empty() {
        return Err(NetError::NoJson);
    }
    parse_status_json(json).map_err(NetError::Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_whitelist_drops_other_ranges() {
        assert_eq!(paired_whitelist(""), "127.0.0.1");
        assert_eq!(paired_whitelist("10.0.0.2,192.168.1.1"), "127.0.0.1");
        assert_eq!(
            paired_whitelist("100.90.0.2, 10.1.1.1, 100.64.0.9"),
            "100.64.0.9,100.90.0.2"
        );
    }

    #[test]
    fn range_filter() {
        assert!(is_tailscale_ip(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(is_tailscale_ip(Ipv4Addr::new(100, 127, 255, 254)));
        assert!(!is_tailscale_ip(Ipv4Addr::new(100, 128, 0, 1)));
        assert!(!is_tailscale_ip(Ipv4Addr::new(192, 168, 1, 10)));
        assert!(!is_tailscale_ip(Ipv4Addr::new(10, 0, 0, 2)));
    }

    #[test]
    fn paired_ip_requires_tailscale_and_list() {
        let list = "100.64.0.9,100.90.0.2";
        assert!(is_paired_ip(list, Ipv4Addr::new(100, 90, 0, 2)));
        assert!(!is_paired_ip(list, Ipv4Addr::new(100, 90, 0, 3)));
        assert!(!is_paired_ip(list, Ipv4Addr::new(10, 0, 0, 2)));
        assert!(!is_paired_ip("10.0.0.2", Ipv4Addr::new(10, 0, 0, 2)));
    }

    #[test]
    fn parse_peers_keeps_only_100() {
        let json = r#"{
            "Self": {
                "HostName": "win1",
                "DNSName": "win1.tailnet.ts.net.",
                "TailscaleIPs": ["100.64.1.2", "fd7a:115c::1"],
                "Online": true,
                "OS": "windows"
            },
            "Peer": {
                "n2": {
                    "HostName": "mac",
                    "DNSName": "mac.tailnet.ts.net.",
                    "TailscaleIPs": ["100.64.9.9", "192.168.1.5"],
                    "Online": true,
                    "OS": "macos"
                }
            }
        }"#;
        let status = parse_status_json(json).unwrap();
        assert!(status_message(&status).is_none());
        let peers = status.peers;
        assert_eq!(peers.len(), 2);
        assert!(peers.iter().any(|p| p.this_machine && p.hostname == "win1"));
        let mac = peers.iter().find(|p| p.hostname == "mac").unwrap();
        assert!(!mac.this_machine);
        assert_eq!(mac.os, "Mac OS");
        assert_eq!(mac.ips, vec![Ipv4Addr::new(100, 64, 9, 9)]);
        assert_eq!(mac.dns_name, "mac.tailnet.ts.net");
        assert_eq!(connect_ip(mac), Some(Ipv4Addr::new(100, 64, 9, 9)));
        let me = peers.iter().find(|p| p.this_machine).unwrap();
        assert_eq!(me.os, "Windows");
    }

    #[test]
    fn json_object_drops_a_prefix() {
        let text = "Tailscale GUI\n{\"BackendState\":\"Running\",\"Peer\":null}";
        let status = parse_status_json(json_object(text)).unwrap();
        assert_eq!(status.backend_state, "Running");
        assert!(json_object("").is_empty());
    }

    #[test]
    fn starting_status_allows_null_lists() {
        let json = r#"{
            "BackendState": "NoState",
            "TailscaleIPs": null,
            "Self": {
                "HostName": "win1",
                "DNSName": "",
                "TailscaleIPs": null,
                "Online": false,
                "OS": "windows"
            },
            "Health": ["Tailscale is starting. Please wait."],
            "Peer": null
        }"#;
        let status = parse_status_json(json).unwrap();
        assert_eq!(status.peers.len(), 1);
        assert!(status.peers[0].ips.is_empty());
        assert_eq!(
            status_message(&status).as_deref(),
            Some("Tailscale is starting. Please wait.")
        );
    }
}
