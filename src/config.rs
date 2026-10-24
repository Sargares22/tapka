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
    /// Keeps one key pressed until the next tap on the item: `shift`, `ctrl`, `alt` or `space`.
    Hold(String),
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
    /// The item shows only while the active window belongs to this program: its file name or
    /// its Store package family, in lower case. In the file: `"only_in": "paint.exe"`.
    pub only_in: Option<String>,
    pub action: Action,
}

/// A meter: a small dial of a program's progress, of a number in a file or of an AI limit. In the
/// file: `{ "name": "Render", "program": "C:\\Tools\\render.exe" }`, `{ "name": "Backup", "file": "D:\\backup.txt" }`
/// or `{ "name": "Claude", "limit": "claude" }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Meter {
    pub name: String,
    pub source: Source,
    /// The id a program gives its windows when it gives one of its own, which its taskbar button
    /// then shows instead of the exe. Kept from the program list when the meter is added. In the
    /// file: `"app"`.
    pub app: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// What an `open` item would open: the path of an exe or a `shell:AppsFolder\…` address.
    Program(String),
    /// A text file whose first number is the percent.
    File(String),
    /// The usage limits of an AI service's sign-in on this machine (`limits`).
    Limit(crate::limits::Service),
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

/// A docked panel: the window of another program the user ticked, told by the full path of its
/// program and its window class, which stay the same across restarts where its title does not. In
/// the file: `{ "exe": "panel.exe", "path": "C:\\Tools\\panel.exe", "class": "PanelWindow" }`.
/// An entry of an older version is the file name alone, `"panel.exe"`: any window of a program of
/// that name.
#[derive(Debug, Clone, PartialEq)]
pub struct Docked {
    /// The program's file name, lower case: what is shown, and what an entry without a path goes by.
    pub exe: String,
    pub path: Option<String>,
    pub class: Option<String>,
}

impl Docked {
    /// What tells one entry from another, and what the panel remembers its window by.
    pub fn key(&self) -> String {
        match &self.path {
            Some(path) => format!("{}|{}", path.to_lowercase(), self.class.as_deref().unwrap_or("")),
            None => self.exe.clone(),
        }
    }

    /// Whether a window of the program at `path`, of class `class`, is this entry's.
    pub fn matches(&self, exe: &str, path: &str, class: &str) -> bool {
        match &self.path {
            Some(own) => own.eq_ignore_ascii_case(path) && self.class.as_deref().is_none_or(|c| c == class),
            None => self.exe == exe,
        }
    }
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
    /// The capsule is on screen only in tablet mode: with a keyboard attached it hides itself.
    pub tablet_only: bool,
    /// A quiet click when a tap on a key works. Off unless asked for.
    pub click_sound: bool,
    /// The capsule is folded into one key. Not the same as hidden: a folded capsule is hidden and
    /// shown like an open one, and comes back folded.
    pub folded: bool,
    /// Where the folded key's pill starts along the edge, as a share of the work area, like `top`.
    /// In the file: `"folded_at"`.
    pub folded_at: f64,
    pub items: Vec<Item>,
    pub meters: Vec<Meter>,
    /// The share of the room for keys and meters that the keys take, once the divider between them
    /// was moved. In the file: `"divider"`.
    pub divider: Option<f64>,
    /// How many key cells long the capsule was stretched to by its end grip; none, and it is as
    /// long as its keys, up to four (`layout::keys_length`). In the file: `"length"`.
    pub length: Option<usize>,
    /// The panels of other programs that stand under the capsule and go along with it, in the
    /// order they stand. In the file: `"docked"`.
    pub docked: Vec<Docked>,
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
            tablet_only: false,
            click_sound: false,
            folded: false,
            folded_at: DEFAULT_TOP,
            items: Vec::new(),
            meters: Vec::new(),
            divider: None,
            length: None,
            docked: Vec::new(),
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
    /// `hotkey`, `hold` or `open`, and its keys or target.
    pub action: (&'static str, &'static str),
    /// Name and hint, in Russian and in English.
    pub ru: (&'static str, &'static str),
    pub en: (&'static str, &'static str),
}

pub const PRESETS: [Preset; 30] = [
    Preset { id: "snip", icon: "snip", action: ("hotkey", "win+shift+s"), ru: ("Снимок", "Выделить область экрана, снимок уйдёт в буфер"), en: ("Snip", "Select a screen area; the shot goes to the clipboard") },
    Preset { id: "paste", icon: "paste", action: ("hotkey", "ctrl+v"), ru: ("Вставить", "Вставить из буфера в активное окно"), en: ("Paste", "Paste the clipboard into the active window") },
    Preset { id: "copy", icon: "copy", action: ("hotkey", "ctrl+c"), ru: ("Копировать", "Скопировать выделенное в буфер"), en: ("Copy", "Copy the selection to the clipboard") },
    Preset { id: "undo", icon: "undo", action: ("hotkey", "ctrl+z"), ru: ("Отменить", "Отменить последнее действие в активном окне"), en: ("Undo", "Undo the last action in the active window") },
    Preset { id: "redo", icon: "redo", action: ("hotkey", "ctrl+y"), ru: ("Повторить", "Вернуть отменённое действие в активном окне"), en: ("Redo", "Redo what was undone in the active window") },
    Preset { id: "selectall", icon: "select-all", action: ("hotkey", "ctrl+a"), ru: ("Выделить всё", "Выделить всё в активном окне"), en: ("Select all", "Select everything in the active window") },
    Preset { id: "voice", icon: "mic", action: ("hotkey", "win+h"), ru: ("Голосовой ввод", "Диктовка Windows в поле, где стоит курсор"), en: ("Voice typing", "Windows dictation into the field with the cursor") },
    Preset { id: "tasks", icon: "tasks", action: ("hotkey", "win+tab"), ru: ("Представление задач", "Открытые окна и рабочие столы"), en: ("Task view", "All open windows and desktops") },
    Preset { id: "desktop", icon: "desktop", action: ("hotkey", "win+d"), ru: ("Рабочий стол", "Свернуть все окна или вернуть их"), en: ("Desktop", "Minimize all windows or bring them back") },
    Preset { id: "explorer", icon: "folder", action: ("hotkey", "win+e"), ru: ("Проводник", "Открыть окно Проводника"), en: ("File Explorer", "Open a File Explorer window") },
    Preset { id: "settings", icon: "gear", action: ("open", "ms-settings:"), ru: ("Параметры", "Открыть Параметры Windows"), en: ("Settings", "Open Windows Settings") },
    Preset { id: "keyboard", icon: "keyboard", action: ("hotkey", "win+ctrl+o"), ru: ("Клавиатура", "Показать или убрать экранную клавиатуру"), en: ("Keyboard", "Show or hide the on-screen keyboard") },
    Preset { id: "clipboard", icon: "clipboard", action: ("hotkey", "win+v"), ru: ("История буфера", "Выбрать, что вставить, из недавно скопированного"), en: ("Clipboard history", "Pick what to paste from recently copied items") },
    Preset { id: "emoji", icon: "emoji", action: ("hotkey", "win+."), ru: ("Эмодзи", "Панель эмодзи и символов для поля, где стоит курсор"), en: ("Emoji", "Emoji and symbols for the field with the cursor") },
    Preset { id: "volumeup", icon: "volumeup", action: ("hotkey", "volumeup"), ru: ("Громче", "Прибавить громкость"), en: ("Louder", "Turn the volume up") },
    Preset { id: "volumedown", icon: "volumedown", action: ("hotkey", "volumedown"), ru: ("Тише", "Убавить громкость"), en: ("Quieter", "Turn the volume down") },
    Preset { id: "mute", icon: "mute", action: ("hotkey", "volumemute"), ru: ("Без звука", "Выключить или вернуть звук"), en: ("Mute", "Turn the sound off or back on") },
    Preset { id: "playpause", icon: "playpause", action: ("hotkey", "playpause"), ru: ("Пауза", "Остановить или продолжить музыку и видео"), en: ("Play, pause", "Pause or resume music and video") },
    Preset { id: "nexttrack", icon: "nexttrack", action: ("hotkey", "nexttrack"), ru: ("Следующий трек", "Перейти к следующей композиции"), en: ("Next track", "Skip to the next track") },
    Preset { id: "holdshift", icon: "shift", action: ("hold", "shift"), ru: ("Держать Shift", HOLD_HINT.0), en: ("Hold Shift", HOLD_HINT.1) },
    Preset { id: "holdctrl", icon: "ctrl", action: ("hold", "ctrl"), ru: ("Держать Ctrl", HOLD_HINT.0), en: ("Hold Ctrl", HOLD_HINT.1) },
    Preset { id: "holdalt", icon: "alt", action: ("hold", "alt"), ru: ("Держать Alt", HOLD_HINT.0), en: ("Hold Alt", HOLD_HINT.1) },
    Preset { id: "holdspace", icon: "space", action: ("hold", "space"), ru: ("Держать Space", HOLD_HINT.0), en: ("Hold Space", HOLD_HINT.1) },
    Preset { id: "esc", icon: "esc", action: ("hotkey", "esc"), ru: ("Esc", "Выйти или отменить в активном окне"), en: ("Esc", "Leave or cancel in the active window") },
    Preset { id: "left", icon: "left", action: ("hotkey", "left"), ru: ("Влево", "Стрелка влево: назад в видео, к предыдущему"), en: ("Left", "Left arrow: back in a video, to the previous one") },
    Preset { id: "right", icon: "right", action: ("hotkey", "right"), ru: ("Вправо", "Стрелка вправо: вперёд в видео, к следующему"), en: ("Right", "Right arrow: ahead in a video, to the next one") },
    Preset { id: "up", icon: "up", action: ("hotkey", "up"), ru: ("Вверх", "Стрелка вверх: выше по списку или странице"), en: ("Up", "Up arrow: up a list or a page") },
    Preset { id: "down", icon: "down", action: ("hotkey", "down"), ru: ("Вниз", "Стрелка вниз: ниже по списку или странице"), en: ("Down", "Down arrow: down a list or a page") },
    Preset { id: "spacebar", icon: "spacebar", action: ("hotkey", "space"), ru: ("Пробел", "Пауза и пуск в плеере, пробел в тексте"), en: ("Space", "Pause and play in a player; a space in text") },
    Preset { id: "fullscreen", icon: "fullscreen", action: ("hotkey", "f11"), ru: ("Во весь экран", "Развернуть активное окно во весь экран или вернуть"), en: ("Full screen", "Make the active window full screen or bring it back") },
];

/// The hint of the ready actions that hold a key; the 20 seconds are `actions::HOLD_IDLE_MS`.
const HOLD_HINT: (&str, &str) =
    ("До повторного касания, отпускается сам через 20 с без нажатий", "Until tapped again; lets go by itself after 20 s without taps");

/// The ready actions a new settings file starts with, in this order: four, so all show without
/// scrolling. Chosen once, by what the machine has when the first file is written: with a keyboard
/// attached the on-screen keyboard gives way to Redo, the pair of Undo for drawing; with a pen File
/// Explorer gives way to Hold Shift.
pub fn default_ids(keyboard: bool, pen: bool) -> [&'static str; 4] {
    ["undo", "snip", if keyboard { "redo" } else { "keyboard" }, if pen { "holdshift" } else { "explorer" }]
}

/// The item of the settings file a ready action makes, named in Russian or in English.
pub fn preset_item(preset: &Preset, russian: bool) -> Value {
    let (name, hint) = if russian { preset.ru } else { preset.en };
    let (kind, value) = preset.action;
    let field = if kind == "open" { "target" } else { "keys" };
    json!({ "name": name, "icon": preset.icon, "hint": hint, "action": kind, field: value })
}

/// Pages of Windows Settings the editor offers: the address and the name in Russian and in English.
/// The addresses are Windows' own and open on any Windows 11.
pub const PAGES: [(&str, &str, &str); 10] = [
    ("ms-settings:bluetooth", "Bluetooth", "Bluetooth"),
    ("ms-settings:network-wifi", "Wi-Fi", "Wi-Fi"),
    ("ms-settings:sound", "Звук", "Sound"),
    ("ms-settings:display", "Экран", "Display"),
    ("ms-settings:nightlight", "Ночной свет", "Night light"),
    ("ms-settings:batterysaver", "Батарея", "Battery"),
    ("ms-settings:notifications", "Уведомления", "Notifications"),
    ("ms-settings:printers", "Принтеры", "Printers"),
    ("ms-settings:windowsupdate", "Обновление", "Windows Update"),
    ("ms-settings:pen", "Перо", "Pen"),
];

/// The item of the settings file a page of Windows Settings makes, with the built-in settings icon.
pub fn page_item(page: &(&str, &str, &str), russian: bool) -> Value {
    let (target, ru, en) = *page;
    json!({ "name": if russian { ru } else { en }, "icon": "gear", "action": "open", "target": target })
}

/// A ready set: ready actions the editor adds together, all of them for one program if asked.
pub struct Bundle {
    pub id: &'static str,
    /// Its name in Russian and in English.
    pub ru: &'static str,
    pub en: &'static str,
    /// The ready actions it adds, in this order.
    pub presets: &'static [&'static str],
}

pub const BUNDLES: [Bundle; 4] = [
    Bundle { id: "tablet", ru: "Планшет", en: "Tablet", presets: &["snip", "paste", "voice", "tasks", "keyboard"] },
    Bundle { id: "drawing", ru: "Рисование", en: "Drawing", presets: &["undo", "redo", "holdshift", "holdctrl", "holdspace", "snip"] },
    Bundle { id: "text", ru: "Текст", en: "Text", presets: &["copy", "paste", "undo", "selectall", "voice"] },
    Bundle { id: "video", ru: "Видео", en: "Video", presets: &["esc", "left", "right", "spacebar", "fullscreen"] },
];

/// Whether two actions do the same: the same keys however they are written (a held `control` is a
/// held `ctrl`), or the same target.
fn same_action(a: &Action, b: &Action) -> bool {
    let keys = |k: &str| crate::keys::parse(k).ok().map(|mut vk| { vk.sort(); vk });
    match (a, b) {
        (Action::Hotkey(x), Action::Hotkey(y)) => keys(x).is_some() && keys(x) == keys(y),
        (Action::Hold(x), Action::Hold(y)) => crate::keys::holdable(x).is_some() && crate::keys::holdable(x) == crate::keys::holdable(y),
        (Action::Open(x), Action::Open(y)) => x.trim().to_lowercase() == y.trim().to_lowercase(),
        _ => false,
    }
}

/// The items a ready set adds to the end of `existing`, named in Russian or in English, each for
/// the program `only_in` names if one does. An action the list already has for the same program,
/// or for all programs when none is named, is not added again; the existing items stay as they are.
pub fn bundle_items(bundle: &Bundle, russian: bool, only_in: Option<&str>, existing: &[Value]) -> Vec<Value> {
    let only_in = only_in.map(str::trim).filter(|p| !p.is_empty());
    let mut have: Vec<Item> = existing.iter().filter_map(|v| parse_item(v).ok()).collect();
    let mut added = Vec::new();
    for preset in bundle.presets.iter().filter_map(|id| PRESETS.iter().find(|p| p.id == *id)) {
        let mut raw = preset_item(preset, russian);
        if let Some(program) = only_in {
            raw["only_in"] = json!(program);
        }
        let Ok(item) = parse_item(&raw) else { continue };
        if has(&have, &item) {
            continue;
        }
        have.push(item);
        added.push(raw);
    }
    added
}

/// Whether the list already has an item that does the same for the same program, or for all
/// programs when the item names none.
fn has(have: &[Item], item: &Item) -> bool {
    have.iter().any(|h| same_action(&h.action, &item.action) && h.only_in == item.only_in)
}

/// A key set is a file of items a person saves and hands to another: `{ "items": [...] }`, the
/// items written as the settings file writes them. It comes from anywhere, so it is read
/// strictly and is no larger than this.
pub const SET_MAX_BYTES: u64 = 256 * 1024;
pub const SET_MAX_ITEMS: usize = 200;

/// An item as a key set carries it: the name, the hint, the action, a built-in icon and the
/// program it is for. Nothing else of the item goes along: not `lit`, not a picture that is a
/// file, not a field the panel does not know.
fn set_item(item: &Item) -> Value {
    let (kind, field, value) = match &item.action {
        Action::Open(target) => ("open", "target", target),
        Action::Hotkey(keys) => ("hotkey", "keys", keys),
        Action::Hold(keys) => ("hold", "keys", keys),
    };
    let mut raw = json!({ "name": item.name });
    if let Some(icon) = item.icon.as_deref().filter(|i| crate::glyphs::builtin(i).is_some()) {
        raw["icon"] = json!(icon);
    }
    if let Some(hint) = &item.hint {
        raw["hint"] = json!(hint);
    }
    raw["action"] = json!(kind);
    raw[field] = json!(value);
    if let Some(program) = &item.only_in {
        raw["only_in"] = json!(program);
    }
    raw
}

/// The text of a key set made of the settings text: every item the panel uses, or only those for
/// the program `only_in` names; and how many they are. Nothing but items goes into it. A set
/// that `read_set` would refuse for its size is not made: the error is `large`.
pub fn set_text(settings: &str, only_in: Option<&str>) -> Result<(String, usize), &'static str> {
    let v: Option<Value> = serde_json::from_str(settings.trim_start_matches('\u{feff}')).ok();
    let only_in = only_in.map(|p| p.trim().to_lowercase()).filter(|p| !p.is_empty());
    let items: Vec<Value> = v
        .as_ref()
        .and_then(|v| v.get("items")?.as_array())
        .ok_or("the settings file is not a JSON object")?
        .iter()
        .filter_map(|raw| parse_item(raw).ok())
        .filter(|item| only_in.is_none() || item.only_in == only_in)
        .map(|item| set_item(&item))
        .collect();
    let n = items.len();
    let text = to_text(&json!({ "items": items }));
    if n > SET_MAX_ITEMS || text.len() as u64 > SET_MAX_BYTES {
        return Err("large");
    }
    Ok((text, n))
}

/// The file a key set is written to when the save dialog answered `chosen`: a key set is a
/// `.tapka` file whatever name was typed. The dialog asked about writing over `chosen` only, so
/// when the name had to be completed and `exists` says that file is there, there is none.
pub fn set_path(chosen: &Path, exists: impl Fn(&Path) -> bool) -> Option<std::path::PathBuf> {
    if chosen.extension().is_some_and(|e| e.eq_ignore_ascii_case("tapka")) {
        return Some(chosen.to_path_buf());
    }
    let mut path = chosen.to_path_buf();
    path.as_mut_os_string().push(".tapka");
    (!exists(&path)).then_some(path)
}

/// The items of a key set, each as `set_item` writes it, or `None` when this is not a key set: not
/// an object with a list of items, the list empty or too long, or an item the panel cannot use.
pub fn parse_set(v: &Value) -> Option<Vec<Value>> {
    let items = v.as_object()?.get("items")?.as_array()?;
    if items.is_empty() || items.len() > SET_MAX_ITEMS {
        return None;
    }
    items.iter().map(|raw| parse_item(raw).ok().map(|item| set_item(&item))).collect()
}

/// Reads a key set file. A file that is too large is not read at all.
pub fn read_set(path: &Path) -> Option<Vec<Value>> {
    if std::fs::metadata(path).ok()?.len() > SET_MAX_BYTES {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    parse_set(&serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?)
}

/// What a key set adds to the end of `existing`: the items to add, each for the program it names
/// itself; how many the list already has (the rule of the ready sets); and how many open
/// something `found` says is not on this machine. `None` when `set` is not the items of a key set.
pub fn set_items(set: &[Value], existing: &[Value], found: impl Fn(&str) -> bool) -> Option<(Vec<Value>, usize, usize)> {
    let set = parse_set(&json!({ "items": set }))?;
    let mut have: Vec<Item> = existing.iter().filter_map(|v| parse_item(v).ok()).collect();
    let (mut added, mut had, mut missing) = (Vec::new(), 0, 0);
    for raw in set {
        let Ok(item) = parse_item(&raw) else { continue };
        if has(&have, &item) {
            had += 1;
        } else if matches!(&item.action, Action::Open(target) if !found(target)) {
            missing += 1;
        } else {
            have.push(item);
            added.push(raw);
        }
    }
    Some((added, had, missing))
}

/// The file written on first run: ready actions of Windows only, named in the system's language,
/// for a machine with or without a `keyboard` attached and a `pen` (`default_ids`).
pub fn default_text(russian: bool, keyboard: bool, pen: bool) -> String {
    let items: Vec<Value> = default_ids(keyboard, pen)
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
            let text = match value.as_array().filter(|items| (key == "items" || key == "meters" || key == "docked") && !items.is_empty()) {
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

/// Whether the settings text says the welcome of the first run is over: `"tour_done": true`. The
/// name is the one the three-step tour of earlier builds wrote, so a file it finished stays finished.
pub fn tour_done(text: &str) -> bool {
    serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).is_ok_and(|v| v.get("tour_done") == Some(&Value::Bool(true)))
}

/// The settings text after a carry: the capsule is at `edge` now, and `along` it stands the
/// capsule or, with `folded`, the folded key. The other of the two places is kept.
pub fn with_landing(text: &str, edge: Edge, folded: bool, along: f64) -> Option<String> {
    let text = with_field(text, "edge", json!(edge_name(edge)))?;
    with_field(&text, if folded { "folded_at" } else { "top" }, share(along))
}

/// The settings text with the entry at `from` of the list `key`, `items` or `meters`, put where the
/// entry at `to` is. Both count the entries the panel uses, the way `parse` numbers them; an entry
/// the panel skips keeps its place in the file.
pub fn with_moved(text: &str, key: &str, from: usize, to: usize) -> Option<String> {
    let mut v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let items = v.get_mut(key)?.as_array_mut()?;
    let used: Vec<usize> = (0..items.len()).filter(|&i| if key == "meters" { parse_meter(&items[i]).is_ok() } else { parse_item(&items[i]).is_ok() }).collect();
    let (from, to) = (*used.get(from)?, *used.get(to)?);
    let item = items.remove(from);
    items.insert(to, item);
    Some(to_text(&v))
}

/// What the page knows of the entry at `at` of the list `key` in the settings text, counted as
/// `with_moved` counts: its name, and for a meter its program, none for a meter of a file or a limit. A move
/// goes by places, and the file may have changed since the page drew them.
pub fn entry_at(text: &str, key: &str, at: usize) -> Option<(String, Option<String>)> {
    let (s, _) = parse(text).ok()?;
    if key == "meters" {
        let m = s.meters.get(at)?;
        let program = match &m.source {
            Source::Program(p) => Some(p.clone()),
            Source::File(_) | Source::Limit(_) => None,
        };
        return Some((m.name.clone(), program));
    }
    s.items.get(at).map(|it| (it.name.clone(), None))
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
        Some("hold") => {
            let keys = text_field(v, "keys").ok_or("empty keys")?.to_lowercase();
            crate::keys::holdable(&keys).ok_or(format!("cannot hold \"{keys}\": only shift, ctrl, alt or space"))?;
            Action::Hold(keys)
        }
        Some(other) => return Err(format!("unknown action \"{other}\"")),
        None => return Err("no action".into()),
    };
    // A `lit` without both parts is left out rather than failing the item
    let lit = v.get("lit").and_then(|l| Some((text_field(l, "program")?.to_lowercase(), text_field(l, "window")?)));
    let only_in = text_field(v, "only_in").map(|p| p.to_lowercase());
    Ok(Item { name, icon: text_field(v, "icon"), hint: text_field(v, "hint"), lit, only_in, action })
}

/// One meter of the file, or why it cannot be used. It has one source: a program, a file or a limit.
pub fn parse_meter(v: &Value) -> Result<Meter, String> {
    let name = text_field(v, "name").ok_or("no name")?;
    let limit = text_field(v, "limit").map(|l| crate::limits::Service::named(&l).ok_or(format!("unknown limit \"{l}\""))).transpose()?;
    let source = match (text_field(v, "program"), text_field(v, "file"), limit) {
        (Some(program), None, None) => Source::Program(program),
        (None, Some(file), None) => Source::File(file),
        (None, None, Some(service)) => Source::Limit(service),
        (None, None, None) => return Err("no program, file or limit".into()),
        _ => return Err("more than one of a program, a file and a limit".into()),
    };
    Ok(Meter { name, source, app: text_field(v, "app") })
}

/// The file name of a program, lower case, from its name or its path; none from what names no file.
fn file_name(text: &str) -> Option<String> {
    let name = text.trim().rsplit(['\\', '/']).next()?.trim().to_lowercase();
    (!name.is_empty()).then_some(name)
}

/// One docked entry of the file, or why it cannot be used: the file name alone, or an object with
/// the path and the class of the window.
pub fn parse_docked(v: &Value) -> Result<Docked, String> {
    if let Some(name) = v.as_str() {
        return file_name(name).map(|exe| Docked { exe, path: None, class: None }).ok_or("not a program file name".into());
    }
    let path = text_field(v, "path");
    let exe = text_field(v, "exe").or_else(|| path.clone()).and_then(|e| file_name(&e)).ok_or("not a program file name")?;
    Ok(Docked { exe, path, class: text_field(v, "class") })
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
    let mut meters = Vec::new();
    for (i, meter) in v.get("meters").and_then(Value::as_array).into_iter().flatten().enumerate() {
        match parse_meter(meter) {
            Ok(m) => meters.push(m),
            Err(why) => skipped.push(format!("meter {} skipped: {why}", i + 1)),
        }
    }
    let mut docked: Vec<Docked> = Vec::new();
    for (i, entry) in v.get("docked").and_then(Value::as_array).into_iter().flatten().enumerate() {
        match parse_docked(entry) {
            Ok(d) if !docked.iter().any(|o| o.key() == d.key()) => docked.push(d),
            Ok(_) => {}
            Err(why) => skipped.push(format!("docked {} skipped: {why}", i + 1)),
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
        tablet_only: v.get("tablet_only").and_then(Value::as_bool).unwrap_or(false),
        click_sound: v.get("click_sound").and_then(Value::as_bool).unwrap_or(false),
        folded: v.get("folded").and_then(Value::as_bool).unwrap_or(false),
        folded_at: v.get("folded_at").and_then(Value::as_f64).unwrap_or(DEFAULT_TOP),
        items: good,
        meters,
        divider: v.get("divider").and_then(Value::as_f64).filter(|d| d.is_finite()).map(|d| d.clamp(0.0, 1.0)),
        length: v.get("length").and_then(Value::as_u64).map(|l| (l as usize).max(crate::layout::LEAST)),
        docked,
    };
    Ok((settings, skipped))
}

/// Reads the settings file, writing the default one first if the file does not exist: `first`
/// makes its text (`default_text`) and is asked only then. The last value says whether the file
/// was created just now.
pub fn load_or_create(path: &Path, first: impl FnOnce() -> String) -> Result<(Settings, Vec<String>, bool), String> {
    let created = !path.exists();
    if created {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create folder: {e}"))?;
        }
        std::fs::write(path, first()).map_err(|e| format!("cannot write settings: {e}"))?;
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read settings: {e}"))?;
    // Notepad may save with a byte-order mark
    let (settings, skipped) = parse(text.trim_start_matches('\u{feff}'))?;
    Ok((settings, skipped, created))
}

/// Changes the settings file at `path`: reads it as it is on the disk now, lets `change` make the
/// new text of that, and puts the new text in its place with `ReplaceFileW`, which keeps the version
/// replaced as `settings.json.bak` in the same step, so a reader never finds the file half written
/// and no save is lost between a copy and a rename. Whatever an editor saved since the panel last
/// read the file is in what `change` gets; one that saves while the new text is being made is found
/// just before the replacement, and the file is read and changed again, up to three times. A file
/// that cannot be read, or is not a JSON object, is left as it is.
pub fn change_file(path: &Path, mut change: impl FnMut(&str) -> Option<String>) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    for _ in 0..3 {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read settings: {e}"))?;
        let text_now = change(&text).ok_or("the settings file is not a JSON object, or no longer holds what was to change, so it is left as it is")?;
        std::fs::write(&tmp, text_now).map_err(|e| format!("cannot write settings: {e}"))?;
        if std::fs::read_to_string(path).ok().as_deref() != Some(text.as_str()) {
            continue;
        }
        return replace_file(path, &tmp, &path.with_extension("json.bak"));
    }
    let _ = std::fs::remove_file(&tmp);
    Err("the settings file kept changing while it was saved, so it is left as it is".into())
}

/// Puts `with` in the place of `path`, which becomes `backup`, in one step.
fn replace_file(path: &Path, with: &Path, backup: &Path) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::{ReplaceFileW, REPLACE_FILE_FLAGS};
    let name = |p: &Path| HSTRING::from(p.as_os_str());
    unsafe { ReplaceFileW(&name(path), &name(with), &name(backup), REPLACE_FILE_FLAGS(0), None, None) }.map_err(|e| format!("cannot write settings: {e}"))
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
    fn the_welcome_is_over_once_the_flag_says_so_and_the_flag_keeps_the_rest() {
        let first = default_text(true, true, true);
        assert!(!tour_done(&first));
        // Over: written once, and nothing else of the file changed
        let over = with_field(&first, "tour_done", json!(true)).unwrap();
        assert!(tour_done(&over));
        assert_eq!(parse(&over).unwrap().0, parse(&first).unwrap().0);
        let own = with_field(OWN, "tour_done", json!(true)).unwrap();
        assert!(own.contains("  \"note\": \"mine\",\n  \"tour_done\": true,\n  \"items\": ["), "{own}");
        // Anything but true is not over; neither is a file that is not JSON
        assert!(!tour_done(&with_field(&first, "tour_done", json!("yes")).unwrap()));
        assert!(!tour_done("not json"));
    }

    #[test]
    fn an_item_is_moved_among_the_items_the_panel_uses() {
        let file = |names: &[&str]| {
            let items: Vec<Value> = names.iter().map(|n| if *n == "broken" { json!({ "name": n }) } else { json!({ "name": n, "action": "open", "target": "x" }) }).collect();
            to_text(&json!({ "top": 0.5, "items": items }))
        };
        let names = |text: &str| parse(text).unwrap().0.items.iter().map(|i| i.name.clone()).collect::<Vec<_>>().join(" ");
        let text = file(&["a", "b", "c", "d"]);
        assert_eq!(names(&with_moved(&text, "items", 0, 2).unwrap()), "b c a d");
        assert_eq!(names(&with_moved(&text, "items", 3, 0).unwrap()), "d a b c");
        assert_eq!(with_moved(&text, "items", 1, 1).unwrap(), text);
        // An item the panel skips is not counted and stays where it is in the file
        let text = file(&["a", "broken", "b", "c"]);
        assert_eq!(with_moved(&text, "items", 0, 2).unwrap(), file(&["broken", "b", "c", "a"]));
        assert_eq!(with_moved(&text, "items", 2, 0).unwrap(), file(&["c", "a", "broken", "b"]));
        // A place the list does not have, and not a settings file at all
        assert!(with_moved(&text, "items", 0, 3).is_none());
        assert!(with_moved("not json", "items", 0, 1).is_none());
    }

    #[test]
    fn a_meter_is_moved_among_the_meters_the_panel_uses_and_the_items_stay() {
        let file = |names: &[&str]| {
            let meters: Vec<Value> = names.iter().map(|n| if *n == "broken" { json!({ "name": n }) } else { json!({ "name": n, "file": format!("D:\\{n}.txt"), "mine": 1 }) }).collect();
            to_text(&json!({ "note": "mine", "meters": meters, "items": [{ "name": "k", "action": "open", "target": "x" }] }))
        };
        let names = |text: &str| parse(text).unwrap().0.meters.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(" ");
        let text = file(&["a", "b", "c"]);
        assert_eq!(names(&with_moved(&text, "meters", 0, 2).unwrap()), "b c a");
        assert_eq!(names(&with_moved(&text, "meters", 2, 0).unwrap()), "c a b");
        // The meter takes its file and its own fields along; the items and the rest stay
        let moved = with_moved(&text, "meters", 0, 1).unwrap();
        assert_eq!(moved, file(&["b", "a", "c"]));
        assert_eq!(parse(&moved).unwrap().0.meters[1].source, Source::File("D:\\a.txt".into()));
        // A meter the panel skips is not counted and stays where it is
        let text = file(&["a", "broken", "b"]);
        assert_eq!(with_moved(&text, "meters", 0, 1).unwrap(), file(&["broken", "b", "a"]));
        assert!(with_moved(&text, "meters", 0, 2).is_none());
        // The entry at a place, as the page knows it, counted the same way: a meter added in front
        // since the page drew the list is not the one it carried
        assert_eq!(entry_at(&text, "meters", 1), Some(("b".into(), None)));
        let program = to_text(&json!({ "meters": [{ "name": "x", "program": "C:\\x.exe" }, { "name": "a", "file": "D:\\a.txt" }], "items": [{ "name": "k", "action": "open", "target": "x" }] }));
        assert_eq!(entry_at(&program, "meters", 0), Some(("x".into(), Some("C:\\x.exe".into()))));
        assert_ne!(entry_at(&program, "meters", 0), Some(("a".into(), None)));
        assert_eq!(entry_at(&program, "items", 0), Some(("k".into(), None)));
        assert_eq!(entry_at(&program, "meters", 2), None);
        // A meter of a limit is counted and carried as a file's is, with no program
        let limit = to_text(&json!({ "meters": [{ "name": "a", "file": "D:\\a.txt" }, { "name": "Claude", "limit": "claude" }], "items": [] }));
        assert_eq!(entry_at(&limit, "meters", 1), Some(("Claude".into(), None)));
        assert_eq!(names(&with_moved(&limit, "meters", 1, 0).unwrap()), "Claude a");
        assert_eq!(entry_at("not json", "meters", 0), None);
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
        // A file from before the switch existed: the capsule stays on screen with a keyboard too
        assert!(!s.tablet_only);
        assert!(parse(r#"{"tablet_only":true,"items":[]}"#).unwrap().0.tablet_only);
        assert!(!parse(r#"{"tablet_only":"yes","items":[]}"#).unwrap().0.tablet_only);
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
    fn the_tap_sound_is_off_unless_the_file_asks_for_it() {
        // A file from before the switch existed, the default file, and anything that is not a switch
        assert!(!parse(r#"{"items":[]}"#).unwrap().0.click_sound);
        assert!(!parse(&default_text(true, false, false)).unwrap().0.click_sound);
        assert!(!Settings::empty().click_sound);
        assert!(!parse(r#"{"click_sound":"yes","items":[]}"#).unwrap().0.click_sound);
        assert!(parse(r#"{"click_sound":true,"items":[]}"#).unwrap().0.click_sound);
        // The settings window turns it on: one field before the items, the rest of the file kept
        let out = with_field(OWN, "click_sound", json!(true)).unwrap();
        assert!(out.contains("  \"note\": \"mine\",
  \"click_sound\": true,
  \"items\": ["), "{out}");
        assert!(out.contains(r#""lit": { "program": "recorder.exe", "window": "Recording" }"#));
        assert!(parse(&out).unwrap().0.click_sound);
        let off = with_field(&out, "click_sound", json!(false)).unwrap();
        assert_eq!(off, out.replace("\"click_sound\": true", "\"click_sound\": false"));
        assert!(!parse(&off).unwrap().0.click_sound);
    }

    #[test]
    fn a_folded_capsule_is_written_into_the_file_and_the_rest_of_the_file_is_kept() {
        // A file from before the capsule could fold, the default file, and anything that is not a switch
        assert!(!parse(OWN).unwrap().0.folded);
        assert!(!parse(&default_text(true, false, false)).unwrap().0.folded);
        assert!(!parse(r#"{"folded":"yes","folded_at":"low","items":[]}"#).unwrap().0.folded);
        // Folding writes two fields before the items; the owner's field, `lit` and the item of a
        // kind this version does not know stay as they were
        let out = with_field(OWN, "folded", json!(true)).and_then(|text| with_field(&text, "folded_at", share(0.61237))).unwrap();
        assert_eq!(out, OWN.replace("  \"items\": [", "  \"folded\": true,\n  \"folded_at\": 0.6124,\n  \"items\": ["));
        let (s, _) = parse(&out).unwrap();
        assert_eq!((s.folded, s.folded_at, s.top), (true, 0.6124, 0.4529));
        // Unfolding keeps the place for the next time
        let back = with_field(&out, "folded", json!(false)).unwrap();
        assert_eq!(back, out.replace("\"folded\": true", "\"folded\": false"));
        assert_eq!((parse(&back).unwrap().0.folded, parse(&back).unwrap().0.folded_at), (false, 0.6124));
        // The folded key carried to the top edge: the capsule's edge goes with it, its own place
        // along an edge and everything the panel does not know stay
        let moved = with_landing(&out, Edge::Top, true, 0.25).unwrap();
        assert_eq!(moved, out.replace("\"edge\": \"right\"", "\"edge\": \"top\"").replace("0.6124", "0.25"));
        let (s, _) = parse(&moved).unwrap();
        assert_eq!((s.edge, s.folded, s.folded_at, s.top), (Edge::Top, true, 0.25, 0.4529));
        // An open capsule carried there keeps the folded key's place instead
        let (s, _) = parse(&with_landing(&back, Edge::Left, false, 0.3).unwrap()).unwrap();
        assert_eq!((s.edge, s.folded_at, s.top), (Edge::Left, 0.6124, 0.3));
    }

    #[test]
    fn meters_and_the_divider_are_written_into_the_file_and_the_rest_of_the_file_is_kept() {
        // A file from before the meters: none, and no divider
        let (s, skipped) = parse(OWN).unwrap();
        assert!(s.meters.is_empty() && s.divider.is_none() && skipped.len() == 1);
        // The settings window writes the list, one meter per line, before the items; the owner's
        // field, `lit`, the item of a kind this version does not know and a meter's own field stay
        let meters = json!([
            { "name": "Render", "program": "C:\\Tools\\render.exe", "app": "Vendor.Render", "mine": 2 },
            { "name": "Backup", "file": "D:\\backup.txt" },
            { "name": "Both", "program": "a.exe", "file": "b.txt" },
            { "name": "None" },
            { "name": "Claude", "limit": "claude" },
            { "name": "GPT", "limit": " ChatGPT " },
            { "name": "Other", "limit": "gemini" },
            { "name": "Two", "file": "b.txt", "limit": "claude" },
        ]);
        let out = with_field(OWN, "meters", meters).and_then(|text| with_field(&text, "divider", share(0.61237))).unwrap();
        assert!(out.contains("  \"note\": \"mine\",\n  \"meters\": [\n    { \"name\": \"Render\", \"program\": \"C:\\\\Tools\\\\render.exe\", \"app\": \"Vendor.Render\", \"mine\": 2 },\n"), "{out}");
        assert!(out.contains("  \"divider\": 0.6124,\n  \"items\": ["), "{out}");
        assert!(out.contains(r#""lit": { "program": "recorder.exe", "window": "Recording" }"#));
        assert!(out.contains(r#"{ "name": "Later", "action": "script", "run": ["a", "b"] }"#));
        let (s, skipped) = parse(&out).unwrap();
        assert_eq!(
            s.meters,
            [
                Meter { name: "Render".into(), source: Source::Program("C:\\Tools\\render.exe".into()), app: Some("Vendor.Render".into()) },
                Meter { name: "Backup".into(), source: Source::File("D:\\backup.txt".into()), app: None },
                Meter { name: "Claude".into(), source: Source::Limit(crate::limits::Service::Claude), app: None },
                Meter { name: "GPT".into(), source: Source::Limit(crate::limits::Service::Chatgpt), app: None },
            ]
        );
        assert_eq!(s.divider, Some(0.6124));
        for why in ["meter 3 skipped: more than one of a program, a file and a limit", "meter 4 skipped: no program, file or limit", "meter 7 skipped: unknown limit \"gemini\"", "meter 8 skipped: more than one of a program, a file and a limit"] {
            assert!(skipped.contains(&why.to_string()), "{skipped:?}");
        }
        // Read back and written again, the text stays as it is
        assert_eq!(to_text(&serde_json::from_str::<Value>(&out).unwrap()), out);
        // Moving the divider changes that one line; an empty list is no meters
        let moved = with_field(&out, "divider", share(0.25)).unwrap();
        assert_eq!(moved, out.replace("0.6124", "0.25"));
        let none = with_field(&out, "meters", json!([])).unwrap();
        assert!(none.contains("  \"meters\": [],\n") && parse(&none).unwrap().0.meters.is_empty(), "{none}");
        // Odd values do not break the file
        let (s, _) = parse(r#"{"meters":"x","divider":"low","items":[]}"#).unwrap();
        assert!(s.meters.is_empty() && s.divider.is_none());
        assert_eq!(parse(r#"{"divider":7,"items":[]}"#).unwrap().0.divider, Some(1.0));
    }

    #[test]
    fn the_length_is_written_into_the_file_and_the_rest_of_the_file_is_kept() {
        // A file from before the end grip: no length, the capsule as long as its keys
        assert_eq!(parse(OWN).unwrap().0.length, None);
        // The release of the grip writes the length, and with it the place when that moved; the
        // owner's field, `lit` and the item of a kind this version does not know stay
        let out = with_field(OWN, "length", json!(6)).and_then(|text| with_field(&text, "top", share(0.21237))).unwrap();
        assert!(out.contains("  \"note\": \"mine\",\n  \"length\": 6,\n  \"items\": ["), "{out}");
        assert!(out.contains(r#""lit": { "program": "recorder.exe", "window": "Recording" }"#));
        assert!(out.contains(r#"{ "name": "Later", "action": "script", "run": ["a", "b"] }"#));
        let (s, _) = parse(&out).unwrap();
        assert_eq!((s.length, s.top, s.items.len()), (Some(6), 0.2124, 1));
        // A new length changes that one line
        assert_eq!(with_field(&out, "length", json!(3)).unwrap(), out.replace("\"length\": 6", "\"length\": 3"));
        // Odd values do not break the file: fewer than two cells are two, what is not a count is none
        assert_eq!(parse(r#"{"length":1,"items":[]}"#).unwrap().0.length, Some(2));
        assert_eq!(parse(r#"{"length":0,"items":[]}"#).unwrap().0.length, Some(2));
        for odd in ["-3", "2.5", "\"long\"", "null"] {
            assert_eq!(parse(&format!(r#"{{"length":{odd},"items":[]}}"#)).unwrap().0.length, None, "{odd}");
        }
    }

    #[test]
    fn the_docked_programs_are_written_into_the_file_and_the_rest_of_the_file_is_kept() {
        // A file from before docking docks nothing
        assert!(parse(OWN).unwrap().0.docked.is_empty());
        // The settings window writes the list, one entry per line, before the items; an entry's own
        // field, the owner's field and the items, the one this version does not know included, stay
        let entries = json!([
            { "exe": "panel.exe", "path": "C:\\Tools\\Panel.exe", "class": "PanelWindow", "mine": 1 },
            "other.exe",
        ]);
        let out = with_field(OWN, "docked", entries).unwrap();
        assert_eq!(
            out,
            OWN.replace(
                "  \"items\": [",
                "  \"docked\": [\n    { \"exe\": \"panel.exe\", \"path\": \"C:\\\\Tools\\\\Panel.exe\", \"class\": \"PanelWindow\", \"mine\": 1 },\n    \"other.exe\"\n  ],\n  \"items\": ["
            )
        );
        let (s, skipped) = parse(&out).unwrap();
        let panel = Docked { exe: "panel.exe".into(), path: Some("C:\\Tools\\Panel.exe".into()), class: Some("PanelWindow".into()) };
        assert_eq!(s.docked, [panel.clone(), Docked { exe: "other.exe".into(), path: None, class: None }]);
        assert_eq!(s.items.len(), 1);
        assert_eq!(skipped, ["item 2 skipped: unknown action \"script\""]);
        assert_eq!(to_text(&serde_json::from_str::<Value>(&out).unwrap()), out);
        // Undocking the last one leaves an empty list and everything else as it was
        let none = with_field(&out, "docked", json!([])).unwrap();
        assert_eq!(none, OWN.replace("  \"items\": [", "  \"docked\": [],\n  \"items\": ["));
        assert!(parse(&none).unwrap().0.docked.is_empty());
        // Written by hand: a path is its file name, the same entry twice is one, and what names no
        // program is skipped and said
        let (s, skipped) = parse(r#"{"docked":["C:\\Tools\\Panel.EXE"," panel.exe ",7,"","b.exe",{"class":"X"},{"path":"D:\\p.exe"}],"items":[]}"#).unwrap();
        assert_eq!(s.docked.iter().map(|d| d.exe.as_str()).collect::<Vec<_>>(), ["panel.exe", "b.exe", "p.exe"]);
        assert_eq!(s.docked[2].path.as_deref(), Some("D:\\p.exe"));
        assert_eq!(skipped, ["docked 3 skipped: not a program file name", "docked 4 skipped: not a program file name", "docked 6 skipped: not a program file name"]);
        assert!(parse(r#"{"docked":"panel.exe","items":[]}"#).unwrap().0.docked.is_empty());
        // An entry with a path goes by the path and the class; an old one by the file name
        assert!(panel.matches("panel.exe", "c:\\tools\\panel.exe", "PanelWindow"));
        assert!(!panel.matches("panel.exe", "D:\\Copy\\panel.exe", "PanelWindow"));
        assert!(!panel.matches("panel.exe", "C:\\Tools\\Panel.exe", "Tooltip"));
        let old = Docked { exe: "panel.exe".into(), path: None, class: None };
        assert!(old.matches("panel.exe", "D:\\Copy\\panel.exe", "Anything"));
        assert_ne!(old.key(), panel.key());
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
        let (s, _, created) = load_or_create(&new.join("settings.json"), String::new).unwrap();
        assert_eq!((s.top, created), (0.4, false));
        assert_eq!(std::fs::read(new.join("icons").join("own.png")).unwrap(), [1, 2, 3]);
        assert!(!new.join("panel.log").exists());
        // The old folder keeps everything
        assert!(old.join("settings.json").exists() && old.join("icons").join("own.png").exists());
        // A settings file already in the new folder is never replaced
        std::fs::write(new.join("settings.json"), r#"{"top":0.9,"items":[]}"#).unwrap();
        assert!(migrate(&old, &new).unwrap().is_empty());
        assert_eq!(load_or_create(&new.join("settings.json"), String::new).unwrap().0.top, 0.9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_save_keeps_what_an_editor_wrote_since_the_panel_read_the_file() {
        let dir = std::env::temp_dir().join(format!("tapka-change-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, OWN).unwrap();
        // The panel reads the file; then its owner adds a field and a key in an editor
        let (read, _, _) = load_or_create(&path, String::new).unwrap();
        assert_eq!(read.items.len(), 1);
        let edited = OWN.replace("\"note\": \"mine\",", "\"note\": \"mine\",\n  \"later\": [1, 2],").replace(
            "\"items\": [",
            "\"items\": [\n    { \"name\": \"Paint\", \"action\": \"open\", \"target\": \"C:\\\\Tools\\\\paint.exe\" },",
        );
        std::fs::write(&path, &edited).unwrap();
        // The panel saves the divider: the editor's field and key are there, and so is the divider
        change_file(&path, |text| with_field(text, "divider", share(0.3))).unwrap();
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["later"], json!([1, 2]));
        assert_eq!(saved["divider"], json!(0.3));
        assert_eq!(saved["items"][0]["name"], "Paint");
        assert_eq!(parse(&std::fs::read_to_string(&path).unwrap()).unwrap().0.items.len(), 2);
        // The version it replaced is kept beside it
        assert_eq!(std::fs::read_to_string(dir.join("settings.json.bak")).unwrap(), edited);
        // A file that is not JSON any more is not replaced, and the copy is not touched
        std::fs::write(&path, "{ half a file").unwrap();
        assert!(change_file(&path, |text| with_field(text, "divider", share(0.5))).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ half a file");
        assert_eq!(std::fs::read_to_string(dir.join("settings.json.bak")).unwrap(), edited);
        // Nor is a file that cannot be read
        assert!(change_file(&dir.join("gone.json"), |text| with_field(text, "divider", share(0.5))).is_err());
        assert!(!dir.join("gone.json").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_save_that_meets_an_editor_s_save_reads_the_file_again_and_gives_up_after_three() {
        let dir = std::env::temp_dir().join(format!("tapka-conflict-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        std::fs::write(&path, OWN).unwrap();
        // An editor saves while the panel makes its new text, once: the panel starts over from it
        let edited = OWN.replace("\"note\": \"mine\"", "\"note\": \"edited\"");
        let mut seen = Vec::new();
        change_file(&path, |text| {
            seen.push(text.to_string());
            if seen.len() == 1 {
                std::fs::write(&path, &edited).unwrap();
            }
            with_field(text, "divider", share(0.3))
        })
        .unwrap();
        assert_eq!(seen, [OWN.to_string(), edited.clone()]);
        let saved: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!((saved["note"].as_str(), saved["divider"].as_f64()), (Some("edited"), Some(0.3)));
        assert_eq!(std::fs::read_to_string(dir.join("settings.json.bak")).unwrap(), edited);
        // A second save replaces the kept copy with the version it replaced
        change_file(&path, |text| with_field(text, "divider", share(0.4))).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&std::fs::read_to_string(dir.join("settings.json.bak")).unwrap()).unwrap()["divider"], json!(0.3));
        // An editor that saves every time: after three tries nothing is written over it
        let before = std::fs::read_to_string(&path).unwrap();
        let mut n = 0;
        let result = change_file(&path, |text| {
            n += 1;
            std::fs::write(&path, format!("{before}{}", " ".repeat(n))).unwrap();
            with_field(text, "divider", share(0.9))
        });
        assert!(result.is_err());
        assert_eq!(n, 3);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), format!("{before}   "));
        assert!(!dir.join("settings.json.tmp").exists());
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
        for (keyboard, pen) in [(false, false), (true, false), (false, true), (true, true)] {
            assert!(default_ids(keyboard, pen).iter().all(|id| ids.contains(id)));
        }
    }

    #[test]
    fn the_settings_pages_are_named_in_both_languages_and_open_settings() {
        assert_eq!(PAGES.len(), 10);
        for russian in [true, false] {
            for page in &PAGES {
                let item = parse_item(&page_item(page, russian)).unwrap_or_else(|why| panic!("{}: {why}", page.0));
                assert!(!item.name.trim().is_empty(), "{}", page.0);
                assert_eq!(item.icon.as_deref(), Some("gear"));
                assert!(crate::glyphs::builtin("gear").is_some());
                assert!(matches!(&item.action, Action::Open(t) if t.starts_with("ms-settings:") && t.len() > "ms-settings:".len()), "{}", page.0);
            }
        }
        assert_eq!(page_item(&PAGES[2], true)["name"], "Звук");
        assert_eq!(page_item(&PAGES[2], false)["name"], "Sound");
        // No page twice
        let mut targets: Vec<_> = PAGES.iter().map(|p| p.0).collect();
        targets.sort();
        targets.dedup();
        assert_eq!(targets.len(), PAGES.len());
    }

    #[test]
    fn a_ready_set_adds_valid_items_once_and_keeps_the_rest() {
        let names = |items: &[Value]| -> Vec<String> { items.iter().map(|i| i["name"].as_str().unwrap().to_string()).collect() };
        // Every set is made of ready actions only, and each of them makes a valid item
        for bundle in &BUNDLES {
            assert!(!bundle.ru.is_empty() && !bundle.en.is_empty());
            for russian in [true, false] {
                let items = bundle_items(bundle, russian, None, &[]);
                assert_eq!(items.len(), bundle.presets.len(), "{}", bundle.id);
                for item in &items {
                    let item = parse_item(item).unwrap();
                    assert!(crate::glyphs::builtin(item.icon.as_deref().unwrap()).is_some());
                }
            }
        }
        let drawing = BUNDLES.iter().find(|b| b.id == "drawing").unwrap();
        assert_eq!(
            names(&bundle_items(drawing, true, None, &[])),
            ["Отменить", "Повторить", "Держать Shift", "Держать Ctrl", "Держать Space", "Снимок"]
        );
        let text = BUNDLES.iter().find(|b| b.id == "text").unwrap();
        assert_eq!(names(&bundle_items(text, false, None, &[])), ["Copy", "Paste", "Undo", "Select all", "Voice typing"]);
        let video = BUNDLES.iter().find(|b| b.id == "video").unwrap();
        assert_eq!(names(&bundle_items(video, true, None, &[])), ["Esc", "Влево", "Вправо", "Пробел", "Во весь экран"]);
        assert_eq!(names(&bundle_items(video, false, None, &[])), ["Esc", "Left", "Right", "Space", "Full screen"]);
        // For one program: every item says so, and the file reads it back
        let for_paint = bundle_items(text, true, Some("paint.exe"), &[]);
        assert!(for_paint.iter().all(|i| parse_item(i).unwrap().only_in.as_deref() == Some("paint.exe")));
        // What the list already has for the same program is not added twice, however it is written;
        // the same action for another program, or for all programs, does not count
        let existing = vec![
            json!({ "name": "Мой вставить", "action": "hotkey", "keys": "Ctrl+V", "only_in": "Paint.exe" }),
            json!({ "name": "Копия", "action": "hotkey", "keys": "ctrl+c" }),
            json!({ "name": "Отмена", "action": "hotkey", "keys": "ctrl+z", "only_in": "sketch.exe" }),
            json!({ "name": "Потом", "action": "script" }),
        ];
        assert_eq!(names(&bundle_items(text, true, Some("paint.exe"), &existing)), ["Копировать", "Отменить", "Выделить всё", "Голосовой ввод"]);
        assert_eq!(names(&bundle_items(text, true, None, &existing)), ["Вставить", "Отменить", "Выделить всё", "Голосовой ввод"]);
        // Added once, the set adds nothing the second time
        let once: Vec<Value> = existing.iter().cloned().chain(bundle_items(text, true, Some("paint.exe"), &existing)).collect();
        assert!(bundle_items(text, true, Some("paint.exe"), &once).is_empty());
        // The held keys count as their own kind: a Shift shortcut is not a held Shift
        let shift = vec![json!({ "name": "s", "action": "hotkey", "keys": "shift" })];
        assert_eq!(bundle_items(drawing, false, None, &shift).len(), 6);
        let held = vec![json!({ "name": "s", "action": "hold", "keys": "Shift" })];
        assert_eq!(bundle_items(drawing, false, None, &held).len(), 5);
        // Held Ctrl written as the file may have it
        let control = vec![json!({ "name": "c", "action": "hold", "keys": "Control" })];
        assert_eq!(names(&bundle_items(drawing, false, None, &control)), ["Undo", "Redo", "Hold Shift", "Hold Space", "Snip"]);
        // The new items go after the existing ones in the file and survive a save
        let saved = with_field(OWN, "items", json!(once)).unwrap();
        let (s, skipped) = parse(&saved).unwrap();
        assert_eq!((s.items.len(), skipped.len()), (7, 1));
        assert_eq!(s.items[3].name, "Копировать");
        assert_eq!(s.items[3].only_in.as_deref(), Some("paint.exe"));
    }

    /// The owner's file with keys of every kind: for all programs and for one, with `lit`, with a
    /// picture that is a file, with a field the panel does not know, and one it cannot use.
    fn with_keys() -> String {
        OWN.replace(
            r#"{ "name": "Later""#,
            r#"{ "name": "Brush", "icon": "own.png", "hint": "Soft", "action": "hotkey", "keys": "b", "only_in": "Paint.exe", "mine": 1 },
    { "name": "Shift", "icon": "shift", "action": "hold", "keys": "shift", "only_in": "paint.exe" },
    { "name": "Notes", "action": "open", "target": "C:\\Tools\\notes.exe" },
    { "name": "Later""#,
        )
    }

    #[test]
    fn saved_keys_carry_items_only_and_come_back_without_repeats() {
        let file = with_keys();
        let existing = serde_json::from_str::<Value>(&file).unwrap()["items"].as_array().unwrap().clone();
        let (text, n) = set_text(&file, None).unwrap();
        assert_eq!(n, 4);
        // Nothing of the look, the place, the `lit` rule, a picture file or an unknown field
        for word in ["scale", "top", "edge", "\"note\"", "lit", "recorder", "own.png", "mine", "Later"] {
            assert!(!text.contains(word), "{word}: {text}");
        }
        assert_eq!(
            text,
            r#"{
  "items": [
    { "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space" },
    { "name": "Brush", "hint": "Soft", "action": "hotkey", "keys": "b", "only_in": "paint.exe" },
    { "name": "Shift", "icon": "shift", "action": "hold", "keys": "shift", "only_in": "paint.exe" },
    { "name": "Notes", "action": "open", "target": "C:\\Tools\\notes.exe" }
  ]
}
"#
        );
        // The keys of one program, however its name is written
        let (paint, n) = set_text(&file, Some("PAINT.exe")).unwrap();
        assert_eq!((n, paint.contains("Dictation"), paint.contains("Brush")), (2, false, true));
        // Read back, the set holds the same items, and the list it came from takes none of them
        let set = parse_set(&serde_json::from_str(&text).unwrap()).unwrap();
        let used: Vec<Item> = existing.iter().filter_map(|v| parse_item(v).ok()).collect();
        assert_eq!(set.len(), used.len());
        for (back, was) in set.iter().map(|v| parse_item(v).unwrap()).zip(&used) {
            assert_eq!((&back.name, &back.hint, &back.action, &back.only_in), (&was.name, &was.hint, &was.action, &was.only_in));
        }
        assert_eq!(set_items(&set, &existing, |_| true), Some((Vec::new(), 4, 0)));
        // An empty list takes them all, each for its own program
        assert_eq!(set_items(&set, &[], |_| true), Some((set.clone(), 0, 0)));
    }

    #[test]
    fn a_set_from_a_file_skips_what_is_not_here_and_keeps_the_rest_of_the_settings() {
        let set = vec![
            json!({ "name": "Gone", "action": "open", "target": "C:\\gone.exe" }),
            json!({ "name": "Site", "action": "open", "target": "https://example.com", "lit": { "program": "a.exe", "window": "b" }, "run": "x" }),
            json!({ "name": "Brush", "icon": "..\\own.png", "action": "hotkey", "keys": "b", "only_in": "absent.exe" }),
            json!({ "name": "Again", "action": "hotkey", "keys": "Ctrl+Space" }),
        ];
        let existing = serde_json::from_str::<Value>(OWN).unwrap()["items"].as_array().unwrap().clone();
        let (added, had, missing) = set_items(&set, &existing, |target| !target.contains("gone")).unwrap();
        assert_eq!((had, missing), (1, 1));
        // A shortcut for a program that is not here is added; what the file has beyond an item is dropped
        assert_eq!(added, [
            json!({ "name": "Site", "action": "open", "target": "https://example.com" }),
            json!({ "name": "Brush", "action": "hotkey", "keys": "b", "only_in": "absent.exe" }),
        ]);
        // Saved the way the editor saves, the owner's field, `lit` and the unknown item are still there
        let all: Vec<Value> = existing.iter().cloned().chain(added).collect();
        let saved = with_field(OWN, "items", json!(all)).unwrap();
        assert_eq!(
            saved,
            OWN.replace(
                r#""run": ["a", "b"] }"#,
                r#""run": ["a", "b"] },
    { "name": "Site", "action": "open", "target": "https://example.com" },
    { "name": "Brush", "action": "hotkey", "keys": "b", "only_in": "absent.exe" }"#,
            )
        );
    }

    #[test]
    fn a_set_over_the_limits_is_not_saved() {
        let key = |n: usize, hint: &str| format!(r#"{{ "name": "k{n}", "hint": "{hint}", "action": "hotkey", "keys": "b" }}"#);
        let settings = |n: usize, hint: &str| format!(r#"{{ "items": [{}] }}"#, (0..n).map(|i| key(i, hint)).collect::<Vec<_>>().join(","));
        // As many as a set may hold are saved, and the file reads back
        let (text, n) = set_text(&settings(SET_MAX_ITEMS, ""), None).unwrap();
        assert_eq!(n, SET_MAX_ITEMS);
        assert_eq!(parse_set(&serde_json::from_str(&text).unwrap()).unwrap().len(), SET_MAX_ITEMS);
        // One more, or fewer that make a file too large to be read
        assert_eq!(set_text(&settings(SET_MAX_ITEMS + 1, ""), None), Err("large"));
        assert_eq!(set_text(&settings(3, &"x".repeat(SET_MAX_BYTES as usize / 3)), None), Err("large"));
    }

    #[test]
    fn a_key_set_is_a_tapka_file_and_writes_over_none_the_dialog_did_not_ask_about() {
        let there = |p: &Path| p == Path::new(r"C:\a\notes.txt.tapka");
        // The dialog's own answer is taken as it is: it asked about that file
        assert_eq!(set_path(Path::new(r"C:\a\keys.tapka"), |_| true), Some(r"C:\a\keys.tapka".into()));
        assert_eq!(set_path(Path::new(r"C:\a\keys.TAPKA"), |_| true), Some(r"C:\a\keys.TAPKA".into()));
        // Another name is completed, and the file it makes must be new
        assert_eq!(set_path(Path::new(r"C:\a\keys"), there), Some(r"C:\a\keys.tapka".into()));
        assert_eq!(set_path(Path::new(r"C:\a\keys.txt"), there), Some(r"C:\a\keys.txt.tapka".into()));
        assert_eq!(set_path(Path::new(r"C:\a\notes.txt"), there), None);
    }

    #[test]
    fn a_file_that_is_not_a_key_set_is_refused() {
        let dir = std::env::temp_dir().join(format!("tapka-set-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("keys.tapka");
        let key = r#"{ "name": "a", "action": "hotkey", "keys": "b" }"#;
        let many = vec![key; SET_MAX_ITEMS + 1].join(",");
        for bad in [
            "not json".to_string(),
            "[]".into(),
            r#"{ "items": {} }"#.into(),
            r#"{ "items": [] }"#.into(),
            r#"{ "keys": [] }"#.into(),
            r#"{ "items": ["text"] }"#.into(),
            r#"{ "items": [{ "name": "a", "action": "script" }] }"#.into(),
            format!(r#"{{ "items": [{key}, {{ "action": "open", "target": "x" }}] }}"#),
            format!(r#"{{ "items": [{many}] }}"#),
            // A set in every other way, but too large
            format!(r#"{{ "items": [{key}], "pad": "{}" }}"#, "x".repeat(SET_MAX_BYTES as usize)),
        ] {
            std::fs::write(&file, &bad).unwrap();
            assert_eq!(read_set(&file), None, "{}", &bad[..bad.len().min(60)]);
        }
        assert_eq!(read_set(&dir.join("none.tapka")), None);
        assert_eq!(set_items(&[json!("text")], &[], |_| true), None);
        // Within the limits it is a set, saved with a byte-order mark or not
        std::fs::write(&file, format!("\u{feff}{{ \"items\": [{key}] }}")).unwrap();
        assert_eq!(read_set(&file).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_sound_and_input_actions_survive_the_settings_file() {
        let ids = ["clipboard", "emoji", "volumeup", "volumedown", "mute", "playpause", "nexttrack"];
        let presets: Vec<_> = ids.iter().map(|id| PRESETS.iter().find(|p| p.id == *id).unwrap()).collect();
        let items: Vec<Value> = presets.iter().map(|p| preset_item(p, true)).collect();
        let text = with_field(OWN, "items", json!(items)).unwrap();
        let (s, skipped) = parse(&text).unwrap();
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(s.items.len(), ids.len());
        for (item, preset) in s.items.iter().zip(&presets) {
            assert_eq!((item.name.as_str(), item.icon.as_deref()), (preset.ru.0, Some(preset.icon)));
            assert_eq!(item.action, Action::Hotkey(preset.action.1.into()));
        }
        // Written again, the file does not change
        assert_eq!(to_text(&serde_json::from_str(&text).unwrap()), text);
    }

    #[test]
    fn the_default_file_has_windows_actions_in_the_language_asked_for() {
        let names = |russian, keyboard, pen| -> Vec<String> { parse(&default_text(russian, keyboard, pen)).unwrap().0.items.into_iter().map(|i| i.name).collect() };
        // A tablet without a keyboard or a pen; a keyboard attached; a pen; both
        assert_eq!(names(true, false, false), ["Отменить", "Снимок", "Клавиатура", "Проводник"]);
        assert_eq!(names(true, true, false), ["Отменить", "Снимок", "Повторить", "Проводник"]);
        assert_eq!(names(true, false, true), ["Отменить", "Снимок", "Клавиатура", "Держать Shift"]);
        assert_eq!(names(true, true, true), ["Отменить", "Снимок", "Повторить", "Держать Shift"]);
        assert_eq!(names(false, false, false), ["Undo", "Snip", "Keyboard", "File Explorer"]);
        assert_eq!(names(false, true, true), ["Undo", "Snip", "Redo", "Hold Shift"]);
        // Each set shows whole on the capsule, with nothing to scroll
        for (keyboard, pen) in [(false, false), (true, false), (false, true), (true, true)] {
            assert!(parse(&default_text(true, keyboard, pen)).unwrap().0.items.len() <= crate::layout::SHOWN);
        }
        let text = default_text(false, false, false);
        assert!(text.starts_with("{\n  \"scale\": 1.0,\n  \"top\": 0.3,\n  \"items\": [\n    { \"name\": \"Undo\", \"icon\": \"undo\", "), "{text}");
        let (s, skipped) = parse(&text).unwrap();
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(s.top, DEFAULT_TOP);
        assert_eq!(s.items[0].action, Action::Hotkey("ctrl+z".into()));
        assert_eq!(s.items[1].action, Action::Hotkey("win+shift+s".into()));
        assert_eq!(s.items[2].action, Action::Hotkey("win+ctrl+o".into()));
        assert_eq!(s.items[3].action, Action::Hotkey("win+e".into()));
        let drawing = parse(&default_text(false, true, true)).unwrap().0;
        assert_eq!(drawing.items[2].action, Action::Hotkey("ctrl+y".into()));
        assert_eq!(drawing.items[3].action, Action::Hold("shift".into()));
        // Nothing of any one machine or of another product
        assert!(s.items.iter().all(|i| i.lit.is_none() && i.only_in.is_none()));
    }

    #[test]
    fn an_item_for_one_program_comes_from_the_file_and_survives_a_save() {
        let text = OWN.replace(
            r#"{ "name": "Later""#,
            r#"{ "name": "Brush", "action": "hotkey", "keys": "b", "only_in": "Paint.exe" },
    { "name": "Later""#,
        );
        let (s, skipped) = parse(&text).unwrap();
        assert_eq!(skipped.len(), 1, "{skipped:?}");
        assert_eq!(s.items[0].only_in, None);
        assert_eq!(s.items[1].only_in.as_deref(), Some("paint.exe"));
        // An empty or odd value shows the item everywhere
        for odd in [r#""""#, r#""  ""#, "5", "null"] {
            let item = parse_item(&serde_json::from_str(&format!(r#"{{ "name": "a", "action": "hotkey", "keys": "b", "only_in": {odd} }}"#)).unwrap()).unwrap();
            assert_eq!(item.only_in, None, "{odd}");
        }
        // The editor hands the items back as the file has them: the field stays, written as it was
        let items = serde_json::from_str::<Value>(&text).unwrap()["items"].clone();
        let saved = with_field(&text, "items", items).unwrap();
        assert_eq!(saved, text);
        assert!(saved.contains(r#""only_in": "Paint.exe""#));
    }

    #[test]
    fn a_held_key_comes_from_the_file_and_the_editor_writes_it_back() {
        let text = OWN.replace(
            r#"{ "name": "Later""#,
            r#"{ "name": "Shift", "icon": "shift", "action": "hold", "keys": "Shift", "only_in": "sketch.exe" },
    { "name": "Later""#,
        );
        let (s, skipped) = parse(&text).unwrap();
        assert_eq!(skipped.len(), 1, "{skipped:?}");
        assert_eq!(s.items[1].action, Action::Hold("shift".into()));
        assert_eq!(s.items[1].only_in.as_deref(), Some("sketch.exe"));
        // Only the three modifiers and Space are held; nothing else makes an item
        for bad in ["win", "ctrl+shift", "a", ""] {
            let item = serde_json::from_str(&format!(r#"{{ "name": "a", "action": "hold", "keys": "{bad}" }}"#)).unwrap();
            assert!(parse_item(&item).is_err(), "{bad}");
        }
        // The editor adds the four ready ones as the file has them, and the file reads back the same
        let held: Vec<Value> = ["holdshift", "holdctrl", "holdalt", "holdspace"]
            .iter()
            .map(|id| preset_item(PRESETS.iter().find(|p| p.id == *id).unwrap(), false))
            .collect();
        assert_eq!(inline(&held[0]), r#"{ "name": "Hold Shift", "icon": "shift", "hint": "Until tapped again; lets go by itself after 20 s without taps", "action": "hold", "keys": "shift" }"#);
        let saved = with_field(&text, "items", json!(held)).unwrap();
        let keys: Vec<Action> = parse(&saved).unwrap().0.items.into_iter().map(|i| i.action).collect();
        assert_eq!(keys, ["shift", "ctrl", "alt", "space"].map(|k| Action::Hold(k.into())));
        assert_eq!(to_text(&serde_json::from_str(&saved).unwrap()), saved);
        // The hint says what the panel does
        assert!(HOLD_HINT.1.contains(&format!("{} s", crate::actions::HOLD_IDLE_MS / 1000)));
        assert!(HOLD_HINT.0.contains(&format!("{} с", crate::actions::HOLD_IDLE_MS / 1000)));
    }
}
