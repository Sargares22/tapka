//! The settings file: one JSON in the app data folder. The panel reads it, the settings window
//! writes it, and a person or an agent may edit it by hand; fields the panel does not know are
//! kept as they are.

use crate::layout::Edge;
use serde_json::{json, Value};
use std::path::Path;

pub const DEFAULT_SCALE: f64 = 1.0;
pub const DEFAULT_TOP: f64 = 0.3;
pub const DEFAULT_ACCENT: &str = "#8ab4ff";

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Open(String),
    Hotkey(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub name: String,
    pub icon: Option<String>,
    /// One line under the name in the card.
    pub hint: Option<String>,
    /// Lights the item up while another program shows a window: `(program file name in lower
    /// case, window title)`. In the file: `"lit": { "program": "recorder.exe", "window": "Recording" }`.
    pub lit: Option<(String, String)>,
    pub action: Action,
}

/// `"theme"` in the file: `"dark"`, `"light"`, or anything else for "as Windows has it".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    System,
    Dark,
    Light,
}

/// `"lang"` in the file: `"ru"`, `"en"`, or anything else for the language of Windows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    System,
    Ru,
    En,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub scale: f64,
    pub top: f64,
    /// The screen edge the capsule is docked to. `top` is where it starts along that edge.
    pub edge: Edge,
    pub theme: Theme,
    /// The accent colour as `#rrggbb`.
    pub accent: String,
    pub lang: Lang,
    /// Ask GitHub for a newer release once per start.
    pub updates: bool,
    pub items: Vec<Item>,
}

impl Settings {
    /// What the capsule shows when the file is invalid at start: defaults and no items.
    pub fn empty() -> Self {
        Settings {
            scale: DEFAULT_SCALE,
            top: DEFAULT_TOP,
            edge: Edge::Right,
            theme: Theme::System,
            accent: DEFAULT_ACCENT.into(),
            lang: Lang::System,
            updates: true,
            items: Vec::new(),
        }
    }
}

/// A ready action of Windows: works on any Windows 11 with nothing installed. Each was pressed
/// on a real machine before it got here; `cargo test live -- --ignored` repeats
/// the check for the ones that open something.
pub struct Preset {
    pub id: &'static str,
    /// A built-in icon.
    pub icon: &'static str,
    /// `hotkey` or `open`, and its keys or target.
    pub action: (&'static str, &'static str),
    /// Name and hint, in Russian and in English.
    pub ru: (&'static str, &'static str),
    pub en: (&'static str, &'static str),
}

pub const PRESETS: [Preset; 10] = [
    Preset { id: "snip", icon: "snip", action: ("hotkey", "win+shift+s"), ru: ("Снимок", "Выделить область экрана, снимок уйдёт в буфер"), en: ("Snip", "Select a screen area; the shot goes to the clipboard") },
    Preset { id: "paste", icon: "paste", action: ("hotkey", "ctrl+v"), ru: ("Вставить", "Вставить из буфера в активное окно"), en: ("Paste", "Paste the clipboard into the active window") },
    Preset { id: "copy", icon: "copy", action: ("hotkey", "ctrl+c"), ru: ("Копировать", "Скопировать выделенное в буфер"), en: ("Copy", "Copy the selection to the clipboard") },
    Preset { id: "undo", icon: "undo", action: ("hotkey", "ctrl+z"), ru: ("Отменить", "Отменить последнее действие в активном окне"), en: ("Undo", "Undo the last action in the active window") },
    Preset { id: "voice", icon: "mic", action: ("hotkey", "win+h"), ru: ("Голосовой ввод", "Диктовка Windows в поле, где стоит курсор"), en: ("Voice typing", "Windows dictation into the field with the cursor") },
    Preset { id: "tasks", icon: "tasks", action: ("hotkey", "win+tab"), ru: ("Представление задач", "Открытые окна и рабочие столы"), en: ("Task view", "All open windows and desktops") },
    Preset { id: "desktop", icon: "desktop", action: ("hotkey", "win+d"), ru: ("Рабочий стол", "Свернуть все окна или вернуть их"), en: ("Desktop", "Minimize all windows or bring them back") },
    Preset { id: "explorer", icon: "folder", action: ("hotkey", "win+e"), ru: ("Проводник", "Открыть окно Проводника"), en: ("File Explorer", "Open a File Explorer window") },
    Preset { id: "settings", icon: "gear", action: ("open", "ms-settings:"), ru: ("Параметры", "Открыть Параметры Windows"), en: ("Settings", "Open Windows Settings") },
    Preset { id: "keyboard", icon: "keyboard", action: ("hotkey", "win+ctrl+o"), ru: ("Клавиатура", "Показать или убрать экранную клавиатуру"), en: ("Keyboard", "Show or hide the on-screen keyboard") },
];

/// The ready actions a new settings file starts with, in this order.
pub const DEFAULT_ITEMS: [&str; 5] = ["snip", "paste", "voice", "explorer", "settings"];

/// The item of the settings file a ready action makes, named in Russian or in English.
pub fn preset_item(preset: &Preset, russian: bool) -> Value {
    let (name, hint) = if russian { preset.ru } else { preset.en };
    let (kind, value) = preset.action;
    let field = if kind == "hotkey" { "keys" } else { "target" };
    json!({ "name": name, "icon": preset.icon, "hint": hint, "action": kind, field: value })
}

/// The file written on first run: ready actions of Windows only, named in the system's language.
pub fn default_text(russian: bool) -> String {
    let items: Vec<Value> = DEFAULT_ITEMS
        .iter()
        .filter_map(|id| PRESETS.iter().find(|p| p.id == *id))
        .map(|p| preset_item(p, russian))
        .collect();
    to_text(&json!({ "scale": DEFAULT_SCALE, "top": DEFAULT_TOP, "items": items }))
}

/// A value on one line, with a space after every brace, colon and comma.
fn inline(v: &Value) -> String {
    match v {
        Value::Object(map) if !map.is_empty() => {
            let fields: Vec<String> = map.iter().map(|(k, v)| format!("{}: {}", Value::from(k.as_str()), inline(v))).collect();
            format!("{{ {} }}", fields.join(", "))
        }
        Value::Array(list) if !list.is_empty() => format!("[{}]", list.iter().map(inline).collect::<Vec<_>>().join(", ")),
        other => other.to_string(),
    }
}

/// The settings as the file's text: one field per line and one item per line with its name
/// first, so the file stays easy to read and to edit by hand.
pub fn to_text(v: &Value) -> String {
    let Some(map) = v.as_object() else { return v.to_string() };
    let fields: Vec<String> = map
        .iter()
        .map(|(key, value)| {
            let text = match value.as_array().filter(|items| key == "items" && !items.is_empty()) {
                Some(items) => format!("[\n{}\n  ]", items.iter().map(|i| format!("    {}", inline(i))).collect::<Vec<_>>().join(",\n")),
                None => inline(value),
            };
            format!("  {}: {text}", Value::from(key.as_str()))
        })
        .collect();
    format!("{{\n{}\n}}\n", fields.join(",\n"))
}

/// The settings text with one top-level field set and everything else kept, fields the panel
/// does not know included. A new field goes before `items`. `None` when the text is not a JSON
/// object.
pub fn with_field(text: &str, key: &str, value: Value) -> Option<String> {
    let mut v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let map = v.as_object_mut()?;
    match (map.contains_key(key), map.keys().position(|k| k == "items")) {
        (false, Some(at)) => {
            map.shift_insert(at, key.into(), value);
        }
        _ => {
            map.insert(key.into(), value);
        }
    }
    Some(to_text(&v))
}

/// A share of the work area as it is written into the file: 0..1, four decimals.
pub fn share(v: f64) -> Value {
    let v = if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
    json!((v * 10_000.0).round() / 10_000.0)
}

pub fn edge_name(edge: Edge) -> &'static str {
    match edge {
        Edge::Right => "right",
        Edge::Left => "left",
        Edge::Top => "top",
    }
}

pub fn theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::System => "system",
        Theme::Dark => "dark",
        Theme::Light => "light",
    }
}

pub fn lang_name(lang: Lang) -> &'static str {
    match lang {
        Lang::System => "system",
        Lang::Ru => "ru",
        Lang::En => "en",
    }
}

/// An accent colour as the file may carry it: `#rrggbb`, in lower case; `None` for anything else.
pub fn accent(text: &str) -> Option<String> {
    let hex = text.trim().strip_prefix('#')?;
    (hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| format!("#{}", hex.to_lowercase()))
}

/// The panel has three sizes. Any other number becomes the nearest of them.
pub const SCALES: [f64; 3] = [0.8, 1.0, 1.25];

pub fn snap_scale(scale: f64) -> f64 {
    let mut best = DEFAULT_SCALE;
    for s in SCALES {
        if (s - scale).abs() < (best - scale).abs() {
            best = s;
        }
    }
    best
}

fn text_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

/// One item of the file, or why it cannot be used.
pub fn parse_item(v: &Value) -> Result<Item, String> {
    let name = text_field(v, "name").ok_or("no name")?;
    let action = match v.get("action").and_then(Value::as_str) {
        Some("open") => Action::Open(text_field(v, "target").ok_or("empty target")?),
        Some("hotkey") => {
            let keys = text_field(v, "keys").ok_or("empty keys")?;
            crate::keys::parse(&keys)?;
            Action::Hotkey(keys)
        }
        Some(other) => return Err(format!("unknown action \"{other}\"")),
        None => return Err("no action".into()),
    };
    // A `lit` without both parts is left out rather than failing the item
    let lit = v.get("lit").and_then(|l| Some((text_field(l, "program")?.to_lowercase(), text_field(l, "window")?)));
    Ok(Item { name, icon: text_field(v, "icon"), hint: text_field(v, "hint"), lit, action })
}

/// Parses the settings text. `Err` means the whole file is invalid (not JSON, or no `items`).
/// Broken items are skipped; each one adds a line with its 1-based number and the reason.
pub fn parse(text: &str) -> Result<(Settings, Vec<String>), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    let items = v.get("items").and_then(Value::as_array).ok_or("no \"items\" list")?;
    let mut skipped = Vec::new();
    let mut good = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match parse_item(item) {
            Ok(it) => good.push(it),
            Err(why) => skipped.push(format!("item {} skipped: {why}", i + 1)),
        }
    }
    let word = |key: &str| v.get(key).and_then(Value::as_str);
    let settings = Settings {
        scale: snap_scale(v.get("scale").and_then(Value::as_f64).unwrap_or(DEFAULT_SCALE)),
        top: v.get("top").and_then(Value::as_f64).unwrap_or(DEFAULT_TOP),
        edge: match word("edge") {
            Some("left") => Edge::Left,
            Some("top") => Edge::Top,
            _ => Edge::Right,
        },
        theme: match word("theme") {
            Some("dark") => Theme::Dark,
            Some("light") => Theme::Light,
            _ => Theme::System,
        },
        accent: word("accent").and_then(accent).unwrap_or_else(|| DEFAULT_ACCENT.into()),
        lang: match word("lang") {
            Some("ru") => Lang::Ru,
            Some("en") => Lang::En,
            _ => Lang::System,
        },
        updates: v.get("updates").and_then(Value::as_bool).unwrap_or(true),
        items: good,
    };
    Ok((settings, skipped))
}

/// Reads the settings file, writing the default one first, in Russian or in English, if the file
/// does not exist. The last value says whether the file was created just now.
pub fn load_or_create(path: &Path, russian: bool) -> Result<(Settings, Vec<String>, bool), String> {
    let created = !path.exists();
    if created {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create folder: {e}"))?;
        }
        std::fs::write(path, default_text(russian)).map_err(|e| format!("cannot write settings: {e}"))?;
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read settings: {e}"))?;
    // Notepad may save with a byte-order mark
    let (settings, skipped) = parse(text.trim_start_matches('\u{feff}'))?;
    Ok((settings, skipped, created))
}

/// The name of version 0.1.0's data folder, which stands next to the current one.
pub const OLD_DATA_DIR: &str = "local.surfacepanel";

/// Moving up from 0.1.0: when the data folder has no settings file yet and the old folder has
/// one, the settings file and the icons are copied over. The old folder is left as it was.
/// Returns the names of the copied files, for the log.
pub fn migrate(old: &Path, new: &Path) -> std::io::Result<Vec<String>> {
    let mut copied = Vec::new();
    if new.join("settings.json").exists() || !old.join("settings.json").exists() {
        return Ok(copied);
    }
    std::fs::create_dir_all(new.join("icons"))?;
    if let Ok(icons) = std::fs::read_dir(old.join("icons")) {
        for icon in icons.flatten().filter(|e| e.path().is_file()) {
            std::fs::copy(icon.path(), new.join("icons").join(icon.file_name()))?;
            copied.push(format!("icons/{}", icon.file_name().to_string_lossy()));
        }
    }
    // The settings file goes last: its presence is what marks the move as done
    std::fs::copy(old.join("settings.json"), new.join("settings.json"))?;
    copied.push("settings.json".into());
    Ok(copied)
}

/// Copies a picture into the icons folder and returns the file name it got there. A file of the
/// same name with other contents is never replaced: the new one gets `-2`, `-3` and so on. The
/// same picture brought in again is not copied twice.
pub fn keep_icon(from: &Path, icons: &Path) -> std::io::Result<String> {
    let name = from.file_name().and_then(|n| n.to_str()).ok_or(std::io::ErrorKind::InvalidInput)?;
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    use std::io::Write;
    let bytes = std::fs::read(from)?;
    std::fs::create_dir_all(icons)?;
    for n in 1.. {
        let candidate = if n == 1 { name.to_string() } else { format!("{stem}-{n}.{ext}") };
        // The file is made only if the name is free at this very moment, so two imports at once
        // cannot both take it
        match std::fs::OpenOptions::new().write(true).create_new(true).open(icons.join(&candidate)) {
            Ok(mut file) => {
                file.write_all(&bytes)?;
                return Ok(candidate);
            }
            Err(taken) if taken.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::read(icons.join(&candidate))? == bytes {
                    return Ok(candidate);
                }
            }
            Err(other) => return Err(other),
        }
    }
    unreachable!()
}

/// Re-reads the settings text over the current settings. An invalid file changes nothing and
/// comes back as the error; a valid one replaces the settings and lists its skipped items.
pub fn reload(current: &Settings, text: &str) -> (Settings, Vec<String>, Option<String>) {
    match parse(text.trim_start_matches('\u{feff}')) {
        Ok((settings, skipped)) => (settings, skipped, None),
        Err(why) => (current.clone(), Vec::new(), Some(why)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file as its owner might have it: an own field, an item with `lit`, an item of a kind
    /// this version does not know.
    const OWN: &str = r#"{
  "edge": "right",
  "scale": 1.0,
  "top": 0.4529,
  "note": "mine",
  "items": [
    { "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space", "lit": { "program": "recorder.exe", "window": "Recording" } },
    { "name": "Later", "action": "script", "run": ["a", "b"] }
  ]
}
"#;

    #[test]
    fn the_file_text_comes_back_unchanged_from_a_round_trip() {
        let v: Value = serde_json::from_str(OWN).unwrap();
        assert_eq!(to_text(&v), OWN);
        assert_eq!(to_text(&json!({ "items": [] })), "{\n  \"items\": []\n}\n");
    }

    #[test]
    fn a_field_is_set_and_everything_else_is_kept() {
        // An existing field keeps its place
        let out = with_field(OWN, "top", share(0.7)).unwrap();
        assert_eq!(out, OWN.replace("0.4529", "0.7"));
        // A new one goes before the items; what the panel does not know stays
        let out = with_field(OWN, "theme", json!("light")).unwrap();
        assert!(out.contains("  \"note\": \"mine\",\n  \"theme\": \"light\",\n  \"items\": ["), "{out}");
        assert!(out.contains(r#""lit": { "program": "recorder.exe", "window": "Recording" }"#));
        assert!(out.contains(r#"{ "name": "Later", "action": "script", "run": ["a", "b"] }"#));
        assert_eq!(parse(&out).unwrap().0.theme, Theme::Light);
        // The items are replaced as a whole and the rest stays
        let out = with_field(OWN, "items", json!([{ "name": "a", "action": "open", "target": "x" }])).unwrap();
        assert_eq!(out, "{\n  \"edge\": \"right\",\n  \"scale\": 1.0,\n  \"top\": 0.4529,\n  \"note\": \"mine\",\n  \"items\": [\n    { \"name\": \"a\", \"action\": \"open\", \"target\": \"x\" }\n  ]\n}\n");
        // A file saved with a byte-order mark, a file without the field, and not a file at all
        assert_eq!(parse(&with_field("\u{feff}{\"items\":[]}", "top", share(0.5)).unwrap()).unwrap().0.top, 0.5);
        assert!(with_field("not json", "top", share(0.5)).is_none());
        assert!(with_field("[]", "top", share(0.5)).is_none());
    }

    #[test]
    fn shares_are_kept_inside_the_work_area_with_four_decimals() {
        assert_eq!(share(0.333333), json!(0.3333));
        assert_eq!(share(7.0), json!(1.0));
        assert_eq!(share(-1.0), json!(0.0));
        assert_eq!(share(f64::NAN), json!(0.0));
    }

    #[test]
    fn theme_accent_language_and_updates_come_from_the_file() {
        let (s, _) = parse(r#"{"items":[]}"#).unwrap();
        assert_eq!((s.theme, s.accent.as_str(), s.lang, s.updates), (Theme::System, DEFAULT_ACCENT, Lang::System, true));
        let (s, _) = parse(r##"{"theme":"light","accent":"#FF8800","lang":"en","updates":false,"items":[]}"##).unwrap();
        assert_eq!((s.theme, s.accent.as_str(), s.lang, s.updates), (Theme::Light, "#ff8800", Lang::En, false));
        // Anything odd falls back to the default
        let (s, _) = parse(r#"{"theme":"pink","accent":"red","lang":"de","updates":"no","items":[]}"#).unwrap();
        assert_eq!((s.theme, s.accent.as_str(), s.lang, s.updates), (Theme::System, DEFAULT_ACCENT, Lang::System, true));
        assert_eq!(accent(" #AbCdEf "), Some("#abcdef".into()));
        for bad in ["abcdef", "#abc", "#abcdeg", "#abcdef0", ""] {
            assert_eq!(accent(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_imported_icon_never_replaces_another_picture() {
        let dir = std::env::temp_dir().join(format!("tapka-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (a, b, icons) = (dir.join("a"), dir.join("b"), dir.join("icons"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("icon.png"), [1]).unwrap();
        std::fs::write(b.join("icon.png"), [2]).unwrap();
        assert_eq!(keep_icon(&a.join("icon.png"), &icons).unwrap(), "icon.png");
        // Another picture under the same name gets a name of its own; the first one stays
        assert_eq!(keep_icon(&b.join("icon.png"), &icons).unwrap(), "icon-2.png");
        assert_eq!(std::fs::read(icons.join("icon.png")).unwrap(), [1]);
        assert_eq!(std::fs::read(icons.join("icon-2.png")).unwrap(), [2]);
        // The same pictures again: no third file
        assert_eq!(keep_icon(&a.join("icon.png"), &icons).unwrap(), "icon.png");
        assert_eq!(keep_icon(&b.join("icon.png"), &icons).unwrap(), "icon-2.png");
        assert_eq!(std::fs::read_dir(&icons).unwrap().count(), 2);
        // A picture already in the icons folder keeps its name
        assert_eq!(keep_icon(&icons.join("icon-2.png"), &icons).unwrap(), "icon-2.png");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrate_copies_settings_and_icons_once() {
        let dir = std::env::temp_dir().join(format!("tapka-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (old, new) = (dir.join(OLD_DATA_DIR), dir.join("tapka"));
        // No old folder: nothing happens and the new folder is not created
        assert!(migrate(&old, &new).unwrap().is_empty());
        assert!(!new.exists());
        std::fs::create_dir_all(old.join("icons")).unwrap();
        std::fs::write(old.join("settings.json"), r#"{"top":0.4,"items":[]}"#).unwrap();
        std::fs::write(old.join("icons").join("own.png"), [1, 2, 3]).unwrap();
        std::fs::write(old.join("panel.log"), "old log").unwrap();
        let mut copied = migrate(&old, &new).unwrap();
        copied.sort();
        assert_eq!(copied, ["icons/own.png", "settings.json"]);
        // The copied file is read as it is, not taken for a first run
        let (s, _, created) = load_or_create(&new.join("settings.json"), false).unwrap();
        assert_eq!((s.top, created), (0.4, false));
        assert_eq!(std::fs::read(new.join("icons").join("own.png")).unwrap(), [1, 2, 3]);
        assert!(!new.join("panel.log").exists());
        // The old folder keeps everything
        assert!(old.join("settings.json").exists() && old.join("icons").join("own.png").exists());
        // A settings file already in the new folder is never replaced
        std::fs::write(new.join("settings.json"), r#"{"top":0.9,"items":[]}"#).unwrap();
        assert!(migrate(&old, &new).unwrap().is_empty());
        assert_eq!(load_or_create(&new.join("settings.json"), false).unwrap().0.top, 0.9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_ready_action_makes_a_valid_item_in_both_languages() {
        for russian in [true, false] {
            for preset in &PRESETS {
                let item = parse_item(&preset_item(preset, russian)).unwrap_or_else(|why| panic!("{}: {why}", preset.id));
                assert!(crate::glyphs::builtin(item.icon.as_deref().unwrap()).is_some(), "{}", preset.id);
                assert!(item.hint.is_some(), "{}", preset.id);
            }
        }
        let ids: Vec<_> = PRESETS.iter().map(|p| p.id).collect();
        assert!(DEFAULT_ITEMS.iter().all(|id| ids.contains(id)));
    }

    #[test]
    fn the_default_file_has_windows_actions_in_the_language_asked_for() {
        let names = |russian| -> Vec<String> { parse(&default_text(russian)).unwrap().0.items.into_iter().map(|i| i.name).collect() };
        assert_eq!(names(true), ["Снимок", "Вставить", "Голосовой ввод", "Проводник", "Параметры"]);
        assert_eq!(names(false), ["Snip", "Paste", "Voice typing", "File Explorer", "Settings"]);
        let text = default_text(false);
        assert!(text.starts_with("{\n  \"scale\": 1.0,\n  \"top\": 0.3,\n  \"items\": [\n    { \"name\": \"Snip\", \"icon\": \"snip\", "), "{text}");
        let (s, skipped) = parse(&text).unwrap();
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(s.top, DEFAULT_TOP);
        assert_eq!(s.items[0].action, Action::Hotkey("win+shift+s".into()));
        assert_eq!(s.items[4].action, Action::Open("ms-settings:".into()));
        // Nothing of any one machine or of another product
        assert!(s.items.iter().all(|i| i.lit.is_none()));
    }
}
