#![windows_subsystem = "windows"]

mod actions;
mod apps;
mod autostart;
mod config;
mod glyphs;
mod grab;
mod keys;
mod layout;
mod shellicon;
mod win;
#[cfg(test)]
mod stories;

use base64::Engine;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

struct AppState {
    settings: Mutex<config::Settings>,
    /// Why the settings file was rejected, until the next successful read.
    error: Mutex<Option<String>>,
    /// The name label is on screen.
    card_open: AtomicBool,
    /// Where the open label is inside the window, physical px, shadow included.
    card_rect: Mutex<(i32, i32, i32, i32)>,
    /// The keyboard is detached: items stand wider apart and hover shows no card.
    tablet: AtomicBool,
    /// Physical px per CSS px of the capsule page: monitor scale times panel scale.
    px_per_css: Mutex<f64>,
    /// The capsule is being carried by its handle: what the carry needs, measured when it starts.
    carry: Mutex<Option<Carry>>,
    /// For each item: the program it opens has a window, and its `lit` window is showing.
    marks: Mutex<Vec<(bool, bool)>>,
    /// The program of the active window, as `win::front_program` last saw it: which items with
    /// `only_in` are on the capsule.
    front: Mutex<Option<(String, Option<String>)>>,
    /// The programs whose items the capsule has already introduced since the start, by file name.
    introduced: Mutex<Vec<String>>,
    /// This start created the settings file: the settings window opens once, with a few words
    /// about the panel.
    intro: AtomicBool,
    /// What is known about a newer release.
    update: Mutex<Update>,
    /// The capsule is meant to be on screen: the keyboard rule, the tray and a second launch say
    /// so. A snapshot takes the capsule off the screen without changing this.
    wanted: AtomicBool,
    /// The capsule is off the screen for a snapshot and comes back by itself.
    snapping: AtomicBool,
    /// One change of the capsule's visibility at a time: the tray, the keyboard rule and a
    /// snapshot's return come from different threads, and the capsule and its handle must end up
    /// the same way. Showing and hiding only post a request to the windows and wait for nothing,
    /// so this lock is safe to hold across them.
    showing: Mutex<()>,
    /// The keys `hold` items keep pressed, and when the capsule was last tapped.
    holds: Mutex<actions::Holds>,
    /// The two signs show in the capsule's foot in place of the plus: the capsule is one sign longer.
    signs: AtomicBool,
    /// Counts the changes of the items: an answer about an item tapped before the last change
    /// names a place that may hold another item by now.
    items_age: std::sync::atomic::AtomicU32,
}

/// The panel never says it is the latest version before a check has passed.
#[derive(Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
enum Update {
    None,
    Checking,
    Latest,
    Found { version: String },
    Failed { why: String },
    Loading,
}

const GITHUB: &str = "https://github.com/Sargares22/tapka";

/// The panel speaks Russian or English: what the settings say, otherwise what Windows speaks.
fn is_russian(settings: &config::Settings) -> bool {
    match settings.lang {
        config::Lang::Ru => true,
        config::Lang::En => false,
        config::Lang::System => win::system_is_russian(),
    }
}

/// The log is cut off at this size: the full file becomes `panel.old.log` and a new one starts.
const LOG_LIMIT: u64 = 1024 * 1024;

/// The data folder: settings, icons and the log. `TAPKA_DATA_DIR` moves it, which is how a first
/// run is tried out without touching one's own settings.
fn data_dir(app: &AppHandle) -> Option<std::path::PathBuf> {
    match std::env::var_os("TAPKA_DATA_DIR").filter(|dir| !dir.is_empty()) {
        Some(dir) => Some(dir.into()),
        None => app.path().app_data_dir().ok(),
    }
}

fn log_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = data_dir(app)?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("panel.log"))
}

fn applog(app: &AppHandle, line: &str) {
    let Some(path) = log_path(app) else { return };
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_LIMIT) {
        let _ = std::fs::rename(&path, path.with_file_name("panel.old.log"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(f, "{now} {line}");
    }
}

/// An item as the page draws it.
#[derive(serde::Serialize)]
struct PageItem {
    /// Its place in the settings file's list: what a tap reports.
    at: usize,
    name: String,
    hint: Option<String>,
    /// A picture as a data URL: the user's own file or the program's icon.
    icon: Option<String>,
    /// A built-in icon as SVG text, drawn in the ink of the theme.
    glyph: Option<&'static str>,
    /// The program this item opens has a window right now.
    on: bool,
    /// The window this item watches for (`lit` in the settings) is showing: recording, say.
    live: bool,
    /// The key this `hold` item keeps pressed is down.
    held: bool,
}

/// An item's picture: `(data URL, built-in SVG)`, at most one of them. In order: the built-in the
/// item names; the PNG or SVG it names in `icons` of the data folder; for an `open` item, the
/// icon Windows shows for its program, file or folder. With none of these the item shows its
/// letter.
fn item_icon(app: &AppHandle, item: &config::Item) -> (Option<String>, Option<&'static str>) {
    let b64 = |bytes: &[u8]| base64::engine::general_purpose::STANDARD.encode(bytes);
    if let Some(file) = item.icon.as_deref() {
        if let Some(svg) = glyphs::builtin(file) {
            return (None, Some(svg));
        }
        let mime = match file.rsplit_once('.').map(|(_, ext)| ext.to_lowercase()).as_deref() {
            Some("png") => Some("image/png"),
            Some("svg") => Some("image/svg+xml"),
            _ => None,
        };
        let read = data_dir(app).map(|dir| std::fs::read(dir.join("icons").join(file)));
        match (mime, read) {
            (Some(mime), Some(Ok(bytes))) => return (Some(format!("data:{mime};base64,{}", b64(&bytes))), None),
            (Some(_), Some(Err(why))) => applog(app, &format!("icon {file}: {why}")),
            _ => {}
        }
    }
    let config::Action::Open(target) = &item.action else { return (None, None) };
    (shell_icon(target), None)
}

/// The icon Windows shows for a target, as a data URL. Asking the shell takes a moment, and the
/// pages ask on every redraw, so each target is asked once.
fn shell_icon(target: &str) -> Option<String> {
    static SEEN: Mutex<Vec<(String, Option<String>)>> = Mutex::new(Vec::new());
    if let Some((_, icon)) = SEEN.lock().unwrap().iter().find(|(t, _)| t == target) {
        return icon.clone();
    }
    let icon = shellicon::png(target, 64)
        .map(|bytes| format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)));
    SEEN.lock().unwrap().push((target.to_string(), icon.clone()));
    icon
}

/// A carry in progress, measured once when it starts. All in physical px.
#[derive(Clone, Copy)]
struct Carry {
    /// The capsule window.
    hwnd: isize,
    /// The capsule window's corner minus the handle window's corner: kept constant while carried.
    offset: (i32, i32),
    /// The work area the capsule must stay in.
    work: (i32, i32, i32, i32),
    /// Where the capsule's strip sits inside its window.
    strip: (i32, i32, i32, i32),
    /// What the slot needs to draw the capsule at its landing place.
    k: f64,
    cell: f64,
    items: usize,
    signs: bool,
    /// The capsule is folded: what is carried, and what lands, is the folded key.
    folded: bool,
}
/// The windows on the desktop that belong to an item's program, front to back. Only `open` items
/// that name an exe or a Store app have any.
fn windows_of<'a>(item: &config::Item, windows: &'a [win::AppWindow]) -> Vec<&'a win::AppWindow> {
    let config::Action::Open(target) = &item.action else { return Vec::new() };
    windows.iter().filter(|w| actions::belongs(target, &w.exe, w.family.as_deref())).collect()
}

/// The items on the capsule now, as places in the settings' list: those for all programs, then
/// those for the active one (`actions::shown_items`).
fn shown_now(app: &AppHandle, items: &[config::Item]) -> Vec<usize> {
    let front = app.state::<AppState>().front.lock().unwrap().clone();
    actions::shown_items(items, program_ref(&front))
}

fn program_ref(program: &Option<(String, Option<String>)>) -> Option<(&str, Option<&str>)> {
    program.as_ref().map(|(exe, family)| (exe.as_str(), family.as_deref()))
}

/// The programs in the Start menu. Listing them takes a moment, so it is done once, and again
/// each time the editor lists them for a new item.
static INSTALLED: Mutex<Option<Vec<apps::App>>> = Mutex::new(None);

fn installed_known() -> Vec<apps::App> {
    if let Some(apps) = INSTALLED.lock().unwrap().clone() {
        return apps;
    }
    let apps = apps::installed();
    *INSTALLED.lock().unwrap() = Some(apps.clone());
    apps
}

/// What the editor calls the program an `only_in` names: the name of its running window's
/// program if it has one, otherwise the Start menu's name for it, otherwise its file name.
fn program_label(only_in: &str, windows: &[win::AppWindow], apps: &[apps::App]) -> String {
    match windows.iter().find(|w| actions::runs_in(only_in, &w.exe, w.family.as_deref())) {
        Some(w) => apps::name_of(apps, &w.exe, w.family.as_deref()),
        None if only_in.ends_with(".exe") => apps::name_of(apps, only_in.rsplit(['\\', '/']).next().unwrap_or(only_in), None),
        None => apps::name_of(apps, "", Some(only_in)),
    }
}

/// For each item: whether its program has a window, and whether the window it watches for is
/// showing. Looks at the desktop; never call it with a lock held.
fn item_marks(items: &[config::Item]) -> Vec<(bool, bool)> {
    let windows = win::app_windows();
    let wanted: Vec<(String, String)> = items.iter().filter_map(|it| it.lit.clone()).collect();
    let mut shown = win::shown(&wanted).into_iter();
    items
        .iter()
        .map(|it| (!windows_of(it, &windows).is_empty(), it.lit.is_some() && shown.next().unwrap_or(false)))
        .collect()
}

/// Everything the page needs to draw the capsule.
#[derive(serde::Serialize)]
struct View {
    items: Vec<PageItem>,
    /// Height of one item's cell in CSS px: the step from key to key.
    cell: f64,
    tablet: bool,
    /// The edge the capsule is docked to: `right`, `left` or `top`. The page lays itself out for it.
    edge: &'static str,
    /// `system`, `dark` or `light`.
    theme: &'static str,
    accent: String,
    /// A quiet click on a tap.
    click_sound: bool,
    /// The name and the hint of the plus in the capsule's foot, which opens the two signs.
    add: (&'static str, &'static str),
    /// The two signs: the gear opens the settings window, the arrow to the edge folds the capsule.
    gear: Sign,
    fold: Sign,
    /// The arrow on the folded key, which points away from the edge.
    unfold: &'static str,
    /// The capsule is folded into one key.
    folded: bool,
    /// Length of the pill in CSS px, with the plus in its foot and with the two signs: the page
    /// draws it this long whatever the window is at the moment (`layout::pill_length`).
    pill: Option<(f64, f64)>,
}

/// A sign of the capsule's foot as the page draws it: its name, its hint and its icon.
#[derive(serde::Serialize)]
struct Sign {
    name: &'static str,
    hint: &'static str,
    glyph: &'static str,
}

#[tauri::command]
fn get_view(app: AppHandle) -> View {
    let state = app.state::<AppState>();
    let tablet = state.tablet.load(Ordering::Relaxed);
    // The settings are copied out: no lock is held while files are read or the desktop is looked at
    let settings = state.settings.lock().unwrap().clone();
    let marks = item_marks(&settings.items);
    let held = held_items(&settings.items, state.holds.lock().unwrap().held());
    let items: Vec<PageItem> = shown_now(&app, &settings.items)
        .into_iter()
        .map(|at| {
            let it = &settings.items[at];
            let (icon, glyph) = item_icon(&app, it);
            let (on, live) = marks[at];
            PageItem { at, on, live, held: held.contains(&at), name: it.name.clone(), hint: it.hint.clone(), icon, glyph }
        })
        .collect();
    let russian = is_russian(&settings);
    let add = if russian { ("Добавить", "Настройки и сворачивание капсулы") } else { ("Add", "Settings and folding the capsule") };
    let (gear, fold) = if russian {
        (("Настройки", "Открыть окно настроек"), ("Свернуть", "Свернуть капсулу в одну клавишу"))
    } else {
        (("Settings", "Open the settings window"), ("Fold", "Fold the capsule into one key"))
    };
    // The arrows are the built-in ones: to the edge to fold, away from it on the folded key
    let (to_edge, from_edge) = match settings.edge {
        layout::Edge::Right => ("right", "left"),
        layout::Edge::Left => ("left", "right"),
        layout::Edge::Top => ("up", "down"),
    };
    let glyph = |name| glyphs::builtin(name).unwrap_or_default();
    let cell = layout::cell(tablet);
    let pill = work_area(&app).map(|(work, k)| {
        let long = |signs| layout::pill_length(work, k, cell, items.len(), settings.edge, signs);
        (long(false), long(true))
    });
    View {
        pill,
        gear: Sign { name: gear.0, hint: gear.1, glyph: glyph("gear") },
        fold: Sign { name: fold.0, hint: fold.1, glyph: glyph(to_edge) },
        unfold: glyph(from_edge),
        folded: settings.folded,
        items,
        cell,
        tablet,
        edge: config::edge_name(settings.edge),
        theme: config::theme_name(settings.theme),
        accent: settings.accent.clone(),
        click_sound: settings.click_sound,
        add,
    }
}

/// The places in the settings' list of the `hold` items whose key is down: they are lit.
fn held_items(items: &[config::Item], held: &[u16]) -> Vec<usize> {
    let down = |item: &config::Item| match &item.action {
        config::Action::Hold(key) => keys::holdable(key).is_some_and(|vk| held.contains(&vk)),
        _ => false,
    };
    (0..items.len()).filter(|&i| down(&items[i])).collect()
}

/// Tells the page which items hold their key down now.
fn show_held(app: &AppHandle) {
    let state = app.state::<AppState>();
    let held = state.holds.lock().unwrap().held().to_vec();
    let lit = held_items(&state.settings.lock().unwrap().items, &held);
    let _ = app.emit("held", lit);
}

/// A tap on a `hold` item: its key goes down, or up if it was held. The key is sent with the holds
/// locked (`actions::Holds`); there is no keyboard hook in the panel for SendInput to wait on. A
/// key Windows refused changes nothing: its item is lit as it was.
fn hold(app: &AppHandle, key: &str) -> Result<(), String> {
    let vk = keys::holdable(key).ok_or(format!("cannot hold \"{key}\""))?;
    let mut sent = Ok(());
    let down = app.state::<AppState>().holds.lock().unwrap().toggle(vk, |vk, up| {
        sent = win::send_key(vk, up);
        sent.is_ok()
    });
    if let Some(down) = down {
        applog(app, &format!("hold: {key} {}", if down { "down" } else { "up" }));
        show_held(app);
    }
    sent
}

/// Lets go of held keys, every one of them or, with `idle`, only once the wait for a tap has run
/// out, and puts their items out. The keys go up with the holds locked, as in `hold`. A key
/// Windows would not let go of stays held, and its item stays lit. Returns whether no key is
/// left held.
fn let_go(app: &AppHandle, why: &str, idle: bool) -> bool {
    let mut failed = Vec::new();
    let send = |vk: u16, up: bool| match win::send_key(vk, up) {
        Ok(()) => true,
        Err(e) => {
            failed.push(format!("action failed: let go of key {vk:#x} ({why}): {e}"));
            false
        }
    };
    let state = app.state::<AppState>();
    let (vks, none_left) = {
        let mut holds = state.holds.lock().unwrap();
        if idle {
            let vks = holds.expire(std::time::Instant::now(), send);
            (vks, holds.held().is_empty())
        } else {
            holds.release_to_fold(send)
        }
    };
    for line in failed {
        applog(app, &line);
    }
    if vks.is_empty() {
        return none_left;
    }
    applog(app, &format!("hold: let go of {} key(s): {why}", vks.len()));
    show_held(app);
    none_left
}

fn let_go_all(app: &AppHandle, why: &str) -> bool {
    let_go(app, why, false)
}

/// Hides the capsule and brings it back when the clipboard changes or the wait runs out.
/// Returns once the capsule is off the screen, so the keys that follow do not catch it, with the
/// number of this hide, or with none when there was nothing to hide.
fn hide_capsule_until_snapshot(app: &AppHandle) -> Result<Option<u32>, String> {
    let w = app.get_webview_window("capsule").ok_or("no capsule window")?;
    let state = app.state::<AppState>();
    let hwnd = w.hwnd().map_err(|e| e.to_string())?.0 as isize;
    let_go_all(app, "the capsule hid for a snapshot");
    let mine = {
        let _turn = state.showing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !state.wanted.load(Ordering::Relaxed) {
            return Ok(None); // already hidden, from the tray or by the keyboard: nothing to bring back
        }
        state.snapping.store(true, Ordering::Relaxed);
        win::set_visible(hwnd, false);
        grab::set_visible(false);
        HIDES.fetch_add(1, Ordering::Relaxed) + 1
    };
    let app = app.clone();
    std::thread::spawn(move || {
        // The selection belongs to the Snipping Tool; while it is up, that program is in front
        let hidden = std::time::Instant::now();
        let mut snip = actions::Snip::NotYet;
        while !actions::capsule_returns(snip, hidden.elapsed().as_millis() as u64) {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let snipping = matches!(win::foreground_program().as_deref(), Some("snippingtool.exe" | "screenclippinghost.exe"));
            snip = match (snip, snipping) {
                (_, true) => actions::Snip::Open,
                (actions::Snip::Open, false) => actions::Snip::Closed,
                (other, false) => other,
            };
        }
        back_from_snapshot(&app, hwnd, mine);
    });
    std::thread::sleep(std::time::Duration::from_millis(150));
    Ok(Some(mine))
}

/// Counts the hides for a snapshot. A second snapshot before the first wait ran out: only the
/// latest wait shows the capsule.
static HIDES: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// The snapshot that hid the capsule, the hide numbered `mine`, is over or never began: the
/// capsule comes back, folded if it was folded. Whoever comes second for the same hide, the wait
/// or a failed send, finds nothing to do.
fn back_from_snapshot(app: &AppHandle, hwnd: isize, mine: u32) {
    let state = app.state::<AppState>();
    let _turn = state.showing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if !actions::ends_hide(HIDES.load(Ordering::Relaxed), mine, state.snapping.load(Ordering::Relaxed)) {
        return;
    }
    state.snapping.store(false, Ordering::Relaxed);
    // Hidden from the tray or by the keyboard in the meantime: it stays hidden
    if state.wanted.load(Ordering::Relaxed) {
        win::set_visible(hwnd, true);
        grab::set_visible(true);
    }
}

/// Performs a step with real Windows calls. A failure goes to the log and comes back as what the
/// capsule says about it. A step that hid the capsule for a snapshot leaves the number of its
/// hide in `hid`.
fn run_step(app: &AppHandle, step: actions::Step, hid: &mut Option<u32>) -> Option<actions::Failure> {
    use actions::Failure;
    // Keys go to the active window. If its program has rights above the panel's, Windows may drop
    // them and say nothing, so the window is looked at before they go. The look only explains:
    // the keys are sent all the same.
    let to_admin = matches!(step, actions::Step::SendKeys(_) | actions::Step::Hold(_)) && win::rights_above(win::front_rights(), win::own_rights());
    let result = match &step {
        actions::Step::Open(target) => win::open(target).map_err(|e| (Failure::Open, e)),
        // Sent with the holds locked, so a held key cannot come up between the look and the send
        actions::Step::SendKeys(keys) => keys::parse(keys)
            .and_then(|vks| {
                let state = app.state::<AppState>();
                let holds = state.holds.lock().unwrap();
                win::send_keys(&actions::without_held(&vks, holds.held()))
            })
            .map_err(|e| (Failure::Keys, e)),
        actions::Step::Hold(key) => hold(app, key).map_err(|e| (Failure::Keys, e)),
        // This fails only when there is no capsule window, and then no page to tell
        actions::Step::HideCapsule => hide_capsule_until_snapshot(app).map(|mine| *hid = mine).map_err(|e| (Failure::Keys, e)),
        actions::Step::Show(target) => {
            let windows = win::app_windows();
            match windows.iter().find(|w| actions::belongs(target, &w.exe, w.family.as_deref())) {
                Some(w) => win::bring_to_front(w.hwnd),
                None => win::open(target), // it closed between the tap and now
            }
            .map_err(|e| (Failure::Open, e))
        }
        actions::Step::Minimize(target) => {
            for w in win::app_windows().iter().filter(|w| actions::belongs(target, &w.exe, w.family.as_deref())) {
                win::minimize(w.hwnd);
            }
            Ok(())
        }
    };
    match result {
        Err((failure, why)) => {
            applog(app, &format!("action failed: {step:?}: {why}"));
            Some(failure)
        }
        Ok(()) if to_admin => {
            applog(app, &format!("action failed: {step:?}: the active window runs as administrator"));
            Some(Failure::Admin)
        }
        Ok(()) => None,
    }
}

/// Runs the steps of a tap on the item at `index` in the settings' list. A step that failed is
/// told to the page, which says so beside the item's key. `age` is `items_age` at the tap: if the
/// items have changed since, the place may hold another item, and nothing is said. A capsule
/// hidden for a snapshot whose keys then failed comes back first: there is no selection to wait
/// for, and the words would be gone before it returned.
fn run_steps(app: &AppHandle, index: usize, age: u32, steps: Vec<actions::Step>) {
    let state = app.state::<AppState>();
    // The hide this tap made, if it made one: a failure ends that hide and no other tap's
    let mut hid = None;
    for step in steps {
        let Some(failure) = run_step(app, step, &mut hid) else { continue };
        // Keys that were sent, to a window that may have dropped them, did not fail to go
        let unsent = hid.filter(|_| failure != actions::Failure::Admin);
        if let (Some(mine), Some(hwnd)) = (unsent, app.get_webview_window("capsule").and_then(|w| w.hwnd().ok())) {
            back_from_snapshot(app, hwnd.0 as isize, mine);
        }
        if state.items_age.load(Ordering::Relaxed) != age {
            continue;
        }
        let russian = is_russian(&state.settings.lock().unwrap());
        let (title, why) = failure_words(failure, russian);
        let _ = app.emit("failed", json!({ "at": index, "title": title, "why": why }));
    }
}

/// What the capsule says beside a key that did not work: two lines, in the panel's language.
fn failure_words(failure: actions::Failure, russian: bool) -> (&'static str, &'static str) {
    use actions::Failure;
    match (failure, russian) {
        (Failure::Keys, true) => ("Не сработало", "Windows не приняла нажатие"),
        (Failure::Open, true) => ("Не сработало", "Не открылось"),
        (Failure::Admin, true) => ("Не сработало", "Это окно запущено от администратора"),
        (Failure::Keys, false) => ("Didn't work", "Windows did not accept the key press"),
        (Failure::Open, false) => ("Didn't work", "Couldn't open it"),
        (Failure::Admin, false) => ("Didn't work", "This window runs as administrator"),
    }
}

/// The page reports a tap on the item at `index` in the settings' list, or with no index on the
/// gear in the capsule's foot, which opens the editor. The action runs off the main thread.
#[tauri::command]
fn tap(app: AppHandle, index: Option<usize>, pointer: String) {
    app.state::<AppState>().holds.lock().unwrap().touch(std::time::Instant::now());
    let Some(index) = index else {
        applog(&app, &format!("tap: settings pointer={pointer}"));
        // A window made from inside a command on the main thread would wait for itself
        std::thread::spawn(move || open_settings_window(&app));
        return;
    };
    let age = app.state::<AppState>().items_age.load(Ordering::Relaxed);
    let item = app.state::<AppState>().settings.lock().unwrap().items.get(index).cloned();
    let Some(item) = item else { return };
    applog(&app, &format!("tap: item {} \"{}\" pointer={pointer}", index + 1, item.name));
    // A key to hold goes down or up here, on the thread of the taps, so quick taps keep their order
    if matches!(item.action, config::Action::Hold(_)) {
        run_steps(&app, index, age, actions::steps_for(&item.action, actions::Running::No));
        return;
    }
    std::thread::spawn(move || {
        let windows = win::app_windows();
        let mine = windows_of(&item, &windows);
        let front = win::foreground();
        let running = if mine.is_empty() {
            actions::Running::No
        } else if mine.iter().any(|w| w.hwnd == front) {
            actions::Running::Front
        } else {
            actions::Running::Behind
        };
        run_steps(&app, index, age, actions::steps_for(&item.action, running));
    });
}

/// The page opened or closed the two signs in the capsule's foot: the capsule is one sign longer
/// while they show.
#[tauri::command]
fn foot(app: AppHandle, open: bool) {
    let state = app.state::<AppState>();
    if open {
        state.holds.lock().unwrap().touch(std::time::Instant::now());
    }
    if state.signs.swap(open, Ordering::Relaxed) == open {
        return;
    }
    let settings = state.settings.lock().unwrap().clone();
    place_capsule(&app, &settings);
}

/// Folds or unfolds the capsule, or puts the folded key at `at` along its edge: in the settings,
/// in the settings file, so it survives a restart, and on the screen.
fn set_fold(app: &AppHandle, folded: bool, at: Option<f64>) {
    let state = app.state::<AppState>();
    // As the file will have it, so reading the file back changes nothing
    let at = at.map(|at| config::share(at).as_f64().unwrap_or(at));
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.folded = folded;
        s.folded_at = at.unwrap_or(s.folded_at);
        s.clone()
    };
    let saved = change_settings(app, |text| {
        let text = config::with_field(text, "folded", json!(folded))?;
        match at {
            Some(at) => config::with_field(&text, "folded_at", config::share(at)),
            None => Some(text),
        }
    });
    if let Err(why) = saved {
        applog(app, &format!("settings: cannot save the fold: {why}"));
    }
    applog(app, &format!("capsule: {}, along {:.3}", if folded { "folded" } else { "unfolded" }, settings.folded_at));
    state.signs.store(false, Ordering::Relaxed);
    place_capsule(app, &settings);
    let _ = app.emit("reload", ());
}

/// A tap on the fold sign: the capsule becomes one key whose middle is where the sign was. Every
/// held key is let go first; if one would not come up, the capsule stays open and says so.
#[tauri::command]
fn fold(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.holds.lock().unwrap().touch(std::time::Instant::now());
    if !let_go_all(&app, "the capsule folds") {
        let russian = is_russian(&state.settings.lock().unwrap());
        let (title, why) = failure_words(actions::Failure::Keys, russian);
        let _ = app.emit("failed", json!({ "at": null, "title": title, "why": why }));
        return Err("a held key would not come up".into());
    }
    let settings = state.settings.lock().unwrap().clone();
    let (work, k) = work_area(&app).ok_or("no screen")?;
    let cell = layout::cell(state.tablet.load(Ordering::Relaxed));
    let shown = shown_now(&app, &settings.items).len();
    let open = layout::window_rect_with(work, k, settings.top, cell, shown, settings.edge, true);
    let at = layout::along_from(work, k, layout::fold_spot(open, k, settings.edge), settings.edge);
    set_fold(&app, true, Some(at));
    Ok(())
}

/// A touch of the folded key that reached the page: the capsule is back at its own place.
#[tauri::command]
fn unfold(app: AppHandle) {
    app.state::<AppState>().holds.lock().unwrap().touch(std::time::Instant::now());
    set_fold(&app, false, None);
}

/// Cuts the window down to what should be visible and take presses: the capsule's strip, plus the
/// label's own rectangle while the label is open. The window itself keeps its size. Of the window
/// of a folded capsule only the pill with its key is left: the room around it, where its shadow
/// was, would take the presses meant for what is under it.
fn apply_region(app: &AppHandle, size: (i32, i32), edge: layout::Edge) {
    let Some(hwnd) = app.get_webview_window("capsule").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize) else {
        return;
    };
    let state = app.state::<AppState>();
    let folded = state.settings.lock().unwrap().folded;
    let k = *state.px_per_css.lock().unwrap();
    if folded {
        win::set_region(hwnd, &[layout::folded_region(size, k, edge)]);
        return;
    }
    let open = state.card_open.load(Ordering::Relaxed) && state.carry.lock().unwrap().is_none();
    let strip = layout::strip(size, k, edge);
    if open {
        win::set_region(hwnd, &[strip, *state.card_rect.lock().unwrap()]);
    } else {
        win::set_region(hwnd, &[strip]);
    }
}

/// The page opens or closes the name label. Its room is already part of the window; only the
/// label's own rectangle, `rect` as `[x, y, width, height]` in CSS px, is let through.
#[tauri::command]
fn card(app: AppHandle, open: bool, rect: Option<[f64; 4]>) {
    let state = app.state::<AppState>();
    if let Some([x, y, w, h]) = rect {
        let k = *state.px_per_css.lock().unwrap();
        let px = |v: f64| (v * k).round() as i32;
        *state.card_rect.lock().unwrap() = (px(x), px(y), px(w), px(h));
    }
    state.card_open.store(open && rect.is_some(), Ordering::Relaxed);
    let Some(w) = app.get_webview_window("capsule") else { return };
    let edge = app.state::<AppState>().settings.lock().unwrap().edge;
    if let Ok(size) = w.outer_size() {
        apply_region(&app, (size.width as i32, size.height as i32), edge);
    }
}

/// Sizes the capsule window for the current settings and docks it to its edge of the primary
/// monitor's work area.
fn place_capsule(app: &AppHandle, settings: &config::Settings) {
    let Some(w) = app.get_webview_window("capsule") else { return };
    let Ok(Some(mon)) = w.primary_monitor() else { return };
    let wa = mon.work_area();
    let work = (wa.position.x, wa.position.y, wa.size.width as i32, wa.size.height as i32);
    let state = app.state::<AppState>();
    if state.carry.lock().unwrap().is_some() {
        return; // the handle has the capsule; it is placed again when the drag ends
    }
    let cell = layout::cell(state.tablet.load(Ordering::Relaxed));
    let k = mon.scale_factor() * settings.scale;
    let shown = shown_now(app, &settings.items).len();
    let (x, y, width, height) = if settings.folded {
        layout::folded_window(work, k, settings.folded_at, settings.edge)
    } else {
        layout::window_rect_with(work, k, settings.top, cell, shown, settings.edge, state.signs.load(Ordering::Relaxed))
    };
    let changed = *state.px_per_css.lock().unwrap() != k;
    *state.px_per_css.lock().unwrap() = k;
    if changed {
        // The panel scale is applied as page zoom
        let _ = w.set_zoom(settings.scale);
    }
    let _ = w.set_size(tauri::PhysicalSize::new(width, height));
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
    apply_region(app, (width, height), settings.edge);
    // Open, the capsule is carried by its handle; folded, by the folded key itself
    grab::place(if settings.folded {
        layout::folded_grip(work, k, settings.folded_at, settings.edge)
    } else {
        layout::grip_on_screen((x, y, width, height), k, settings.edge)
    });
    let _turn = state.showing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    grab::set_visible(state.wanted.load(Ordering::Relaxed) && !state.snapping.load(Ordering::Relaxed));
}

/// Work area of the primary monitor, `(x, y, width, height)` in physical px, and px per CSS px.
fn work_area(app: &AppHandle) -> Option<((i32, i32, i32, i32), f64)> {
    let w = app.get_webview_window("capsule")?;
    let mon = w.primary_monitor().ok()??;
    let wa = mon.work_area();
    let k = *app.state::<AppState>().px_per_css.lock().unwrap();
    Some(((wa.position.x, wa.position.y, wa.size.width as i32, wa.size.height as i32), k))
}

/// The capsule's strip inside its window right now, `(x, y, width, height)` in physical px from
/// the window's corner.
fn strip_now(app: &AppHandle) -> Option<(i32, i32, i32, i32)> {
    let w = app.get_webview_window("capsule")?;
    let size = w.outer_size().ok()?;
    let state = app.state::<AppState>();
    let k = *state.px_per_css.lock().unwrap();
    let edge = state.settings.lock().unwrap().edge;
    Some(layout::strip((size.width as i32, size.height as i32), k, edge))
}

/// Windows has started carrying the handle window (`grab`): the capsule, or the folded key, comes
/// off its edge and keeps its place relative to the handle. Everything the carry needs is measured here, once.
fn carry_begin(app: &AppHandle, handle: isize) {
    let state = app.state::<AppState>();
    let (Some(w), Some((work, k)), Some(strip)) = (app.get_webview_window("capsule"), work_area(app), strip_now(app)) else {
        return;
    };
    let Ok(hwnd) = w.hwnd().map(|h| h.0 as isize) else { return };
    let (Some(capsule), Some(grip)) = (win::window_rect(hwnd), win::window_rect(handle)) else { return };
    let (items, folded) = {
        let s = state.settings.lock().unwrap();
        (s.items.clone(), s.folded)
    };
    let (cell, items) = (layout::cell(state.tablet.load(Ordering::Relaxed)), shown_now(app, &items).len());
    let offset = (capsule.0 - grip.0, capsule.1 - grip.1);
    let signs = state.signs.load(Ordering::Relaxed);
    *state.carry.lock().unwrap() = Some(Carry { hwnd, offset, work, strip, k, cell, items, signs, folded });
    state.card_open.store(false, Ordering::Relaxed);
    let _ = app.emit("carry", ());
}

/// Windows proposes to move the handle window to `(x, y)`. The capsule goes along, kept inside
/// the work area, and the slot shows where it would dock if let go now. Returns where the handle
/// may actually go.
fn carry_step(app: &AppHandle, x: i32, y: i32) -> (i32, i32) {
    let Some(c) = *app.state::<AppState>().carry.lock().unwrap() else { return (x, y) };
    let ((wx, wy, ww, wh), (sx, sy, sw, sh), (ox, oy)) = (c.work, c.strip, c.offset);
    let cx = (x + ox).clamp(wx - sx, (wx + ww - sx - sw).max(wx - sx));
    let cy = (y + oy).clamp(wy - sy, (wy + wh - sy - sh).max(wy - sy));
    win::move_window(c.hwnd, cx, cy);
    let on_screen = (cx + sx, cy + sy, sw, sh);
    let edge = layout::drop_edge(c.work, c.k, on_screen);
    let along = layout::along_from(c.work, c.k, on_screen, edge);
    let landing = if c.folded {
        layout::folded_window(c.work, c.k, along, edge)
    } else {
        layout::window_rect_with(c.work, c.k, along, c.cell, c.items, edge, c.signs)
    };
    grab::show_slot(layout::pill_on_screen(landing, c.k, edge), (layout::RADIUS * c.k).round() as i32, c.hwnd);
    (cx - ox, cy - oy)
}

/// Changes the settings file: reads it, lets `change` make the new text, and replaces the file in
/// one step, so the watcher never reads it half written. One change at a time: the settings
/// window, a carry and the editor may all save at the same moment, and none may write over what
/// another has just saved.
fn change_settings(app: &AppHandle, change: impl FnOnce(&str) -> Option<String>) -> Result<(), String> {
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = settings_path(app).ok_or("no settings path")?;
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let text = change(&text).ok_or("the settings file is not a JSON object")?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, &path)).map_err(|e| e.to_string())
}

/// A press on the handle window is over. `entered` says that Windows began to carry the window,
/// `moved` is the farthest the window got, in physical px. A touch of the folded key, a press
/// that carried it nowhere, brings the capsule back. Otherwise the drag ended and the capsule
/// docks: dropped close to the top of the screen it lies down along the top edge; otherwise it
/// goes to the nearer of the left and right edges at the height it was dropped at. The edge and
/// the place along it, of the capsule or of the folded key, go into the settings file, so they
/// survive a restart.
fn carry_end(app: &AppHandle, entered: bool, moved: i32) {
    let state = app.state::<AppState>();
    let carried = state.carry.lock().unwrap().take();
    let k = *state.px_per_css.lock().unwrap();
    let folded = state.settings.lock().unwrap().folded;
    if folded && layout::is_tap(entered, moved, k) {
        state.holds.lock().unwrap().touch(std::time::Instant::now());
        applog(app, "tap: the folded key");
        let _ = app.emit("click", ());
        set_fold(app, false, None);
        return;
    }
    let Some(Carry { hwnd, work, strip: (sx, sy, sw, sh), .. }) = carried else { return };
    let Some(at) = win::window_rect(hwnd) else {
        let _ = app.emit("reload", ());
        return;
    };
    let on_screen = (at.0 + sx, at.1 + sy, sw, sh);
    let edge = layout::drop_edge(work, k, on_screen);
    let along = layout::along_from(work, k, on_screen, edge);
    let settings = {
        let mut s = state.settings.lock().unwrap();
        s.edge = edge;
        if folded {
            // As the file will have it, so reading the file back changes nothing
            s.folded_at = config::share(along).as_f64().unwrap_or(along);
        } else {
            s.top = along;
        }
        s.clone()
    };
    if let Err(why) = change_settings(app, |text| config::with_landing(text, edge, folded, along)) {
        applog(app, &format!("settings: cannot save the capsule's place: {why}"));
    }
    applog(app, &format!("moved: {edge:?} edge, along {along:.3}{}", if folded { ", folded" } else { "" }));
    place_capsule(app, &settings);
    let _ = app.emit("reload", ());
}
/// Everything the settings window shows. The items come from the file as they are written
/// there, fields the panel does not know included, so the window can hand them back untouched.
#[tauri::command(async)]
fn get_settings(app: AppHandle) -> Value {
    let state = app.state::<AppState>();
    let s = state.settings.lock().unwrap().clone();
    let russian = is_russian(&s);
    let raw: Vec<Value> = settings_path(&app)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')).ok())
        .and_then(|v| v.get("items").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    // The programs are looked up only when some item is for one program
    let programs = s.items.iter().any(|it| it.only_in.is_some()).then(|| (win::app_windows(), installed_known()));
    let items: Vec<Value> = raw
        .into_iter()
        .map(|raw| match config::parse_item(&raw) {
            Ok(item) => {
                let (icon, glyph) = item_icon(&app, &item);
                let only = item.only_in.as_deref().zip(programs.as_ref()).map(|(p, (windows, apps))| program_label(p, windows, apps));
                json!({ "raw": raw, "icon": icon, "glyph": glyph, "problem": null, "only": only })
            }
            Err(why) => json!({ "raw": raw, "icon": null, "glyph": null, "problem": why, "only": null }),
        })
        .collect();
    let presets: Vec<Value> = config::PRESETS.iter().map(|p| json!({ "id": p.id, "item": config::preset_item(p, russian) })).collect();
    let pages: Vec<Value> = config::PAGES.iter().map(|p| json!({ "item": config::page_item(p, russian) })).collect();
    // A ready set as the editor lists it: its name and the items it brings, by name and icon
    let bundles: Vec<Value> = config::BUNDLES
        .iter()
        .map(|b| json!({ "id": b.id, "name": if russian { b.ru } else { b.en }, "items": config::bundle_items(b, russian, None, &[]) }))
        .collect();
    let glyphs: serde_json::Map<String, Value> = glyphs::names().map(|n| (n.to_string(), json!(glyphs::builtin(n)))).collect();
    json!({
        "items": items,
        "scale": s.scale,
        "edge": config::edge_name(s.edge),
        "theme": config::theme_name(s.theme),
        "accent": s.accent,
        "lang": config::lang_name(s.lang),
        "russian": russian,
        "autostart": autostart::is_enabled(),
        "updates": s.updates,
        "tablet_only": s.tablet_only,
        "click_sound": s.click_sound,
        "version": app.package_info().version.to_string(),
        "error": *state.error.lock().unwrap(),
        "intro": state.intro.load(Ordering::Relaxed),
        "presets": presets,
        "pages": pages,
        "bundles": bundles,
        "glyphs": glyphs,
        "update": *state.update.lock().unwrap(),
    })
}

/// Writes one field of the settings file and applies it at once.
fn write_field(app: &AppHandle, key: &str, value: Value) -> Result<(), String> {
    change_settings(app, |text| config::with_field(text, key, value))?;
    reload_settings(app);
    // The windows redraw even when nothing the panel itself uses has changed
    let _ = app.emit("reload", ());
    Ok(())
}

/// The settings window changes one setting. Autostart lives in the registry, the rest in the file.
#[tauri::command(async)]
fn set_pref(app: AppHandle, key: String, value: Value) -> Result<(), String> {
    let word = |allowed: &[&str]| value.as_str().filter(|v| allowed.contains(v)).map(|v| json!(v)).ok_or(format!("{key}: {value}"));
    let value = match key.as_str() {
        "autostart" => return autostart::set(value.as_bool().ok_or("autostart: not a switch")?),
        "theme" => word(&["system", "dark", "light"])?,
        "lang" => word(&["system", "ru", "en"])?,
        "edge" => word(&["left", "right", "top"])?,
        "accent" => json!(value.as_str().and_then(config::accent).ok_or("accent: not a colour")?),
        "scale" => json!(config::snap_scale(value.as_f64().ok_or("scale: not a number")?)),
        "updates" => json!(value.as_bool().ok_or("updates: not a switch")?),
        "tablet_only" => json!(value.as_bool().ok_or("tablet_only: not a switch")?),
        "click_sound" => json!(value.as_bool().ok_or("click_sound: not a switch")?),
        _ => return Err(format!("unknown setting {key}")),
    };
    write_field(&app, &key, value)
}

/// The editor hands over the whole list of items, in its order, each as the file has it.
#[tauri::command(async)]
fn save_items(app: AppHandle, items: Vec<Value>) -> Result<(), String> {
    write_field(&app, "items", Value::Array(items))
}

/// A key was carried to another place on the capsule: the item at `from` of the settings' list
/// goes where the item at `to` is.
#[tauri::command(async)]
fn move_item(app: AppHandle, from: usize, to: usize) -> Result<(), String> {
    change_settings(&app, |text| config::with_moved(text, from, to))?;
    applog(&app, &format!("order: item {} moved to place {}", from + 1, to + 1));
    reload_settings(&app);
    let _ = app.emit("reload", ());
    Ok(())
}

#[tauri::command(async)]
fn installed_apps() -> Vec<apps::App> {
    let apps = apps::installed();
    *INSTALLED.lock().unwrap() = Some(apps.clone());
    apps
}

/// The shortcuts pinned to the taskbar, for the editor's list.
#[tauri::command(async)]
fn pinned_apps() -> Vec<apps::App> {
    apps::pinned()
}

/// The items a ready set adds to the editor's list, named in the language of the panel, each for
/// one program when `only_in` names it; what the list already has is left out.
#[tauri::command(async)]
fn bundle_items(app: AppHandle, id: String, only_in: Option<String>, items: Vec<Value>) -> Result<Vec<Value>, String> {
    let bundle = config::BUNDLES.iter().find(|b| b.id == id).ok_or(format!("unknown set {id}"))?;
    let russian = is_russian(&app.state::<AppState>().settings.lock().unwrap());
    Ok(config::bundle_items(bundle, russian, only_in.as_deref(), &items))
}

/// A Windows file dialog of the settings window for key sets, which are `.tapka` files.
fn key_set_dialog(app: &AppHandle, window: &tauri::WebviewWindow) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    use tauri_plugin_dialog::DialogExt;
    app.dialog().file().set_parent(window).add_filter("Tapka", &["tapka"])
}

/// "Save my keys…": writes the items of the settings file, all of them or those for the program
/// `only_in` names, into a file the user chooses; `name` is what the dialog offers to call it.
/// Answers with the number of keys saved, or nothing when the dialog was closed. The errors
/// `large` and `taken` are words the page has its own lines for.
#[tauri::command(async)]
fn save_key_set(app: AppHandle, window: tauri::WebviewWindow, name: String, only_in: Option<String>) -> Result<Option<usize>, String> {
    let text = settings_path(&app).and_then(|path| std::fs::read_to_string(path).ok()).ok_or("cannot read settings")?;
    let (text, keys) = config::set_text(&text, only_in.as_deref())?;
    let Some(file) = key_set_dialog(&app, &window).set_file_name(format!("{name}.tapka")).blocking_save_file() else { return Ok(None) };
    let chosen = file.into_path().map_err(|e| e.to_string())?;
    let path = config::set_path(&chosen, |p| p.exists()).ok_or("taken")?;
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(Some(keys))
}

/// "Set from a file…": the user chooses a file, and its keys come back for the editor to show,
/// with the file's name. Nothing of the file is run or looked up, and nothing is added yet.
/// Answers with nothing when the dialog was closed, and with an error when it is not a key set.
#[tauri::command(async)]
fn open_key_set(app: AppHandle, window: tauri::WebviewWindow) -> Result<Option<Value>, String> {
    let Some(file) = key_set_dialog(&app, &window).blocking_pick_file() else { return Ok(None) };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let items = config::read_set(&path).ok_or("not a key set")?;
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    Ok(Some(json!({ "name": name, "items": items })))
}

/// What a set from a file adds to the editor's list: the items to put at its end, how many the
/// list already has, and how many open something that is not on this machine.
#[tauri::command(async)]
fn key_set_items(set: Vec<Value>, items: Vec<Value>) -> Result<Value, String> {
    let (added, had, missing) = config::set_items(&set, &items, apps::found).ok_or("not a key set")?;
    Ok(json!({ "added": added, "had": had, "missing": missing }))
}

/// The programs that have windows right now, by name, for the editor's "Show only in" list: each
/// as `{ name, exe }`.
#[tauri::command(async)]
fn running_programs() -> Vec<Value> {
    let apps = installed_known();
    let mut found: Vec<(String, String)> = Vec::new();
    for w in win::app_windows() {
        if !found.iter().any(|(_, exe)| *exe == w.exe) {
            found.push((apps::name_of(&apps, &w.exe, w.family.as_deref()), w.exe));
        }
    }
    found.sort_by_key(|(name, _)| name.to_lowercase());
    found.into_iter().map(|(name, exe)| json!({ "name": name, "exe": exe })).collect()
}

/// The icon Windows shows for a program, for the editor's list.
#[tauri::command(async)]
fn icon_of(target: String) -> Option<String> {
    shell_icon(&target)
}

/// Copies a picture the user chose into `icons` of the data folder and returns its file name,
/// which is what an item's `icon` carries.
#[tauri::command(async)]
fn import_icon(app: AppHandle, path: String) -> Result<String, String> {
    let from = std::path::Path::new(&path);
    if !matches!(path.rsplit_once('.').map(|(_, ext)| ext.to_lowercase()).as_deref(), Some("png" | "svg")) {
        return Err("PNG or SVG only".into());
    }
    let icons = data_dir(&app).ok_or("no data folder")?.join("icons");
    config::keep_icon(from, &icons).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_settings_file(app: AppHandle) {
    if let Some(path) = settings_path(&app) {
        if let Err(why) = win::open(&path.to_string_lossy()) {
            applog(&app, &format!("action failed: open settings file: {why}"));
        }
    }
}

fn set_update(app: &AppHandle, update: Update) {
    *app.state::<AppState>().update.lock().unwrap() = update;
    let _ = app.emit("reload", ());
}

/// Asks GitHub for the latest release and remembers the answer. Quiet: the answer shows only in
/// the settings window.
async fn look_for_update(app: &AppHandle) {
    use tauri_plugin_updater::UpdaterExt;
    set_update(app, Update::Checking);
    let found = match app.updater() {
        Ok(updater) => updater.check().await.map_err(|e| e.to_string()),
        Err(why) => Err(why.to_string()),
    };
    let update = match found {
        Ok(Some(update)) => Update::Found { version: update.version },
        Ok(None) => Update::Latest,
        Err(why) => Update::Failed { why },
    };
    applog(app, &match &update {
        Update::Found { version } => format!("update: version {version} is available"),
        Update::Failed { why } => format!("update: check failed: {why}"),
        _ => "update: this is the latest version".to_string(),
    });
    set_update(app, update);
}

/// The settings window's "Check" button. With update checks off the panel does not go online.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<(), String> {
    if !app.state::<AppState>().settings.lock().unwrap().updates {
        return Err("update checks are off".into());
    }
    look_for_update(&app).await;
    Ok(())
}

/// The "Update" button: downloads the installer of the new version, checks its signature
/// against the project's key, runs it and leaves; the installer starts the new version.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    set_update(&app, Update::Loading);
    let done = async {
        let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?.ok_or("no newer version")?;
        applog(&app, &format!("update: installing {}", update.version));
        update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())
    }
    .await;
    match done {
        Ok(()) => app.restart(),
        Err(why) => {
            applog(&app, &format!("update: failed: {why}"));
            set_update(&app, Update::Failed { why: why.clone() });
            Err(why)
        }
    }
}

#[tauri::command]
fn open_log_folder(app: AppHandle) {
    if let Some(dir) = data_dir(&app) {
        let _ = win::open(&dir.to_string_lossy());
    }
}

#[tauri::command]
fn open_github() {
    let _ = win::open(GITHUB);
}

#[tauri::command]
fn quit(app: AppHandle) {
    app.exit(0);
}

fn window_theme(theme: config::Theme) -> Option<tauri::Theme> {
    match theme {
        config::Theme::Dark => Some(tauri::Theme::Dark),
        config::Theme::Light => Some(tauri::Theme::Light),
        config::Theme::System => None,
    }
}

/// The settings window: created on demand, brought forward if open. It opens on the items. It
/// has no taskbar button, so it is put in front by force: a tap on the capsule, which never takes
/// focus, gives the panel no right to change the active window.
fn open_settings_window(app: &AppHandle) {
    // One opening at a time: a second tap that comes while the window opens adds nothing, and two
    // threads must not both find no window and make one each.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let Ok(_turn) = ONE_AT_A_TIME.try_lock() else { return };
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.unminimize();
        let _ = w.show();
        if let Ok(hwnd) = w.hwnd() {
            if let Err(why) = win::bring_to_front(hwnd.0 as isize) {
                applog(app, &format!("settings window: {why}"));
            }
        }
        let _ = w.emit("section", "items");
        return;
    }
    let theme = app.state::<AppState>().settings.lock().unwrap().theme;
    let built = tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App("settings.html".into()))
        .title("Tapka")
        .skip_taskbar(true)
        .inner_size(880.0, 640.0)
        .min_inner_size(640.0, 460.0)
        .theme(window_theme(theme))
        .build();
    match built {
        Ok(w) => {
            // Always in the middle of the screen: left to Windows, each new window lands a step
            // further down and to the right of the last one
            let _ = w.center();
            if let Ok(hwnd) = w.hwnd() {
                if let Err(why) = win::bring_to_front(hwnd.0 as isize) {
                    applog(app, &format!("settings window: {why}"));
                }
            }
        }
        Err(why) => applog(app, &format!("settings window: {why}")),
    }
}
const TRAY_TITLE: &str = "Tapka";

/// The tray tooltip is where a rejected settings file shows up, until the next successful read.
fn update_tray_tooltip(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("tray") else { return };
    let text = match app.state::<AppState>().error.lock().unwrap().as_deref() {
        // Windows cuts tooltips at 127 characters
        Some(why) => {
            let russian = is_russian(&app.state::<AppState>().settings.lock().unwrap());
            let what = if russian { "ошибка в файле настроек" } else { "the settings file is invalid" };
            format!("{TRAY_TITLE}: {what}: {why}").chars().take(120).collect()
        }
        None => TRAY_TITLE.to_string(),
    };
    let _ = tray.set_tooltip(Some(text));
}

/// Shows or hides the capsule without giving it focus. During a snapshot it is only remembered
/// that the capsule should be shown: the snapshot's own return shows it.
fn set_capsule_visible(app: &AppHandle, visible: bool) {
    if !visible {
        let_go_all(app, "the capsule was hidden");
    }
    let state = app.state::<AppState>();
    // Asked for before the turn is taken: off the main thread the answer waits for the main
    // thread, which may itself be waiting for the turn (the tray, a command of the page)
    let hwnd = app.get_webview_window("capsule").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
    let _turn = state.showing.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.wanted.store(visible, Ordering::Relaxed);
    if visible && state.snapping.load(Ordering::Relaxed) {
        return;
    }
    if let Some(hwnd) = hwnd {
        win::set_visible(hwnd, visible);
        grab::set_visible(visible);
    }
}

/// The tray menu in the panel's language: settings, show or hide the capsule, quit.
fn tray_menu(app: &AppHandle, russian: bool) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItem};
    let (prefs, toggle, quit) = if russian { ("Настройки…", "Показать или скрыть капсулу", "Выход") } else { ("Settings…", "Show or hide the capsule", "Quit") };
    let prefs = MenuItem::with_id(app, "prefs", prefs, true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", toggle, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    Menu::with_items(app, &[&prefs, &toggle, &quit])
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let russian = is_russian(&app.state::<AppState>().settings.lock().unwrap());
    let mut tray = tauri::tray::TrayIconBuilder::with_id("tray").tooltip(TRAY_TITLE).menu(&tray_menu(app, russian)?);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(move |app, event| match event.id.as_ref() {
        "prefs" => open_settings_window(app),
        "toggle" => {
            let wanted = app.state::<AppState>().wanted.load(Ordering::Relaxed);
            set_capsule_visible(app, !wanted);
        }
        "quit" => app.exit(0),
        _ => {}
    })
    .build(app)?;
    Ok(())
}

/// What follows the settings outside the pages: the slot's colour, the settings window's title
/// bar and the language of the tray menu.
fn apply_look(app: &AppHandle, settings: &config::Settings) {
    grab::set_accent(&settings.accent);
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.set_theme(window_theme(settings.theme));
    }
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("tray"), tray_menu(app, is_russian(settings))) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn settings_path(app: &AppHandle) -> Option<std::path::PathBuf> {
    Some(data_dir(app)?.join("settings.json"))
}

/// Reads settings.json over the current settings and redraws the capsule. An invalid file keeps
/// the items that are on screen; the reason goes to the log and stays in `AppState::error`.
fn reload_settings(app: &AppHandle) {
    // One reading at a time: a thread that read the file earlier must not apply it later
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(path) = settings_path(app) else { return };
    let state = app.state::<AppState>();
    if state.carry.lock().unwrap().is_some() {
        return; // the drop saves the file and redraws
    }
    let read = std::fs::read_to_string(&path).map_err(|e| format!("cannot read settings: {e}"));
    let current = state.settings.lock().unwrap().clone();
    let (settings, skipped, error) = match read {
        Ok(text) => config::reload(&current, &text),
        Err(why) => (current.clone(), Vec::new(), Some(why)),
    };
    // The panel's own save, or a save that changed nothing: there is nothing to redraw
    if settings == current && error == *state.error.lock().unwrap() {
        return;
    }
    for line in skipped {
        applog(app, &format!("settings: {line}"));
    }
    match &error {
        Some(why) => applog(app, &format!("settings: invalid file, items unchanged: {why}")),
        None => applog(app, &format!("settings: {} items", settings.items.len())),
    }
    if settings.items != current.items {
        state.items_age.fetch_add(1, Ordering::Relaxed);
    }
    *state.settings.lock().unwrap() = settings.clone();
    *state.error.lock().unwrap() = error;
    apply_look(app, &settings);
    place_capsule(app, &settings);
    update_tray_tooltip(app);
    let _ = app.emit("reload", ());
}

/// WebView2 in a window that never takes focus gets presses but not mouse movement, so hover does
/// not exist for the page (measured 2026-10-04). The cursor is
/// read here and the page is told where it is over the capsule, in CSS px, or `null` when it left.
fn watch_cursor(app: AppHandle) {
    let Some(hwnd) = app.get_webview_window("capsule").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize) else {
        return;
    };
    std::thread::spawn(move || {
        let mut last = None;
        loop {
            // Close by, the cursor is followed often enough for the highlight to keep up; away
            // from the capsule, or with the capsule hidden, a slow look is enough to notice it
            // coming back, and the processor is left to sleep
            let pause = if last.is_some() { 80 } else { 250 };
            std::thread::sleep(std::time::Duration::from_millis(pause));
            if app.state::<AppState>().carry.lock().unwrap().is_some() {
                continue;
            }
            let at = win::cursor_in_window(hwnd);
            if at == last {
                continue;
            }
            last = at;
            let k = *app.state::<AppState>().px_per_css.lock().unwrap();
            let _ = app.emit("hover", at.map(|(x, y)| (x as f64 / k, y as f64 / k)));
        }
    });
}

/// The primary monitor's work area in physical px and its scale: what the capsule's place depends
/// on besides the settings.
fn screen(app: &AppHandle) -> Option<((i32, i32, u32, u32), f64)> {
    let mon = app.get_webview_window("capsule")?.primary_monitor().ok()??;
    let wa = mon.work_area();
    Some(((wa.position.x, wa.position.y, wa.size.width, wa.size.height), mon.scale_factor()))
}

/// Looks at the active window's program. When it brings other items to the capsule, the capsule
/// takes the new set; the first time in this start a program brings its own items, the page is
/// also given the words that say whose they are.
fn follow_front(app: &AppHandle) {
    let state = app.state::<AppState>();
    let front = win::front_program();
    let before = std::mem::replace(&mut *state.front.lock().unwrap(), front.clone());
    if front == before {
        return;
    }
    let_go_all(app, "the active program changed");
    let settings = state.settings.lock().unwrap().clone();
    // A folded capsule stays folded and says nothing; unfolded, it draws the keys of the program
    // in front then
    if settings.folded {
        return;
    }
    let now = actions::shown_items(&settings.items, program_ref(&front));
    if now == actions::shown_items(&settings.items, program_ref(&before)) {
        return;
    }
    let brings = now.iter().any(|&i| settings.items[i].only_in.is_some());
    let note = match &front {
        Some((exe, family)) if brings && !state.introduced.lock().unwrap().contains(exe) => {
            state.introduced.lock().unwrap().push(exe.clone());
            let name = apps::name_of(&installed_known(), exe, family.as_deref());
            Some(if is_russian(&settings) { format!("Клавиши для {name}") } else { format!("Keys for {name}") })
        }
        _ => None,
    };
    place_capsule(app, &settings);
    let _ = app.emit("set", note);
}

/// Notices a saved settings file, a detached or attached keyboard and a turned or resized screen
/// within half a second. `by_keyboard` is where the keyboard rule had the capsule at start.
fn watch_settings(app: AppHandle, mut by_keyboard: bool) {
    let Some(path) = settings_path(&app) else { return };
    let modified = move || std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    std::thread::spawn(move || {
        let mut seen = modified();
        let mut seen_screen = screen(&app);
        let mut was_shown = true;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let now = modified();
            if now != seen {
                seen = now;
                reload_settings(&app);
            }
            let tablet = win::tablet_mode();
            let state = app.state::<AppState>();
            let_go(&app, &format!("no tap for {} seconds", actions::HOLD_IDLE_MS / 1000), true);
            // The screen was turned, or its resolution, scale or taskbar changed: the capsule goes
            // to the same edge and the same share along it in the new work area
            let now_screen = screen(&app);
            if now_screen != seen_screen && now_screen.is_some() && state.carry.lock().unwrap().is_none() {
                seen_screen = now_screen;
                if let Some(((_, _, w, h), scale)) = now_screen {
                    applog(&app, &format!("screen: work area {w}x{h}, scale {scale}"));
                }
                let settings = state.settings.lock().unwrap().clone();
                place_capsule(&app, &settings);
                let _ = app.emit("reload", ());
            }
            // A hidden capsule shows no marks and needs no handle above it: the desktop is not
            // looked over until it is back
            let shown = app.get_webview_window("capsule").and_then(|w| w.hwnd().ok()).is_some_and(|h| win::is_visible(h.0 as isize));
            if shown && state.carry.lock().unwrap().is_none() {
                grab::raise();
                follow_front(&app);
            }
            // A program an item opens appeared or closed: the page marks the item
            let items = state.settings.lock().unwrap().items.clone();
            let now = if shown { item_marks(&items) } else { state.marks.lock().unwrap().clone() };
            let changed = {
                let mut marks = state.marks.lock().unwrap();
                let changed = *marks != now;
                if changed {
                    *marks = now.clone();
                }
                changed
            };
            // Back from hiding, the page is told the marks whether or not they differ from the
            // last ones sent: it may have redrawn itself in between
            let back = shown && !was_shown;
            was_shown = shown;
            if changed || back {
                let _ = app.emit("marks", now);
            }
            if state.tablet.swap(tablet, Ordering::Relaxed) != tablet {
                applog(&app, if tablet { "tablet mode: on" } else { "tablet mode: off" });
                let settings = state.settings.lock().unwrap().clone();
                place_capsule(&app, &settings);
                let _ = app.emit("reload", ());
            }
            // The keyboard was attached or detached, or the switch was turned: the capsule goes
            // where the rule puts it. Between such moments the tray's choice stands
            let tablet_only = state.settings.lock().unwrap().tablet_only;
            let now = actions::keyboard_shows_capsule(tablet_only, tablet);
            if now != by_keyboard {
                by_keyboard = now;
                applog(&app, if now { "capsule: shown by the keyboard rule" } else { "capsule: hidden, a keyboard is attached" });
                set_capsule_visible(&app, now);
            }
        }
    });
}

/// First read at start: brings the settings over from 0.1.0's folder if they are there, creates
/// the default file if there is none. An invalid file gives an empty capsule and the reason. The
/// last value says that the file was created just now.
fn load_settings(app: &AppHandle) -> (config::Settings, Option<String>, bool) {
    let Some(path) = settings_path(app) else { return (config::Settings::empty(), None, false) };
    let own = std::env::var_os("TAPKA_DATA_DIR").is_none();
    if let (true, Some(dir), Some(roaming)) = (own, path.parent(), path.parent().and_then(|d| d.parent())) {
        match config::migrate(&roaming.join(config::OLD_DATA_DIR), dir) {
            Ok(copied) if copied.is_empty() => {}
            Ok(copied) => applog(app, &format!("moved from {}: {}", config::OLD_DATA_DIR, copied.join(", "))),
            Err(why) => applog(app, &format!("moving from {} failed: {why}", config::OLD_DATA_DIR)),
        }
    }
    match config::load_or_create(&path, win::system_is_russian()) {
        Ok((settings, skipped, created)) => {
            for line in skipped {
                applog(app, &format!("settings: {line}"));
            }
            applog(app, &format!("settings: {} items{}", settings.items.len(), if created { ", first run" } else { "" }));
            (settings, None, created)
        }
        Err(why) => {
            applog(app, &format!("settings: invalid file, capsule is empty: {why}"));
            (config::Settings::empty(), Some(why), false)
        }
    }
}
fn main() {
    tauri::Builder::default()
        // A second launch ends itself and shows the first one's capsule
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| set_capsule_visible(app, true)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_view,
            tap,
            foot,
            fold,
            unfold,
            card,
            get_settings,
            set_pref,
            save_items,
            move_item,
            installed_apps,
            pinned_apps,
            bundle_items,
            save_key_set,
            open_key_set,
            key_set_items,
            running_programs,
            icon_of,
            import_icon,
            check_update,
            install_update,
            open_settings_file,
            open_log_folder,
            open_github,
            quit
        ])
        .setup(|app| {
            let (settings, error, created) = load_settings(app.handle());
            let tablet = win::tablet_mode();
            applog(app.handle(), if tablet { "tablet mode: on" } else { "tablet mode: off" });
            let by_keyboard = actions::keyboard_shows_capsule(settings.tablet_only, tablet);
            app.manage(AppState {
                settings: Mutex::new(settings),
                error: Mutex::new(error),
                card_open: AtomicBool::new(false),
                card_rect: Mutex::new((0, 0, 0, 0)),
                tablet: AtomicBool::new(tablet),
                px_per_css: Mutex::new(1.0),
                carry: Mutex::new(None),
                marks: Mutex::new(Vec::new()),
                front: Mutex::new(None),
                introduced: Mutex::new(Vec::new()),
                intro: AtomicBool::new(created),
                update: Mutex::new(Update::None),
                wanted: AtomicBool::new(by_keyboard),
                snapping: AtomicBool::new(false),
                showing: Mutex::new(()),
                holds: Mutex::new(actions::Holds::default()),
                signs: AtomicBool::new(false),
                items_age: std::sync::atomic::AtomicU32::new(0),
            });
            grab::start(app.handle().clone());
            let capsule = app.get_webview_window("capsule").and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
            if let Some(hwnd) = capsule {
                win::tool_window(hwnd);
                win::quiet_frame(hwnd);
            }
            let settings = app.state::<AppState>().settings.lock().unwrap().clone();
            place_capsule(app.handle(), &settings);
            if by_keyboard {
                // Not through Tauri's show: it rewrites the window's style, taskbar button included
                if let Some(hwnd) = capsule {
                    win::set_visible(hwnd, true);
                }
                // The handle was placed while the capsule was not yet on screen
                grab::set_visible(true);
                grab::raise();
            } else {
                applog(app.handle(), "capsule: hidden, a keyboard is attached");
            }
            applog(app.handle(), "start");
            if let Err(why) = build_tray(app.handle()) {
                applog(app.handle(), &format!("tray: {why}"));
            }
            update_tray_tooltip(app.handle());
            apply_look(app.handle(), &settings);
            if created {
                open_settings_window(app.handle());
            }
            watch_settings(app.handle().clone(), by_keyboard);
            watch_cursor(app.handle().clone());
            if settings.updates {
                // Once per start, a few seconds in, so the start itself waits for nothing
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    // The switch may have been turned off in these seconds. The file is asked, not
                    // the memory: a carry in progress puts off re-reading the file
                    let on = settings_path(&app)
                        .and_then(|path| std::fs::read_to_string(path).ok())
                        .and_then(|text| config::parse(text.trim_start_matches('\u{feff}')).ok())
                        .is_some_and(|(settings, _)| settings.updates);
                    if on {
                        tauri::async_runtime::block_on(look_for_update(&app));
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running Tapka")
        .run(|app, event| {
            // No key stays down after the panel is gone
            if let tauri::RunEvent::Exit = event {
                let_go_all(app, "the panel quits");
            }
        });
}
