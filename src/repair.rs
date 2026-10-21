//! Path repair: a program a key, a meter or a docked panel names by the full path of its exe was
//! moved or updated into another folder. The panel looks for the same exe file elsewhere: among
//! the Start menu's programs, the running processes, and the sibling folders of a folder that
//! carries a version. Only a find that is plainly the same installation, a newer version beside the
//! old one, is written by itself; any other is offered. Pure, apart from what the caller hands in.

use crate::config::{to_text, Action, Settings, Source};
use serde_json::Value;
use std::cmp::Ordering;

/// What the look for a program that is gone came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    /// One place, a newer version of the same installation (`same_install`): written by itself.
    Same(String),
    /// One place, anywhere else: maybe another program of the same file name, so it is only offered.
    Elsewhere(String),
    None,
    Several,
}

/// A path as two are compared: backslashes, lower case.
pub fn key(path: &str) -> String {
    path.trim().replace('/', "\\").to_lowercase()
}

fn file_name(path: &str) -> String {
    key(path).rsplit('\\').next().unwrap_or("").to_string()
}

/// The root of a path's drive, as `C:\`.
pub fn root(path: &str) -> String {
    format!("{}\\", key(path).split('\\').next().unwrap_or("").to_uppercase())
}

/// Whether a target is the full path of an exe on a drive: the only kind the panel repairs. An app
/// of the Start menu's list keeps its id wherever its files go; a network path is never gone to,
/// and a bare file name Windows finds by itself.
pub fn repairable(target: &str) -> bool {
    let target = key(target);
    matches!(target.as_bytes(), [drive, b':', b'\\', ..] if drive.is_ascii_lowercase()) && target.ends_with(".exe")
}

/// The exe paths the settings name, each once: the targets of `open` items, the programs of
/// meters, the paths of docked panels.
pub fn targets(settings: &Settings) -> Vec<String> {
    let items = settings.items.iter().filter_map(|it| match &it.action {
        Action::Open(target) => Some(target),
        _ => None,
    });
    let meters = settings.meters.iter().filter_map(|m| match &m.source {
        Source::Program(target) => Some(target),
        Source::File(_) => None,
    });
    let docked = settings.docked.iter().filter_map(|d| d.path.as_ref());
    let mut found: Vec<String> = Vec::new();
    for target in items.chain(meters).chain(docked) {
        if repairable(target) && !found.iter().any(|f| key(f) == key(target)) {
            found.push(target.trim().to_string());
        }
    }
    found
}

/// A version as a folder name carries it: two to six numbers between dots, `v` in front allowed.
fn version(text: &str) -> Option<Vec<u64>> {
    let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
    let parts: Vec<&str> = text.split('.').collect();
    if !(2..=6).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    parts.iter().map(|p| p.parse().ok()).collect()
}

/// A folder name that carries a version: the version alone (`1.2.3`), or a name, a dash or an
/// underscore and the version (`app-1.2.3`, `app_1.2.3`). The name with its dash, lower case,
/// tells the folders of one program from others beside them.
fn versioned(name: &str) -> Option<(String, Vec<u64>)> {
    if let Some(v) = version(name) {
        return Some((String::new(), v));
    }
    let at = name.rfind(['-', '_']).filter(|&at| at > 0)?;
    Some((name[..=at].to_lowercase(), version(&name[at + 1..])?))
}

/// Versions compared as numbers, a missing number counting as 0: `1.10` is newer than `1.9`,
/// `1.2` and `1.2.0` are the same.
fn newer(a: &[u64], b: &[u64]) -> Ordering {
    (0..a.len().max(b.len())).map(|i| a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0))).find(|o| o.is_ne()).unwrap_or(Ordering::Equal)
}

/// Whether `found` is a newer version of the installation `missing` was in: both share the folders
/// down to one with a version, and `found` has there a folder of the same kind with a higher one.
pub fn same_install(missing: &str, found: &str) -> bool {
    let (old, new) = (key(missing), key(found));
    let (old, new): (Vec<&str>, Vec<&str>) = (old.split('\\').collect(), new.split('\\').collect());
    (1..old.len().saturating_sub(1)).any(|at| {
        let (Some((kind, was)), Some(now)) = (versioned(old[at]), new.get(at).filter(|_| at + 1 < new.len())) else { return false };
        old[..at] == new[..at] && versioned(now).is_some_and(|(k, v)| k == kind && newer(&v, &was) == Ordering::Greater)
    })
}

/// What the sibling folders with a version hold of a program that is gone.
#[derive(Debug, Clone, PartialEq)]
pub enum Folder {
    Newer(String),
    /// Two folders of the newest version hold it, or that version is the old one's.
    Several,
    None,
}

/// Where a program that was in a folder with a version is now: the newest sibling folder of the
/// same kind that holds the same exe at the same place under it, or the same exe right inside it.
/// A version older than the missing one is never taken, and one equal to it is not told apart
/// (`1.2` beside `1.2.0`). The nearest folder with a version on the way up from the exe is tried
/// first. `folders` gives the names of the folders in a folder (its path ends with a backslash),
/// `is_file` whether a file is there.
pub fn newer_folder(missing: &str, folders: impl Fn(&str) -> Vec<String>, is_file: impl Fn(&str) -> bool) -> Folder {
    let path = missing.trim().replace('/', "\\");
    let parts: Vec<&str> = path.split('\\').collect();
    let Some(&exe) = parts.last() else { return Folder::None };
    // From the exe's own folder up, never the drive
    for at in (1..parts.len().saturating_sub(1)).rev() {
        let Some((kind, was)) = versioned(parts[at]) else { continue };
        let parent = parts[..at].join("\\");
        let below = parts[at + 1..].join("\\");
        let holding: Vec<(Vec<u64>, String)> = folders(&format!("{parent}\\"))
            .into_iter()
            .filter(|name| !name.eq_ignore_ascii_case(parts[at]))
            .filter_map(|name| versioned(&name).filter(|(k, v)| *k == kind && newer(v, &was).is_ge()).map(|(_, v)| (v, name)))
            .filter_map(|(v, name)| [below.as_str(), exe].iter().map(|inside| format!("{parent}\\{name}\\{inside}")).find(|p| is_file(p)).map(|p| (v, p)))
            .collect();
        let Some(top) = holding.iter().map(|(v, _)| v).max_by(|a, b| newer(a, b)) else { continue };
        let tops: Vec<&String> = holding.iter().filter(|(v, _)| newer(v, top).is_eq()).map(|(_, p)| p).collect();
        if tops.len() > 1 || newer(top, &was).is_eq() {
            return Folder::Several;
        }
        return Folder::Newer(tops[0].clone());
    }
    Folder::None
}

/// What the look found for the exe at `missing`, from the paths of existing exes it gathered: those
/// of the same file name, other than the missing path itself, each path once.
pub fn pick(missing: &str, candidates: impl IntoIterator<Item = String>) -> Found {
    let name = file_name(missing);
    let mut found: Vec<String> = Vec::new();
    for c in candidates {
        if file_name(&c) == name && key(&c) != key(missing) && !found.iter().any(|f| key(f) == key(&c)) {
            found.push(c);
        }
    }
    match found.len() {
        0 => Found::None,
        1 if same_install(missing, &found[0]) => Found::Same(found.remove(0)),
        1 => Found::Elsewhere(found.remove(0)),
        _ => Found::Several,
    }
}

/// Where the exe at `missing` is now: among `others`, the exe paths the Start menu and the running
/// processes give, and in a newer version folder (`newer_folder`); only a file that is there counts.
pub fn find(missing: &str, others: impl IntoIterator<Item = String>, folders: impl Fn(&str) -> Vec<String>, is_file: impl Fn(&str) -> bool) -> Found {
    let newer = match newer_folder(missing, folders, &is_file) {
        Folder::Several => return Found::Several,
        Folder::Newer(path) => Some(path),
        Folder::None => None,
    };
    pick(missing, others.into_iter().chain(newer).filter(|c| is_file(c)))
}

/// What the look asks of the machine.
pub trait Machine {
    fn is_file(&self, path: &str) -> bool;
    /// Whether a drive's root, as `C:\`, is there and can be read.
    fn ready(&self, root: &str) -> bool;
    /// The names of the folders in a folder.
    fn folders(&self, path: &str) -> Vec<String>;
    /// The exe paths of the Start menu's programs.
    fn start_menu(&self) -> Vec<String>;
    /// The full paths of the running processes of a file name.
    fn processes(&self, exe: &str) -> Vec<String>;
}

/// What became of each of `targets` that is gone: one on a drive that is not there or not ready
/// (a card, a stick, a network drive) is not gone, only out of reach, and is left out.
pub fn look(targets: &[String], machine: &impl Machine) -> Vec<(String, Found)> {
    targets
        .iter()
        .filter(|t| machine.ready(&root(t)) && !machine.is_file(t))
        .map(|t| {
            let exe = file_name(t);
            let others = machine.start_menu().into_iter().chain(machine.processes(&exe));
            (t.clone(), find(t, others, |p| machine.folders(p), |p| machine.is_file(p)))
        })
        .collect()
}

/// The paths the look may write by itself: old and new.
pub fn repairs(found: &[(String, Found)]) -> Vec<(String, String)> {
    found.iter().filter_map(|(old, f)| if let Found::Same(new) = f { Some((old.clone(), new.clone())) } else { None }).collect()
}

/// The exe paths of the settings that are gone and were not repaired, each with the one other place
/// it was found in, if one: their keys are grey, and a key's card offers that place.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lost(Vec<(String, Option<String>)>);

impl Lost {
    pub fn of(found: &[(String, Found)]) -> Lost {
        Lost(found.iter().filter_map(|(old, f)| match f {
            Found::Same(_) => None,
            Found::Elsewhere(at) => Some((key(old), Some(at.clone()))),
            Found::None | Found::Several => Some((key(old), None)),
        }).collect())
    }

    pub fn add(&mut self, target: &str) {
        if !self.has(target) {
            self.0.push((key(target), None));
        }
    }

    pub fn has(&self, target: &str) -> bool {
        self.0.iter().any(|(k, _)| *k == key(target))
    }

    /// The one other place the program of `target` was found in.
    pub fn found(&self, target: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| *k == key(target)).and_then(|(_, at)| at.as_deref())
    }

    /// Whether the exe of a lost `target` is there again: then it is lost no more.
    pub fn back(&mut self, target: &str, is_file: impl Fn(&str) -> bool) -> bool {
        let back = self.has(target) && is_file(target.trim());
        if back {
            self.0.retain(|(k, _)| *k != key(target));
        }
        back
    }

    pub fn forget(&mut self, target: &str) {
        self.0.retain(|(k, _)| *k != key(target));
    }
}

/// The settings text with each old path of `repaired` put to its new one: the target of an `open`
/// item, the program of a meter, the path of a docked panel, and no other field. `None` when the
/// text is not a JSON object; the text as it was when nothing in it names an old path.
pub fn with_paths(text: &str, repaired: &[(String, String)]) -> Option<String> {
    let mut v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    v.as_object()?;
    let mut changed = false;
    let mut put = |list: &str, field: &str, open_only: bool| {
        let Some(entries) = v.get_mut(list).and_then(Value::as_array_mut) else { return };
        for entry in entries.iter_mut().filter_map(Value::as_object_mut) {
            if open_only && entry.get("action").and_then(Value::as_str) != Some("open") {
                continue;
            }
            let Some(now) = entry.get(field).and_then(Value::as_str) else { continue };
            if let Some((_, new)) = repaired.iter().find(|(old, _)| key(old) == key(now)) {
                entry.insert(field.into(), Value::from(new.as_str()));
                changed = true;
            }
        }
    };
    put("items", "target", true);
    put("meters", "program", false);
    put("docked", "path", false);
    Some(if changed { to_text(&v) } else { text.to_string() })
}

/// One line of the settings window: the path of `name`'s program was put right.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Notice {
    pub name: String,
    pub path: String,
}

/// The lines about repaired paths. Each shows in the settings window from the first time the
/// window asks for them until the window is next opened; one the window has not asked for yet
/// waits. Kept in memory only.
#[derive(Default)]
pub struct Notices {
    opening: u32,
    /// Each line, and the opening of the window that showed it first.
    list: Vec<(Notice, Option<u32>)>,
}

impl Notices {
    pub fn add(&mut self, notice: Notice) {
        self.list.retain(|(n, _)| n.name != notice.name);
        self.list.push((notice, None));
    }

    /// A new settings window was made: what the last one showed is gone.
    pub fn opened(&mut self) {
        self.opening += 1;
        self.list.retain(|(_, seen)| seen.is_none());
    }

    /// What the settings window shows now.
    pub fn shown(&mut self) -> Vec<Notice> {
        let now = self.opening;
        self.list.iter_mut().filter_map(|(n, seen)| (*seen.get_or_insert(now) == now).then(|| n.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A machine as the tests see it: folders and the files in them, the drives that are there, the
    /// Start menu's programs and the running processes.
    #[derive(Default)]
    struct Disk {
        folders: HashMap<String, Vec<String>>,
        files: Vec<String>,
        drives: Vec<String>,
        menu: Vec<String>,
        running: Vec<String>,
    }

    impl Disk {
        fn new(files: &[&str]) -> Disk {
            let mut folders: HashMap<String, Vec<String>> = HashMap::new();
            for file in files {
                let parts: Vec<&str> = file.split('\\').collect();
                for at in 1..parts.len() - 1 {
                    let list = folders.entry(format!("{}\\", parts[..at].join("\\")).to_lowercase()).or_default();
                    if !list.iter().any(|n| n == parts[at]) {
                        list.push(parts[at].to_string());
                    }
                }
            }
            Disk { folders, files: files.iter().map(|f| f.to_lowercase()).collect(), drives: vec![r"C:\".into(), r"D:\".into()], ..Disk::default() }
        }
        fn newest(&self, missing: &str) -> Folder {
            newer_folder(missing, |p| self.folders(p), |p| self.is_file(p))
        }
    }

    impl Machine for Disk {
        fn is_file(&self, path: &str) -> bool {
            self.files.contains(&path.to_lowercase())
        }
        fn ready(&self, root: &str) -> bool {
            self.drives.iter().any(|d| d.eq_ignore_ascii_case(root))
        }
        fn folders(&self, path: &str) -> Vec<String> {
            self.folders.get(&path.to_lowercase()).cloned().unwrap_or_default()
        }
        fn start_menu(&self) -> Vec<String> {
            self.menu.clone()
        }
        fn processes(&self, exe: &str) -> Vec<String> {
            self.running.iter().filter(|p| file_name(p) == exe).cloned().collect()
        }
    }

    #[test]
    fn only_the_full_path_of_an_exe_on_a_drive_is_repaired() {
        assert!(repairable(r"C:\Tools\paint.exe"));
        assert!(repairable(" d:/Tools/Paint.EXE "));
        for not in [r"shell:AppsFolder\Vendor.Paint_abc!App", "paint.exe", r"\\host\share\paint.exe", r"C:\Docs\plan.txt", "https://example.com/a.exe", r"C:\Tools"] {
            assert!(!repairable(not), "{not}");
        }
        assert_eq!(root(r"e:/Tools/paint.exe"), r"E:\");
    }

    #[test]
    fn a_program_moved_to_a_newer_version_folder_is_found_there() {
        let disk = Disk::new(&[
            r"C:\Apps\Paint\app-1.2.4\Paint.exe",
            r"C:\Apps\Paint\app-1.10.0\Paint.exe",
            r"C:\Apps\Paint\app-1.9.9\Paint.exe",
            // Another program's folders and one without the exe are passed over
            r"C:\Apps\Paint\tool-9.0\Paint.exe",
            r"C:\Apps\Paint\app-2.0\readme.txt",
        ]);
        assert_eq!(disk.newest(r"C:\Apps\Paint\app-1.2.3\Paint.exe"), Folder::Newer(r"C:\Apps\Paint\app-1.10.0\Paint.exe".into()));
        // A folder that is the version alone, the exe deeper under it, an underscore
        let disk = Disk::new(&[r"C:\Apps\Sketch\1.3.0\bin\sketch.exe", r"C:\Apps\Sketch\1.2.9\bin\sketch.exe", r"C:\Apps\Ink\ink_v2.1\ink.exe"]);
        assert_eq!(disk.newest(r"C:\Apps\Sketch\1.2.3\bin\sketch.exe"), Folder::Newer(r"C:\Apps\Sketch\1.3.0\bin\sketch.exe".into()));
        assert_eq!(disk.newest(r"C:\Apps\Ink\ink_v2.0\ink.exe"), Folder::Newer(r"C:\Apps\Ink\ink_v2.1\ink.exe".into()));
        // The same exe right inside the new folder, when the old place under it is not there
        let disk = Disk::new(&[r"C:\Apps\Sketch\1.4\sketch.exe"]);
        assert_eq!(disk.newest(r"C:\Apps\Sketch\1.2.3\bin\sketch.exe"), Folder::Newer(r"C:\Apps\Sketch\1.4\sketch.exe".into()));
        // No version in the path, or no folder of a newer one: nothing
        assert_eq!(disk.newest(r"C:\Apps\Sketch\sketch.exe"), Folder::None);
        assert_eq!(Disk::new(&[r"C:\Apps\Paint\app-1.2.4\other.exe"]).newest(r"C:\Apps\Paint\app-1.2.3\Paint.exe"), Folder::None);
        assert_eq!(Disk::new(&[r"C:\v1.2\Paint.exe"]).newest(r"C:\v1.1\Paint.exe"), Folder::Newer(r"C:\v1.2\Paint.exe".into()));
    }

    #[test]
    fn an_older_version_is_never_taken_and_an_equal_one_is_not_guessed() {
        // Only older folders beside the missing one: nothing
        let disk = Disk::new(&[r"C:\Apps\Paint\app-1.2.2\Paint.exe", r"C:\Apps\Paint\app-1.1\Paint.exe"]);
        assert_eq!(disk.newest(r"C:\Apps\Paint\app-1.2.3\Paint.exe"), Folder::None);
        // The same version under another name
        let disk = Disk::new(&[r"C:\Apps\Paint\app-1.2.0\Paint.exe"]);
        assert_eq!(disk.newest(r"C:\Apps\Paint\app-1.2\Paint.exe"), Folder::Several);
        // Two names of the newest version
        let disk = Disk::new(&[r"C:\Apps\Paint\app-1.3\Paint.exe", r"C:\Apps\Paint\app-1.3.0\Paint.exe", r"C:\Apps\Paint\app-1.2.9\Paint.exe"]);
        assert_eq!(disk.newest(r"C:\Apps\Paint\app-1.2\Paint.exe"), Folder::Several);
        assert_eq!(find(r"C:\Apps\Paint\app-1.2\Paint.exe", Vec::new(), |p| disk.folders(p), |p| disk.is_file(p)), Found::Several);
    }

    #[test]
    fn versions_are_compared_as_numbers() {
        assert_eq!(versioned("app-1.2.3"), Some(("app-".into(), vec![1, 2, 3])));
        assert_eq!(versioned("My_App_v10.0"), Some(("my_app_".into(), vec![10, 0])));
        assert_eq!(versioned("1.3.0"), Some((String::new(), vec![1, 3, 0])));
        for not in ["app", "app-1", "2026", "app-1.2-beta", "-1.2", "1..2", "app-1.2.3.4.5.6.7"] {
            assert_eq!(versioned(not), None, "{not}");
        }
        assert_eq!(newer(&[1, 10], &[1, 9]), Ordering::Greater);
        assert_eq!(newer(&[1, 2], &[1, 2, 0]), Ordering::Equal);
    }

    #[test]
    fn only_a_newer_version_of_the_same_installation_is_the_same_program() {
        let gone = r"C:\Apps\Paint\app-1.2.3\Paint.exe";
        assert!(same_install(gone, r"C:\Apps\Paint\app-1.2.4\Paint.exe"));
        assert!(same_install(gone, r"c:/apps/paint/APP-2.0/bin/paint.exe"));
        assert!(same_install(r"C:\Apps\Sketch\1.2\bin\sketch.exe", r"C:\Apps\Sketch\1.3\sketch.exe"));
        for other in [
            r"D:\Portable\Paint.exe",
            r"C:\Apps\Paint\Paint.exe",
            r"C:\Apps\Paint\tool-2.0\Paint.exe",
            r"C:\Apps\Paint\app-1.2.2\Paint.exe",
            r"C:\Apps\Paint\app-1.2.3.0\Paint.exe",
            r"C:\Other\Paint\app-1.3\Paint.exe",
        ] {
            assert!(!same_install(gone, other), "{other}");
        }
    }

    #[test]
    fn one_place_of_the_same_installation_repairs_one_elsewhere_is_offered_and_two_do_not() {
        let gone = r"C:\Apps\Paint\app-1.2.3\Paint.exe";
        let new = r"C:\Apps\Paint\app-1.2.4\Paint.exe";
        assert_eq!(pick(gone, [new.to_string()]), Found::Same(new.into()));
        // The same path from the Start menu and a running process is one program
        assert_eq!(pick(gone, [new.to_string(), new.to_uppercase(), new.replace('\\', "/")]), Found::Same(new.into()));
        // Another folder: another program of the same file name, maybe
        assert_eq!(pick(gone, [r"D:\Portable\Paint.exe".to_string()]), Found::Elsewhere(r"D:\Portable\Paint.exe".into()));
        // Another file name, or the path that is gone, is not the program
        assert_eq!(pick(gone, [r"C:\Apps\Other\other.exe".to_string(), gone.to_lowercase()]), Found::None);
        assert_eq!(pick(gone, Vec::new()), Found::None);
        assert_eq!(pick(gone, [new.to_string(), r"D:\Portable\Paint.exe".to_string()]), Found::Several);
    }

    #[test]
    fn the_look_writes_a_newer_version_offers_another_folder_and_leaves_a_drive_that_is_not_there() {
        let paint = r"C:\Apps\Paint\app-1.2.3\Paint.exe".to_string();
        let ink = r"C:\Apps\Ink\ink.exe".to_string();
        let card = r"E:\Tools\brush.exe".to_string();
        let here = r"C:\Tools\notes.exe".to_string();
        let mut disk = Disk::new(&[r"C:\Apps\Paint\app-1.3.0\Paint.exe", r"D:\Other\ink.exe", r"C:\Tools\notes.exe"]);
        // The Start menu has another program of the name ink.exe; the drive E: is not plugged in
        disk.menu = vec![r"D:\Other\ink.exe".into(), r"C:\Gone\paint.exe".into()];
        let found = look(&[paint.clone(), ink.clone(), card.clone(), here], &disk);
        assert_eq!(found, [(paint.clone(), Found::Same(r"C:\Apps\Paint\app-1.3.0\Paint.exe".into())), (ink.clone(), Found::Elsewhere(r"D:\Other\ink.exe".into()))]);
        assert_eq!(repairs(&found), [(paint.clone(), r"C:\Apps\Paint\app-1.3.0\Paint.exe".to_string())]);
        let mut lost = Lost::of(&found);
        assert!(!lost.has(&paint) && !lost.has(&card));
        assert!(lost.has(&ink.to_uppercase()));
        assert_eq!(lost.found(&ink), Some(r"D:\Other\ink.exe"));
        // A running process of the name counts too, and with the Start menu's makes two
        disk.running = vec![r"C:\Portable\ink.exe".into()];
        disk.files.push(r"c:\portable\ink.exe".into());
        assert_eq!(look(&[ink.clone()], &disk), [(ink.clone(), Found::Several)]);
        // The exe is back: lost no more
        assert!(!lost.back(&ink, |_| false));
        assert!(lost.back(&ink, |_| true));
        assert!(!lost.has(&ink));
    }

    #[test]
    fn the_targets_are_the_exe_paths_of_keys_meters_and_docked_panels() {
        let (settings, _) = crate::config::parse(
            r#"{ "items": [
                { "name": "Paint", "action": "open", "target": "C:\\Apps\\paint.exe" },
                { "name": "Paint again", "action": "open", "target": "c:/apps/PAINT.exe" },
                { "name": "Notes", "action": "open", "target": "shell:AppsFolder\\Vendor.Notes_abc!App" },
                { "name": "Site", "action": "open", "target": "https://example.com" },
                { "name": "Keys", "action": "hotkey", "keys": "ctrl+v" } ],
              "meters": [ { "name": "Render", "program": "D:\\Render\\render.exe" }, { "name": "Backup", "file": "D:\\b.txt" } ],
              "docked": [ "old.exe", { "exe": "dial.exe", "path": "C:\\Dial\\dial.exe", "class": "D" } ] }"#,
        )
        .unwrap();
        assert_eq!(targets(&settings), [r"C:\Apps\paint.exe", r"D:\Render\render.exe", r"C:\Dial\dial.exe"]);
    }

    #[test]
    fn a_repair_writes_only_the_old_path_and_keeps_the_rest_of_the_file() {
        let text = r#"{
  "note": "mine",
  "items": [
    { "name": "Paint", "icon": "brush.png", "hint": "Draw", "action": "open", "target": "C:\\Apps\\app-1.2.3\\Paint.exe", "only_in": "x.exe", "mine": [1, 2] },
    { "name": "Path as text", "action": "hotkey", "keys": "ctrl+v", "target": "C:\\Apps\\app-1.2.3\\Paint.exe" },
    { "name": "Other", "action": "open", "target": "C:\\Other\\other.exe" }
  ],
  "meters": [
    { "name": "Render", "program": "c:/apps/APP-1.2.3/paint.exe", "app": "Vendor.Paint" }
  ],
  "docked": [
    "paint.exe",
    { "exe": "paint.exe", "path": "C:\\Apps\\app-1.2.3\\Paint.exe", "class": "P" }
  ]
}
"#;
        let new = r"C:\Apps\app-1.2.4\Paint.exe";
        let out = with_paths(text, &[(r"C:\Apps\app-1.2.3\Paint.exe".into(), new.into())]).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let mut want: Value = serde_json::from_str(text).unwrap();
        want["items"][0]["target"] = new.into();
        want["meters"][0]["program"] = new.into();
        want["docked"][1]["path"] = new.into();
        assert_eq!(v, want);
        // Field order kept: the target is where it was
        assert_eq!(v["items"][0].as_object().unwrap().keys().collect::<Vec<_>>(), ["name", "icon", "hint", "action", "target", "only_in", "mine"]);
        // Nothing to repair: the text as it was; not an object: nothing
        assert_eq!(with_paths(text, &[(r"C:\Gone\gone.exe".into(), new.into())]).as_deref(), Some(text));
        assert_eq!(with_paths("[1]", &[]), None);
    }

    #[test]
    fn a_line_about_a_repair_shows_until_the_settings_window_is_next_opened() {
        let mut n = Notices::default();
        let paint = Notice { name: "Paint".into(), path: r"C:\Apps\app-1.2.4\Paint.exe".into() };
        n.add(paint.clone());
        // The window is opened, and shows it as often as it redraws
        n.opened();
        assert_eq!(n.shown(), [paint.clone()]);
        assert_eq!(n.shown(), [paint.clone()]);
        // Opened again: gone
        n.opened();
        assert_eq!(n.shown(), []);
        // A repair while the window is open shows in it at once, and not in the next one
        let ink = Notice { name: "Ink".into(), path: r"C:\Ink\2.0\ink.exe".into() };
        n.add(ink.clone());
        assert_eq!(n.shown(), [ink]);
        n.opened();
        assert_eq!(n.shown(), []);
        // A repair while the window is closed waits for its next opening
        n.add(paint.clone());
        n.opened();
        assert_eq!(n.shown(), [paint]);
    }
}
