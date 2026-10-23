//! Keyboard keys named in `kh1-kh2.ini` (`hud_toggle_key`), as Windows virtual-key codes.

/// What a key setting asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeySetting {
    /// No key.
    Off,
    /// This virtual-key code.
    Key(u8),
}

/// Page Down (`VK_NEXT`).
pub const PAGE_DOWN: u8 = 0x22;

/// The setting for `v` (lower case, trimmed): `off`, a key name (`pagedown`, `pageup`, `home`,
/// `end`, `insert`, `delete`, `f1`..`f12`, a letter or digit) or a virtual-key code (`0x22`).
/// `None`: not a key this knows.
pub fn parse(v: &str) -> Option<KeySetting> {
    let v = v.trim().to_ascii_lowercase();
    let key = match v.as_str() {
        "off" | "none" | "no" | "false" | "0" => return Some(KeySetting::Off),
        "pagedown" | "page_down" | "page down" | "pgdn" | "pgdown" | "next" => PAGE_DOWN,
        "pageup" | "page_up" | "page up" | "pgup" | "prior" => 0x21,
        "end" => 0x23,
        "home" => 0x24,
        "insert" | "ins" => 0x2D,
        "delete" | "del" => 0x2E,
        "pause" => 0x13,
        "scrolllock" | "scroll_lock" | "scroll lock" => 0x91,
        _ => {
            if let Some(h) = v.strip_prefix("0x") {
                return u8::from_str_radix(h, 16).ok().filter(|&k| k != 0 && k != 0xFF).map(KeySetting::Key);
            }
            if let Some(n) = v.strip_prefix('f').and_then(|n| n.parse::<u8>().ok()) {
                return (1..=12).contains(&n).then_some(KeySetting::Key(0x6F + n));
            }
            let mut c = v.chars();
            match (c.next(), c.next()) {
                (Some(ch), None) if ch.is_ascii_alphanumeric() => ch.to_ascii_uppercase() as u8,
                _ => return None,
            }
        }
    };
    Some(KeySetting::Key(key))
}

/// The key's name for the log.
pub fn name(vk: u8) -> String {
    match vk {
        0x21 => "Page Up".into(),
        PAGE_DOWN => "Page Down".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x13 => "Pause".into(),
        0x91 => "Scroll Lock".into(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        b'0'..=b'9' | b'A'..=b'Z' => (vk as char).to_string(),
        _ => format!("key {vk:#04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_down_and_off() {
        assert_eq!(parse("pagedown"), Some(KeySetting::Key(PAGE_DOWN)));
        assert_eq!(parse(" PgDn "), Some(KeySetting::Key(PAGE_DOWN)));
        assert_eq!(parse("off"), Some(KeySetting::Off));
        assert_eq!(parse("none"), Some(KeySetting::Off));
    }

    #[test]
    fn other_keys() {
        assert_eq!(parse("pageup"), Some(KeySetting::Key(0x21)));
        assert_eq!(parse("f1"), Some(KeySetting::Key(0x70)));
        assert_eq!(parse("F12"), Some(KeySetting::Key(0x7B)));
        assert_eq!(parse("h"), Some(KeySetting::Key(b'H')));
        assert_eq!(parse("7"), Some(KeySetting::Key(b'7')));
        assert_eq!(parse("0x22"), Some(KeySetting::Key(0x22)));
    }

    #[test]
    fn unknown() {
        assert_eq!(parse("f13"), None);
        assert_eq!(parse("0x0"), None);
        assert_eq!(parse("pagedownn"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn names() {
        assert_eq!(name(PAGE_DOWN), "Page Down");
        assert_eq!(name(0x70), "F1");
        assert_eq!(name(b'H'), "H");
    }
}
