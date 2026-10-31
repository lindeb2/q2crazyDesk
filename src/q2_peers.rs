//! Put Tailscale peers into RustDesk's discovered-peer list (R-NET-4, R-DEV-4).

use std::{
    collections::HashMap,
    net::Ipv4Addr,
    sync::Mutex,
    time::{Duration, Instant},
};

use hbb_common::config::{Config, DiscoveryPeer, LanPeers, LocalConfig};

use q2_net::{self, Peer};

const NET_ERROR_KEY: &str = "q2-net-error";
pub const PAIRED_KEY: &str = "q2-paired";
const REFRESH_GAP: Duration = Duration::from_secs(10);

static LAST_REFRESH: Mutex<Option<Instant>> = Mutex::new(None);
static TAILSCALE_RUNNING: Mutex<bool> = Mutex::new(false);
static TAILSCALE_ONLINE: Mutex<Option<HashMap<String, bool>>> = Mutex::new(None);

pub fn is_tailscale_running() -> bool {
    let last = *LAST_REFRESH.lock().unwrap();
    if last.is_none() {
        refresh();
    }
    *TAILSCALE_RUNNING.lock().unwrap()
}

pub fn query_onlines(ids: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut onlines = Vec::new();
    let mut offlines = Vec::new();
    let mut remaining = Vec::new();

    let map = TAILSCALE_ONLINE.lock().unwrap();
    let lan_peers = if map.is_none() {
        Some(LanPeers::load().peers)
    } else {
        None
    };

    for id in ids {
        if is_tailscale_peer_id(id) {
            let online = if let Some(ref m) = *map {
                m.get(id).copied().unwrap_or(false)
            } else if let Some(ref peers) = lan_peers {
                peers.iter().any(|p| &p.id == id && p.online)
            } else {
                false
            };
            if online {
                onlines.push(id.clone());
            } else {
                offlines.push(id.clone());
            }
        } else {
            remaining.push(id.clone());
        }
    }
    (onlines, offlines, remaining)
}

pub fn whitelist_for(paired_csv: &str) -> String {
    if paired_csv.trim().is_empty() {
        let peers = LanPeers::load().peers;
        let ips: Vec<String> = peers
            .into_iter()
            .filter(|p| is_tailscale_peer_id(&p.id))
            .map(|p| p.id)
            .collect();
        if !ips.is_empty() {
            return ips.join(",");
        }
    }
    q2_net::paired_whitelist(paired_csv)
}

pub fn apply_whitelist() {
    let paired = Config::get_option(PAIRED_KEY);
    Config::set_option("whitelist".to_owned(), whitelist_for(&paired));
}

pub fn is_paired_peer(ip: &str) -> bool {
    let Ok(addr) = ip.parse::<Ipv4Addr>() else {
        return false;
    };
    if !q2_net::is_tailscale_ip(addr) {
        return false;
    }
    let paired = Config::get_option(PAIRED_KEY);
    if paired.trim().is_empty() {
        return true;
    }
    q2_net::is_paired_ip(&paired, addr)
}

pub fn refresh() {
    if !refresh_due() {
        return;
    }
    match q2_net::read_status() {
        Ok(status) => {
            let message = q2_net::status_message(&status).unwrap_or_default();
            *TAILSCALE_RUNNING.lock().unwrap() = message.is_empty();
            LocalConfig::set_flutter_option(NET_ERROR_KEY.to_owned(), message);
            store_peers(&status.peers);
        }
        Err(err) => {
            *TAILSCALE_RUNNING.lock().unwrap() = false;
            let down = matches!(err, q2_net::NetError::TailscaleNotRunning);
            LocalConfig::set_flutter_option(NET_ERROR_KEY.to_owned(), err.to_string());
            if down {
                mark_tailscale_offline();
            }
            notify();
        }
    }
}

fn store_peers(status: &[Peer]) {
    {
        let mut map = TAILSCALE_ONLINE.lock().unwrap();
        let entries = map.get_or_insert_with(HashMap::new);
        entries.clear();
        for remote in status.iter().filter(|p| !p.this_machine) {
            if let Some(ip) = q2_net::connect_ip(remote) {
                entries.insert(ip.to_string(), remote.online);
            }
        }
    }
    let mut peers = LanPeers::load().peers;
    let mut seen = Vec::new();
    for remote in status.iter().filter(|p| !p.this_machine) {
        let Some(ip) = q2_net::connect_ip(remote) else {
            continue;
        };
        seen.push(ip);
        let id = ip.to_string();
        if let Some(existing) = peers.iter_mut().find(|p| p.id == id) {
            fill(existing, remote, ip);
        } else {
            let mut peer = DiscoveryPeer::default();
            fill(&mut peer, remote, ip);
            peers.insert(0, peer);
        }
    }
    for peer in peers.iter_mut() {
        if is_tailscale_peer_id(&peer.id) && !seen.iter().any(|ip| ip.to_string() == peer.id) {
            peer.online = false;
        }
    }
    LanPeers::store(&peers);
    notify();
}

fn fill(peer: &mut DiscoveryPeer, remote: &Peer, ip: Ipv4Addr) {
    peer.id = ip.to_string();
    peer.hostname = remote.hostname.clone();
    peer.username = if remote.dns_name.is_empty() {
        remote.hostname.clone()
    } else {
        remote.dns_name.clone()
    };
    peer.platform = remote.os.clone();
    peer.online = remote.online;
    peer.ip_mac.insert(ip.to_string(), String::new());
}

fn mark_tailscale_offline() {
    {
        let mut map = TAILSCALE_ONLINE.lock().unwrap();
        if let Some(entries) = map.as_mut() {
            for v in entries.values_mut() {
                *v = false;
            }
        }
    }
    let mut peers = LanPeers::load().peers;
    let mut changed = false;
    for peer in peers.iter_mut() {
        if is_tailscale_peer_id(&peer.id) && peer.online {
            peer.online = false;
            changed = true;
        }
    }
    if changed {
        LanPeers::store(&peers);
    }
}

fn refresh_due() -> bool {
    let mut last = LAST_REFRESH.lock().unwrap();
    if last.is_some_and(|t| t.elapsed() < REFRESH_GAP) {
        return false;
    }
    *last = Some(Instant::now());
    true
}

pub(crate) fn is_tailscale_peer_id(id: &str) -> bool {
    id.parse::<Ipv4Addr>()
        .map(q2_net::is_tailscale_ip)
        .unwrap_or(false)
}

fn notify() {
    #[cfg(feature = "flutter")]
    crate::flutter_ffi::main_load_lan_peers();
}
