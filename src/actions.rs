//! What the panel does in response to a tap, separated from the Windows calls that do it.
//! `steps_for` turns a tap into a list of `Step`s; tests compare the list instead of pressing
//! real keys.

use crate::config::Action;

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    HideCapsule,
    SendKeys(String),
    Open(String),
    /// Bring the already running program behind this target to the front.
    Show(String),
    /// Minimize the windows of the program behind this target.
    Minimize(String),
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
    }
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
}
