//! Keyboard shortcuts for jumping between devices (R-INPUT-5).
//!
//! A shortcut is written as `ctrl+alt+shift+esc`. Keys are Windows virtual-key
//! codes, because that is what the Windows hooks report.

use std::collections::BTreeSet;

pub const CTRL: [u32; 3] = [0x11, 0xA2, 0xA3];
pub const ALT: [u32; 3] = [0x12, 0xA4, 0xA5];
pub const SHIFT: [u32; 3] = [0x10, 0xA0, 0xA1];
pub const WIN: [u32; 2] = [0x5B, 0x5C];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u32,
}

/// Which modifiers are down, out of a set of held virtual-key codes.
pub fn modifiers_held(held: &BTreeSet<u32>) -> (bool, bool, bool, bool) {
    let any = |group: &[u32]| group.iter().any(|vk| held.contains(vk));
    (any(&CTRL), any(&ALT), any(&SHIFT), any(&WIN))
}

pub fn is_modifier(vk: u32) -> bool {
    CTRL.contains(&vk) || ALT.contains(&vk) || SHIFT.contains(&vk) || WIN.contains(&vk)
}

impl Hotkey {
    /// `ctrl+alt+shift+esc`, `win+f1`, `ctrl+shift+k`. Exactly one non-modifier
    /// key is required. Returns `None` for anything else.
    pub fn parse(text: &str) -> Option<Hotkey> {
        let mut hotkey = Hotkey {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            vk: 0,
        };
        for part in text.split('+') {
            match part.trim().to_ascii_lowercase().as_str() {
                "" => return None,
                "ctrl" | "control" => hotkey.ctrl = true,
                "alt" => hotkey.alt = true,
                "shift" => hotkey.shift = true,
                "win" | "super" | "meta" | "cmd" => hotkey.win = true,
                key => {
                    if hotkey.vk != 0 {
                        return None;
                    }
                    hotkey.vk = key_code(key)?;
                }
            }
        }
        (hotkey.vk != 0).then_some(hotkey)
    }

    /// True when `vk` was just pressed and exactly this set of modifiers is
    /// held. `held` is every key that is down, including `vk` itself.
    pub fn matches(&self, vk: u32, held: &BTreeSet<u32>) -> bool {
        if vk != self.vk {
            return false;
        }
        modifiers_held(held) == (self.ctrl, self.alt, self.shift, self.win)
    }
}

fn key_code(name: &str) -> Option<u32> {
    let bytes = name.as_bytes();
    if bytes.len() == 1 {
        let c = bytes[0];
        return match c {
            b'a'..=b'z' => Some(0x41 + (c - b'a') as u32),
            b'0'..=b'9' => Some(0x30 + (c - b'0') as u32),
            _ => None,
        };
    }
    if let Some(number) = name.strip_prefix('f') {
        if let Ok(n) = number.parse::<u32>() {
            if (1..=24).contains(&n) {
                return Some(0x70 + n - 1);
            }
        }
    }
    Some(match name {
        "esc" | "escape" => 0x1B,
        "space" => 0x20,
        "enter" | "return" => 0x0D,
        "tab" => 0x09,
        "backspace" => 0x08,
        "pause" => 0x13,
        "scrolllock" => 0x91,
        "insert" => 0x2D,
        "delete" | "del" => 0x2E,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(vks: &[u32]) -> BTreeSet<u32> {
        vks.iter().copied().collect()
    }

    #[test]
    fn parses_modifiers_and_keys() {
        let k = Hotkey::parse("Ctrl+Alt+Shift+Esc").unwrap();
        assert_eq!((k.ctrl, k.alt, k.shift, k.win, k.vk), (true, true, true, false, 0x1B));
        let k = Hotkey::parse(" win + F1 ").unwrap();
        assert_eq!((k.win, k.vk), (true, 0x70));
        assert_eq!(Hotkey::parse("ctrl+k").unwrap().vk, 0x4B);
        assert_eq!(Hotkey::parse("alt+7").unwrap().vk, 0x37);
        assert_eq!(Hotkey::parse("f12").unwrap().vk, 0x7B);
    }

    #[test]
    fn rejects_bad_shortcuts() {
        assert_eq!(Hotkey::parse(""), None);
        assert_eq!(Hotkey::parse("ctrl+alt"), None);
        assert_eq!(Hotkey::parse("ctrl+a+b"), None);
        assert_eq!(Hotkey::parse("ctrl++a"), None);
        assert_eq!(Hotkey::parse("ctrl+nonsense"), None);
        assert_eq!(Hotkey::parse("f25"), None);
    }

    #[test]
    fn matches_only_with_exactly_those_modifiers() {
        let k = Hotkey::parse("ctrl+alt+f1").unwrap();
        assert!(k.matches(0x70, &held(&[0xA2, 0xA4, 0x70])));
        assert!(k.matches(0x70, &held(&[0xA3, 0x12, 0x70])));
        assert!(!k.matches(0x70, &held(&[0xA2, 0x70])));
        assert!(!k.matches(0x70, &held(&[0xA2, 0xA4, 0xA0, 0x70])));
        assert!(!k.matches(0x71, &held(&[0xA2, 0xA4, 0x71])));
    }

    #[test]
    fn modifiers_are_recognised() {
        assert!(is_modifier(0xA2) && is_modifier(0x5B) && !is_modifier(0x41));
    }
}
