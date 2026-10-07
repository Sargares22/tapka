//! Hotkey strings like `win+shift+s` → Windows virtual-key codes in the order they are pressed.

pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_ALT: u16 = 0x12;
pub const VK_LWIN: u16 = 0x5B;

fn modifier(name: &str) -> Option<u16> {
    Some(match name {
        "ctrl" | "control" => VK_CONTROL,
        "shift" => VK_SHIFT,
        "alt" => VK_ALT,
        "win" => VK_LWIN,
        _ => return None,
    })
}

fn key(name: &str) -> Option<u16> {
    let b = name.as_bytes();
    if b.len() == 1 && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit()) {
        return Some(b[0].to_ascii_uppercase() as u16); // VK codes of letters and digits are their ASCII
    }
    if let Some(n) = name.strip_prefix('f').and_then(|n| n.parse::<u16>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x70 + n - 1);
        }
    }
    Some(match name {
        "space" => 0x20,
        "enter" => 0x0D,
        "tab" => 0x09,
        "esc" | "escape" => 0x1B,
        "backspace" => 0x08,
        "delete" | "del" => 0x2E,
        "insert" => 0x2D,
        "home" => 0x24,
        "end" => 0x23,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "printscreen" => 0x2C,
        "." | "period" => 0xBE,
        "," | "comma" => 0xBC,
        "volumemute" | "mute" => 0xAD,
        "volumedown" => 0xAE,
        "volumeup" => 0xAF,
        "nexttrack" => 0xB0,
        "prevtrack" => 0xB1,
        "playpause" => 0xB3,
        _ => return None,
    })
}

/// The key a `hold` item keeps pressed: Shift, Ctrl, Alt or Space, in any case.
pub fn holdable(name: &str) -> Option<u16> {
    Some(match name.trim().to_lowercase().as_str() {
        "shift" => VK_SHIFT,
        "ctrl" | "control" => VK_CONTROL,
        "alt" => VK_ALT,
        "space" => 0x20,
        _ => return None,
    })
}

/// Parses a hotkey: any number of modifiers and exactly one key, joined with `+`, in any case.
/// Returns the codes with modifiers first (in the order written) and the key last.
pub fn parse(text: &str) -> Result<Vec<u16>, String> {
    let mut mods = Vec::new();
    let mut main = None;
    for part in text.split('+') {
        let part = part.trim().to_lowercase();
        if let Some(m) = modifier(&part) {
            if !mods.contains(&m) {
                mods.push(m);
            }
        } else if let Some(k) = key(&part) {
            if main.replace(k).is_some() {
                return Err(format!("hotkey \"{text}\": more than one key"));
            }
        } else {
            return Err(format!("hotkey \"{text}\": unknown key \"{part}\""));
        }
    }
    let Some(main) = main else { return Err(format!("hotkey \"{text}\": no key besides modifiers")) };
    mods.push(main);
    Ok(mods)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_come_first_and_the_key_last() {
        assert_eq!(parse("win+shift+s").unwrap(), [VK_LWIN, VK_SHIFT, 0x53]);
        assert_eq!(parse("ctrl+space").unwrap(), [VK_CONTROL, 0x20]);
        assert_eq!(parse("s+shift+win").unwrap(), [VK_SHIFT, VK_LWIN, 0x53]);
    }

    #[test]
    fn case_spaces_and_repeats_do_not_matter() {
        assert_eq!(parse(" Ctrl + Alt + Delete ").unwrap(), [VK_CONTROL, VK_ALT, 0x2E]);
        assert_eq!(parse("ctrl+control+a").unwrap(), [VK_CONTROL, 0x41]);
    }

    #[test]
    fn single_keys_function_keys_and_digits() {
        assert_eq!(parse("f1").unwrap(), [0x70]);
        assert_eq!(parse("alt+f24").unwrap(), [VK_ALT, 0x87]);
        assert_eq!(parse("win+1").unwrap(), [VK_LWIN, 0x31]);
        assert_eq!(parse("printscreen").unwrap(), [0x2C]);
    }

    #[test]
    fn media_keys_and_punctuation() {
        assert_eq!(parse("volumeup").unwrap(), [0xAF]);
        assert_eq!(parse("volumedown").unwrap(), [0xAE]);
        assert_eq!(parse("volumemute").unwrap(), parse("mute").unwrap());
        assert_eq!(parse("playpause").unwrap(), [0xB3]);
        assert_eq!(parse("nexttrack").unwrap(), [0xB0]);
        assert_eq!(parse("prevtrack").unwrap(), [0xB1]);
        assert_eq!(parse("win+.").unwrap(), [VK_LWIN, 0xBE]);
        assert_eq!(parse("ctrl+period").unwrap(), [VK_CONTROL, 0xBE]);
        assert_eq!(parse("alt+,").unwrap(), parse("alt+comma").unwrap());
    }

    #[test]
    fn only_three_modifiers_and_space_can_be_held() {
        assert_eq!(holdable("shift"), Some(VK_SHIFT));
        assert_eq!(holdable(" Ctrl "), Some(VK_CONTROL));
        assert_eq!(holdable("alt"), Some(VK_ALT));
        assert_eq!(holdable("space"), Some(0x20));
        for other in ["win", "a", "ctrl+shift", "", "enter"] {
            assert_eq!(holdable(other), None, "{other}");
        }
    }

    #[test]
    fn broken_hotkeys_are_rejected() {
        assert!(parse("ctrl+shift").unwrap_err().contains("no key"));
        assert!(parse("ctrl+a+b").unwrap_err().contains("more than one key"));
        assert!(parse("ctrl+").unwrap_err().contains("unknown key"));
        assert!(parse("ctrl+f25").unwrap_err().contains("unknown key"));
        assert!(parse("super+s").unwrap_err().contains("unknown key \"super\""));
    }
}
