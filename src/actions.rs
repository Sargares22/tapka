//! What the panel does in response to a tap, separated from the Windows calls that do it.
//! `steps_for` turns a tap into a list of `Step`s; tests compare the list instead of pressing
//! real keys.

use crate::config::{Action, Item};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    HideCapsule,
    SendKeys(String),
    Open(String),
    /// Bring the already running program behind this target to the front.
    Show(String),
    /// Minimize the windows of the program behind this target.
    Minimize(String),
    /// Press this key and keep it down, or let it go if it is held (`Holds::toggle`).
    Hold(String),
}

/// What is known about the program an `open` item starts, at the moment of a tap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Running {
    /// No window of it is open, or the item is not a program at all.
    No,
    /// It has windows, and another program is in front.
    Behind,
    /// One of its windows is the active one.
    Front,
}

/// The longest a hidden capsule stays away, whatever happens.
pub const RETURN_AFTER_MS: u64 = 20_000;
/// How long the region selection is given to open before the capsule gives up waiting for it.
pub const SNIP_OPENS_WITHIN_MS: u64 = 3_000;

/// What the capsule has seen of the screen region selection since it hid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Snip {
    /// The selection has not come up yet.
    NotYet,
    /// The selection is on screen.
    Open,
    /// The selection was on screen and is gone: a shot was taken or it was cancelled.
    Closed,
}

/// Win+Shift+S starts the screen region selection; the capsule hides so it is not in the shot.
/// No other hotkey hides it.
pub fn hides_capsule(keys: &str) -> bool {
    let Ok(mut vks) = crate::keys::parse(keys) else { return false };
    vks.sort();
    vks == [crate::keys::VK_SHIFT, 0x53, crate::keys::VK_LWIN]
}

/// A hidden capsule comes back as soon as the selection it hid for has closed, with a shot or
/// without one. It also comes back if the selection never opened, and after RETURN_AFTER_MS
/// whatever happens. It does not wait for the clipboard: other programs change it too.
pub fn capsule_returns(snip: Snip, hidden_ms: u64) -> bool {
    match snip {
        Snip::Closed => true,
        Snip::NotYet => hidden_ms >= SNIP_OPENS_WITHIN_MS,
        Snip::Open => hidden_ms >= RETURN_AFTER_MS,
    }
}

/// Where the keyboard puts the capsule: on screen, unless the user asked to see it only in tablet
/// mode and a keyboard is attached. The tray and a second launch may then say otherwise, until
/// the keyboard is attached or detached again.
pub fn keyboard_shows_capsule(tablet_only: bool, tablet: bool) -> bool {
    !tablet_only || tablet
}

/// Does a window belong to the program an `open` target starts? `exe` is the window's program
/// file name in lower case, `family` its Store package family, if it has one.
/// A target that is a path to an exe matches by file name; a `shell:AppsFolder\<family>!<app>`
/// address matches by package family. Addresses, documents and anything else match nothing: the
/// panel cannot tell which window they opened.
pub fn belongs(target: &str, exe: &str, family: Option<&str>) -> bool {
    let target = target.trim().to_lowercase();
    if let Some(app) = target.strip_prefix("shell:appsfolder\\") {
        let wanted = app.split('!').next().unwrap_or("");
        return !wanted.is_empty() && family.is_some_and(|f| f.to_lowercase() == wanted);
    }
    if target.ends_with(".exe") && !target.contains("://") {
        return target.rsplit(['\\', '/']).next() == Some(exe);
    }
    false
}

/// Is a window of this program the one an `only_in` names? `only_in` is a program's file name or
/// its Store package family; a path to an exe and a `shell:AppsFolder\…` address match as in
/// `belongs`.
pub fn runs_in(only_in: &str, exe: &str, family: Option<&str>) -> bool {
    belongs(only_in, exe, family) || family.is_some_and(|f| f.to_lowercase() == only_in.trim().to_lowercase())
}

/// How many of the items for all programs stay at the head of the capsule when a program brings
/// its own: these keys never move, and the program's keys come right after them, so they are
/// among the four in sight rather than past the end of the list.
pub const FIXED_KEYS: usize = 2;

/// The items the capsule shows while `front` is the program of the active window, as indices into
/// `items`: the first FIXED_KEYS items for all programs, then the items for this program, then
/// the rest of the items for all programs, each in its order in the file. `front` is `None` when
/// nothing is known, or when the desktop or the panel itself is active: then only the items for
/// all programs show.
pub fn shown_items(items: &[Item], front: Option<(&str, Option<&str>)>) -> Vec<usize> {
    let everywhere: Vec<usize> = (0..items.len()).filter(|&i| items[i].only_in.is_none()).collect();
    let here = (0..items.len()).filter(|&i| match (items[i].only_in.as_deref(), front) {
        (Some(only), Some((exe, family))) => runs_in(only, exe, family),
        _ => false,
    });
    let (head, rest) = everywhere.split_at(FIXED_KEYS.min(everywhere.len()));
    head.iter().copied().chain(here).chain(rest.iter().copied()).collect()
}

/// The steps a tap on an item with this action asks for, in order. An `open` item whose program
/// is already running behaves like its taskbar button: behind other windows it comes to the
/// front, in front it is minimized. Nothing is ever closed.
pub fn steps_for(action: &Action, running: Running) -> Vec<Step> {
    match (action, running) {
        (Action::Open(target), Running::Behind) => vec![Step::Show(target.clone())],
        (Action::Open(target), Running::Front) => vec![Step::Minimize(target.clone())],
        (Action::Open(target), Running::No) => vec![Step::Open(target.clone())],
        (Action::Hotkey(keys), _) if hides_capsule(keys) => vec![Step::HideCapsule, Step::SendKeys(keys.clone())],
        (Action::Hotkey(keys), _) => vec![Step::SendKeys(keys.clone())],
        (Action::Hold(key), _) => vec![Step::Hold(key.clone())],
    }
}

/// A held key lets go by itself after this long without a tap on the capsule. A tap is the only
/// sign of the person the panel sees, and a key left down by mistake would change every key
/// pressed on the device afterwards.
pub const HOLD_IDLE_MS: u64 = 20_000;

/// The keys `hold` items keep pressed, and when the capsule was last tapped. Every change is
/// sent to the system through `send(vk, up)` before the method returns, so whoever holds this
/// under a lock sends the key under the same lock: a tap and a let-go on two threads cannot
/// leave a key down in Windows that is no longer counted here, or the other way round.
#[derive(Debug, Default)]
pub struct Holds {
    /// Virtual-key codes, in the order they were pressed.
    keys: Vec<u16>,
    last_tap: Option<Instant>,
}

impl Holds {
    pub fn held(&self) -> &[u16] {
        &self.keys
    }

    /// A tap anywhere on the capsule: the wait before held keys let go starts over.
    pub fn touch(&mut self, now: Instant) {
        self.last_tap = Some(now);
    }

    /// A tap on a `hold` item: its key is pressed if it was up and let go if it was held.
    /// Returns whether the key is held now.
    pub fn toggle(&mut self, vk: u16, mut send: impl FnMut(u16, bool)) -> bool {
        let down = match self.keys.iter().position(|&k| k == vk) {
            Some(at) => {
                self.keys.remove(at);
                false
            }
            None => {
                self.keys.push(vk);
                true
            }
        };
        send(vk, !down);
        down
    }

    /// Lets go of every held key once HOLD_IDLE_MS have passed since the last tap at `now`.
    /// Returns the keys let go.
    pub fn expire(&mut self, now: Instant, send: impl FnMut(u16, bool)) -> Vec<u16> {
        let idle = self.last_tap.is_none_or(|tap| now.saturating_duration_since(tap) >= Duration::from_millis(HOLD_IDLE_MS));
        if idle {
            self.release_all(send)
        } else {
            Vec::new()
        }
    }

    /// Lets go of every held key, the last pressed first: the active program changed, the capsule
    /// went off the screen, or the panel is quitting. Returns the keys let go.
    pub fn release_all(&mut self, mut send: impl FnMut(u16, bool)) -> Vec<u16> {
        let mut keys = std::mem::take(&mut self.keys);
        keys.reverse();
        for &vk in &keys {
            send(vk, true);
        }
        keys
    }
}

/// A hotkey's keys without the modifiers a `hold` item already keeps down: sent again, they would
/// come up at the end of the hotkey and no longer be held. The last key is always sent.
pub fn without_held(vks: &[u16], held: &[u16]) -> Vec<u16> {
    let Some((key, mods)) = vks.split_last() else { return Vec::new() };
    mods.iter().filter(|vk| !held.contains(vk)).chain([key]).copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_running_program_behaves_like_its_taskbar_button() {
        let chrome = Action::Open(r"C:\Program Files\Google\Chrome\Application\chrome.exe".into());
        let Action::Open(target) = &chrome else { unreachable!() };
        assert_eq!(steps_for(&chrome, Running::No), [Step::Open(target.clone())]);
        assert_eq!(steps_for(&chrome, Running::Behind), [Step::Show(target.clone())]);
        assert_eq!(steps_for(&chrome, Running::Front), [Step::Minimize(target.clone())]);
        // A hotkey has no program behind it: it is sent whatever is running
        let dictate = Action::Hotkey("ctrl+space".into());
        for running in [Running::No, Running::Behind, Running::Front] {
            assert_eq!(steps_for(&dictate, running), [Step::SendKeys("ctrl+space".into())]);
        }
    }

    #[test]
    fn windows_are_matched_to_exe_paths_and_store_addresses_only() {
        let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        assert!(belongs(chrome, "chrome.exe", None));
        assert!(belongs("  C:/Tools/Notes.EXE ", "notes.exe", None));
        assert!(!belongs(chrome, "msedge.exe", None));
        let store = r"shell:AppsFolder\Vendor.Notes_abc123xyz!App";
        assert!(belongs(store, "notes.exe", Some("Vendor.Notes_abc123xyz")));
        assert!(belongs(store, "anything.exe", Some("vendor.notes_ABC123XYZ")));
        assert!(!belongs(store, "notes.exe", None));
        assert!(!belongs(store, "notes.exe", Some("Vendor.Other_abc123xyz")));
        // An address or a document: no way to know which window it became
        assert!(!belongs("http://localhost:8080", "chrome.exe", None));
        assert!(!belongs("https://example.com/setup.exe", "setup.exe", None));
        assert!(!belongs(r"C:\Docs\notes.txt", "notepad.exe", None));
    }

    #[test]
    fn items_for_one_program_show_only_while_it_is_in_front() {
        let item = |name: &str, only_in: Option<&str>| Item {
            name: name.into(),
            icon: None,
            hint: None,
            lit: None,
            only_in: only_in.map(str::to_string),
            action: Action::Hotkey("b".into()),
        };
        let items = [
            item("snip", None),
            item("brush", Some("paint.exe")),
            item("paste", None),
            item("notes", Some("vendor.notes_abc123xyz")),
            item("eraser", Some("paint.exe")),
        ];
        // No program known, the desktop or the panel itself: the items for all programs
        assert_eq!(shown_items(&items, None), [0, 2]);
        // Its own program: the first two items for all programs, then its items, then the rest;
        // with fewer than two for all programs, its items come right after those there are
        assert_eq!(shown_items(&items, Some(("paint.exe", None))), [0, 2, 1, 4]);
        assert_eq!(shown_items(&items[1..], Some(("paint.exe", None))), [1, 0, 3]);
        // A Store app is known by its package family as well
        assert_eq!(shown_items(&items, Some(("notes.exe", Some("Vendor.Notes_abc123xyz")))), [0, 2, 3]);
        // Another program: the same as none
        assert_eq!(shown_items(&items, Some(("other.exe", None))), [0, 2]);
        // A path or a Store address in `only_in` names the program as an `open` target does
        assert!(runs_in(r"c:\tools\paint.exe", "paint.exe", None));
        assert!(runs_in(r"shell:appsfolder\vendor.notes_abc123xyz!app", "notes.exe", Some("Vendor.Notes_abc123xyz")));
        assert!(!runs_in("paint", "paint.exe", None));
    }

    #[test]
    fn a_program_s_keys_come_after_the_first_two_keys_for_all_programs() {
        let item = |only_in: Option<&str>| Item { name: "a".into(), icon: None, hint: None, lit: None, only_in: only_in.map(str::to_string), action: Action::Hotkey("b".into()) };
        // all1, paint1, all2, all3, paint2, all4, all5
        let items = [item(None), item(Some("paint.exe")), item(None), item(None), item(Some("paint.exe")), item(None), item(None)];
        // all1, all2, paint1, paint2, all3, all4, all5
        assert_eq!(shown_items(&items, Some(("paint.exe", None))), [0, 2, 1, 4, 3, 5, 6]);
        assert_eq!(shown_items(&items, None), [0, 2, 3, 5, 6]);
    }

    #[test]
    fn a_held_key_is_pressed_by_a_tap_and_let_go_by_the_next() {
        use crate::keys::{VK_CONTROL, VK_SHIFT};
        assert_eq!(steps_for(&Action::Hold("shift".into()), Running::No), [Step::Hold("shift".into())]);
        let mut holds = Holds::default();
        let mut sent = Vec::new();
        let t = Instant::now();
        holds.touch(t);
        assert!(holds.toggle(VK_SHIFT, |vk, up| sent.push((vk, up))));
        // Two at once: Shift and Ctrl
        holds.touch(t);
        assert!(holds.toggle(VK_CONTROL, |vk, up| sent.push((vk, up))));
        assert_eq!(holds.held(), [VK_SHIFT, VK_CONTROL]);
        // The second tap on Shift lets it go; Ctrl stays
        holds.touch(t);
        assert!(!holds.toggle(VK_SHIFT, |vk, up| sent.push((vk, up))));
        assert_eq!(holds.held(), [VK_CONTROL]);
        assert_eq!(sent, [(VK_SHIFT, false), (VK_CONTROL, false), (VK_SHIFT, true)]);
        // A hotkey sent meanwhile goes without the held Ctrl, which stays down
        assert_eq!(without_held(&[VK_CONTROL, VK_SHIFT, 0x53], holds.held()), [VK_SHIFT, 0x53]);
        assert_eq!(without_held(&[0x20], &[0x20]), [0x20]);
    }

    #[test]
    fn held_keys_let_go_by_themselves_after_twenty_seconds_without_a_tap() {
        assert_eq!(HOLD_IDLE_MS, 20_000);
        let mut holds = Holds::default();
        let t = Instant::now();
        let at = |ms| t + Duration::from_millis(ms);
        let mut ups = Vec::new();
        holds.touch(t);
        holds.toggle(0x10, |_, _| {});
        holds.toggle(0x12, |_, _| {});
        assert!(holds.expire(at(19_999), |vk, _| ups.push(vk)).is_empty());
        // Any tap on the capsule starts the wait over
        holds.touch(at(15_000));
        assert!(holds.expire(at(34_999), |vk, _| ups.push(vk)).is_empty());
        assert!(ups.is_empty());
        // Then they go, the last pressed first, and nothing is left to let go
        assert_eq!(holds.expire(at(35_000), |vk, up| ups.push(if up { vk } else { 0 })), [0x12, 0x10]);
        assert_eq!(ups, [0x12, 0x10]);
        assert!(holds.held().is_empty());
        assert!(holds.expire(at(60_000), |_, _| panic!("nothing is held")).is_empty());
    }

    #[test]
    fn held_keys_let_go_when_the_program_changes_or_the_capsule_hides() {
        let mut holds = Holds::default();
        let mut sent = Vec::new();
        holds.touch(Instant::now());
        holds.toggle(0x11, |_, _| {});
        holds.toggle(0x20, |_, _| {});
        assert_eq!(holds.release_all(|vk, up| sent.push((vk, up))), [0x20, 0x11]);
        assert_eq!(sent, [(0x20, true), (0x11, true)]);
        assert!(holds.held().is_empty());
        assert!(holds.release_all(|_, _| panic!("nothing is held")).is_empty());
        // Pressed again after that, a key is held as before
        assert!(holds.toggle(0x11, |_, _| {}));
    }

    #[test]
    fn a_tap_and_a_let_go_at_the_same_moment_leave_windows_and_the_count_agreeing() {
        use std::sync::{Arc, Mutex};
        // What Windows thinks is down, changed only through `send`, as the panel does it: under
        // the lock on the holds
        let holds = Arc::new(Mutex::new(Holds::default()));
        let windows = Arc::new(Mutex::new(Vec::<u16>::new()));
        let send = |windows: &Arc<Mutex<Vec<u16>>>| {
            let windows = windows.clone();
            move |vk: u16, up: bool| {
                let mut down = windows.lock().unwrap();
                down.retain(|&k| k != vk);
                if !up {
                    down.push(vk);
                }
            }
        };
        let taps = {
            let (holds, send) = (holds.clone(), send(&windows));
            std::thread::spawn(move || {
                for i in 0..20_000u32 {
                    holds.lock().unwrap().toggle(if i % 3 == 0 { 0x10 } else { 0x11 }, send.clone());
                }
            })
        };
        let lets_go = {
            let (holds, send) = (holds.clone(), send(&windows));
            std::thread::spawn(move || {
                for _ in 0..20_000 {
                    holds.lock().unwrap().release_all(send.clone());
                }
            })
        };
        taps.join().unwrap();
        lets_go.join().unwrap();
        let mut counted = holds.lock().unwrap().held().to_vec();
        let mut down = windows.lock().unwrap().clone();
        counted.sort();
        down.sort();
        assert_eq!(counted, down);
    }
}
