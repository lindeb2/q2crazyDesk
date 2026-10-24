//! Per-pair key maps and Swedish layout translation (R-KEYS).
//!
//! `@ { } [ ] | \ ~ $ €` are AltGr+digit on Swedish Windows and Option+digit
//! (sometimes with Shift) on a Swedish Mac. Modifier remapping alone cannot
//! produce the right character; this crate maps the produced character and
//! then the default Mac↔Windows modifiers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OsFamily {
    Windows,
    Macos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Key {
    Digit(u8),
    Letter(char),
    /// Key left of Backspace on a Swedish board (`¨` / `^` / `~`).
    Diaeresis,
    /// Key left of Return (`'` / `*`).
    Apostrophe,
    /// Key left of Z (`<` / `>` / `|`).
    LtGt,
    /// `+` / `?` / `\`.
    Plus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
    pub altgr: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Combo {
    pub key: Key,
    pub mods: Mods,
}

impl Combo {
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            mods: Mods {
                ctrl: false,
                alt: false,
                shift: false,
                meta: false,
                altgr: false,
            },
        }
    }

    pub const fn with_shift(mut self) -> Self {
        self.mods.shift = true;
        self
    }

    pub const fn with_altgr(mut self) -> Self {
        self.mods.altgr = true;
        self
    }

    pub const fn with_option(mut self) -> Self {
        self.mods.alt = true;
        self
    }

    pub const fn with_option_shift(mut self) -> Self {
        self.mods.alt = true;
        self.mods.shift = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub controller: OsFamily,
    pub target: OsFamily,
}

impl Mapping {
    pub fn default_for(controller: OsFamily, target: OsFamily) -> Self {
        Self {
            controller,
            target,
        }
    }

    /// Translate a physical combo on the controller into the combo that should
    /// be injected on the target so the same character (or the same shortcut)
    /// appears.
    pub fn translate(&self, input: Combo) -> Combo {
        if let Some(ch) = swedish_char(self.controller, input) {
            if let Some(out) = combo_for_swedish_char(self.target, ch) {
                return out;
            }
        }
        remap_modifiers(self.controller, self.target, input)
    }
}

pub fn remap_modifiers(controller: OsFamily, target: OsFamily, input: Combo) -> Combo {
    if controller == target {
        return input;
    }
    let mut m = input.mods;
    match (controller, target) {
        (OsFamily::Macos, OsFamily::Windows) => {
            // Cmd → Ctrl, Control → Win, Option → Alt.
            let cmd = m.meta;
            let control = m.ctrl;
            let option = m.alt && !m.altgr;
            m.meta = control;
            m.ctrl = cmd;
            m.alt = option;
        }
        (OsFamily::Windows, OsFamily::Macos) => {
            // Ctrl → Cmd, Win → Control, Alt → Option.
            let ctrl = m.ctrl;
            let win = m.meta;
            let alt = m.alt && !m.altgr;
            m.meta = ctrl;
            m.ctrl = win;
            m.alt = alt;
            m.altgr = false;
        }
        _ => {}
    }
    Combo {
        key: input.key,
        mods: m,
    }
}

/// Character produced by a combo on a Swedish layout. Dead-key sequences are
/// represented by their result (`¨`+`o` is a later layer; `¨` alone is the
/// diaeresis character when followed by space, encoded here as the lone press
/// of the dead key with no letter).
pub fn swedish_char(os: OsFamily, combo: Combo) -> Option<char> {
    match os {
        OsFamily::Windows => windows_se(combo),
        OsFamily::Macos => macos_se(combo),
    }
}

pub fn combo_for_swedish_char(os: OsFamily, ch: char) -> Option<Combo> {
    SWEDISH.iter().find_map(|(c, win, mac)| {
        if *c == ch {
            Some(match os {
                OsFamily::Windows => *win,
                OsFamily::Macos => *mac,
            })
        } else {
            None
        }
    })
}

const fn d(n: u8) -> Key {
    Key::Digit(n)
}

const fn letter(c: char) -> Key {
    Key::Letter(c)
}

fn windows_se(c: Combo) -> Option<char> {
    SWEDISH
        .iter()
        .find(|(_, win, _)| *win == c)
        .map(|(ch, _, _)| *ch)
}

fn macos_se(c: Combo) -> Option<char> {
    SWEDISH
        .iter()
        .find(|(_, _, mac)| *mac == c)
        .map(|(ch, _, _)| *ch)
}

const fn win_altgr(key: Key) -> Combo {
    Combo::new(key).with_altgr()
}

const fn mac_opt(key: Key) -> Combo {
    Combo::new(key).with_option()
}

const fn mac_opt_shift(key: Key) -> Combo {
    Combo::new(key).with_option_shift()
}

/// Source combos: Swedish Windows vs Swedish Mac for the characters in R-KEYS-3.
const SWEDISH: &[(char, Combo, Combo)] = &[
    ('å', Combo::new(Key::Letter('å')), Combo::new(Key::Letter('å'))),
    ('ä', Combo::new(Key::Letter('ä')), Combo::new(Key::Letter('ä'))),
    ('ö', Combo::new(Key::Letter('ö')), Combo::new(Key::Letter('ö'))),
    (
        'Å',
        Combo::new(Key::Letter('å')).with_shift(),
        Combo::new(Key::Letter('å')).with_shift(),
    ),
    (
        'Ä',
        Combo::new(Key::Letter('ä')).with_shift(),
        Combo::new(Key::Letter('ä')).with_shift(),
    ),
    (
        'Ö',
        Combo::new(Key::Letter('ö')).with_shift(),
        Combo::new(Key::Letter('ö')).with_shift(),
    ),
    ('@', win_altgr(d(2)), mac_opt(d(2))),
    ('£', win_altgr(d(3)), mac_opt(d(3))),
    ('$', win_altgr(d(4)), mac_opt(d(4))),
    ('€', win_altgr(letter('e')), mac_opt_shift(d(2))),
    ('{', win_altgr(d(7)), mac_opt_shift(d(8))),
    ('[', win_altgr(d(8)), mac_opt(d(8))),
    (']', win_altgr(d(9)), mac_opt(d(9))),
    ('}', win_altgr(d(0)), mac_opt_shift(d(9))),
    ('\\', win_altgr(Key::Plus), mac_opt_shift(d(7))),
    ('|', win_altgr(Key::LtGt), mac_opt(d(7))),
    ('~', win_altgr(Key::Diaeresis), mac_opt(Key::Diaeresis)),
    (
        '^',
        Combo::new(Key::Diaeresis).with_shift(),
        mac_opt_shift(Key::Diaeresis),
    ),
    ('´', Combo::new(Key::Apostrophe), Combo::new(Key::Apostrophe)),
    ('¨', Combo::new(Key::Diaeresis), Combo::new(Key::Diaeresis)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn map(a: OsFamily, b: OsFamily) -> Mapping {
        Mapping::default_for(a, b)
    }

    #[test]
    fn mac_cmd_c_becomes_ctrl_c_on_windows() {
        let mut combo = Combo::new(letter('c'));
        combo.mods.meta = true;
        let out = map(OsFamily::Macos, OsFamily::Windows).translate(combo);
        assert!(out.mods.ctrl);
        assert!(!out.mods.meta);
        assert_eq!(out.key, letter('c'));
    }

    #[test]
    fn win_ctrl_c_becomes_cmd_c_on_mac() {
        let mut combo = Combo::new(letter('c'));
        combo.mods.ctrl = true;
        let out = map(OsFamily::Windows, OsFamily::Macos).translate(combo);
        assert!(out.mods.meta);
        assert!(!out.mods.ctrl);
    }

    #[test]
    fn swedish_specials_every_direction() {
        let chars = [
            'å', 'ä', 'ö', 'Å', 'Ä', 'Ö', '@', '£', '$', '€', '{', '}', '[', ']', '|', '\\', '~',
            '^', '´', '¨',
        ];
        for os in [OsFamily::Windows, OsFamily::Macos] {
            for ch in chars {
                let combo = combo_for_swedish_char(os, ch).expect("combo");
                assert_eq!(swedish_char(os, combo), Some(ch), "{ch} on {os:?}");
            }
        }
        let mac_at = combo_for_swedish_char(OsFamily::Macos, '@').unwrap();
        let win = map(OsFamily::Macos, OsFamily::Windows).translate(mac_at);
        assert_eq!(swedish_char(OsFamily::Windows, win), Some('@'));

        let win_at = combo_for_swedish_char(OsFamily::Windows, '@').unwrap();
        let mac = map(OsFamily::Windows, OsFamily::Macos).translate(win_at);
        assert_eq!(swedish_char(OsFamily::Macos, mac), Some('@'));
    }

    #[test]
    fn same_os_passthrough() {
        let combo = Combo::new(letter('a')).with_shift();
        assert_eq!(
            map(OsFamily::Windows, OsFamily::Windows).translate(combo),
            combo
        );
    }
}
