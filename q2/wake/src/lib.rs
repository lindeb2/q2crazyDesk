//! Wake-on-LAN magic packet and the away-from-home relay (R-WAKE).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split([':', '-']).collect();
        if parts.len() != 6 {
            return None;
        }
        let mut bytes = [0u8; 6];
        for (i, p) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(p, 16).ok()?;
        }
        Some(Self(bytes))
    }
}

/// 6 × 0xFF followed by the MAC repeated 16 times.
pub fn magic_packet(mac: MacAddr) -> [u8; 102] {
    let mut pkt = [0u8; 102];
    for b in pkt.iter_mut().take(6) {
        *b = 0xff;
    }
    for i in 0..16 {
        let off = 6 + i * 6;
        pkt[off..off + 6].copy_from_slice(&mac.0);
    }
    pkt
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeRelayRequest {
    pub target_mac: MacAddr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakePath {
    /// Requester is on the home LAN and sends the packet itself.
    Direct,
    /// Requester is away; an online home peer must send it (R-WAKE-2).
    Relay,
}

pub fn choose_path(requester_on_home_lan: bool) -> WakePath {
    if requester_on_home_lan {
        WakePath::Direct
    } else {
        WakePath::Relay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_layout() {
        let mac = MacAddr::parse("aa:bb:cc:dd:ee:ff").unwrap();
        let pkt = magic_packet(mac);
        assert!(pkt[..6].iter().all(|b| *b == 0xff));
        assert_eq!(&pkt[6..12], &mac.0);
        assert_eq!(&pkt[96..102], &mac.0);
        assert_eq!(pkt.len(), 102);
    }

    #[test]
    fn parse_hyphen_and_colon() {
        assert_eq!(
            MacAddr::parse("AA-BB-CC-DD-EE-FF").unwrap().0,
            [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]
        );
        assert!(MacAddr::parse("not-a-mac").is_none());
    }

    #[test]
    fn away_uses_relay() {
        assert_eq!(choose_path(true), WakePath::Direct);
        assert_eq!(choose_path(false), WakePath::Relay);
    }
}
