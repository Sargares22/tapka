//! The tasks: what agents and scripts say they are working on, one small text file each in a folder
//! they write themselves. The capsule shows a section of them only while there is one. Nothing here
//! writes: a task comes when its file comes and goes when its file goes.

use crate::meters;
use std::path::{Path, PathBuf};

/// A task whose file was not written for this long is stale, unless the file says how long it may
/// be quiet (`fresh:`, in seconds).
const FRESH_SECONDS: i64 = 120;
/// A stale task is shown as stale for this long and then no more: a writer that died must not keep
/// the section open for good. The file stays; a new write brings the task back.
const GONE_SECONDS: i64 = 3600;
/// A folder full of files is not read without end.
const MOST: usize = 100;
/// A file is read this far.
const HEAD: u64 = 4096;
const NAME_WIDTH: usize = 40;
const STATUS_WIDTH: usize = 160;

/// The folder of the task files: `TAPKA_TASKS_DIR`, or `.tapka\tasks` in the user's home. Not the
/// panel's data folder: a program from a package writes to its private copy of `%APPDATA%`, which
/// the panel never sees, and any agent may run in one.
pub fn folder() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("TAPKA_TASKS_DIR").filter(|dir| !dir.is_empty()) {
        return Some(dir.into());
    }
    std::env::var_os("USERPROFILE").filter(|home| !home.is_empty()).map(|home| PathBuf::from(home).join(".tapka").join("tasks"))
}

/// What a task's file says.
#[derive(Debug, Clone, PartialEq)]
struct Said {
    name: String,
    percent: Option<f64>,
    status: String,
    fresh: i64,
    /// How fast the percent grows, in percent a second, if the writer knows it: the bar moves by it
    /// between writes in place of the speed the page works out from the writes it saw.
    rate: Option<f64>,
    /// When the work began, if the file says so; otherwise the file's own birth counts.
    started: Option<i64>,
}

/// A file's lines `name:`, `percent:`, `status:`, `fresh:`, `rate:` and `started:`; `stem` is the file's name, for a task
/// that names itself no other way. A line it does not know is left alone, so a writer may add its
/// own. A text with none of them is a file caught while it is written: `None`.
fn said(stem: &str, text: &str) -> Option<Said> {
    let mut found = Said { name: String::new(), percent: None, status: String::new(), fresh: FRESH_SECONDS, rate: None, started: None };
    let mut any = false;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim().to_lowercase().as_str() {
            "name" => found.name = value.chars().take(NAME_WIDTH).collect(),
            "status" => found.status = value.chars().take(STATUS_WIDTH).collect(),
            "percent" => found.percent = percent(value),
            "fresh" => found.fresh = value.parse().ok().filter(|s| *s > 0).unwrap_or(FRESH_SECONDS),
            "rate" => found.rate = rate(value),
            "started" => found.started = meters::moment(value),
            _ => continue,
        }
        any = true;
    }
    if found.name.is_empty() {
        found.name = stem.chars().take(NAME_WIDTH).collect();
    }
    any.then_some(found)
}

/// A percent as a task gives it: `46`, `46.5`, `46,5 %`. One decimal is kept, for a long task that
/// should still be seen moving; rounded down, so nothing says 100 before it is. A word is no percent:
/// the task shows that it runs without saying how far.
fn percent(text: &str) -> Option<f64> {
    let number: f64 = text.trim_end_matches('%').trim().replace(',', ".").parse().ok()?;
    number.is_finite().then(|| (number.clamp(0.0, 100.0) * 10.0).floor() / 10.0)
}

/// A speed as a task gives it: `0.25`, `0,25 %`. Only a speed forward counts, and no more than the
/// whole bar a second.
fn rate(text: &str) -> Option<f64> {
    let number: f64 = text.trim_end_matches('%').trim().replace(',', ".").parse().ok()?;
    (number.is_finite() && number > 0.0).then(|| number.min(100.0))
}

/// One task file as last read.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    id: String,
    said: Said,
    /// When the file was written, seconds since 1970.
    written: i64,
    /// When the file came to be, seconds since 1970.
    born: i64,
    /// The file's own time and size, which tell a new write, two in one second too.
    stamp: (std::time::SystemTime, u64),
}

/// The task files of `dir`, by file name so that a task keeps its place, each `*.task` file read
/// only when its time or size changed. `known` is the last answer: a file that cannot be read or is
/// caught half written keeps what it said before, so a writer without an atomic rename causes no
/// flicker. A write to a temporary name that is then moved in never shows half a file at all.
pub fn scan(dir: &Path, known: &[Entry]) -> Vec<Entry> {
    let Ok(listing) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = listing
        .flatten()
        .map(|file| file.path())
        .filter(|path| path.extension().is_some_and(|e| e.eq_ignore_ascii_case("task")))
        .collect();
    files.sort();
    files.truncate(MOST);
    files.iter().filter_map(|path| entry(path, known)).collect()
}

fn entry(path: &Path, known: &[Entry]) -> Option<Entry> {
    use std::io::Read;
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let before = known.iter().find(|e| e.id == id);
    let Some(meta) = std::fs::metadata(path).ok() else { return before.cloned() };
    let Some(stamp) = meta.modified().ok().map(|at| (at, meta.len())) else { return before.cloned() };
    if before.is_some_and(|e| e.stamp == stamp) {
        return before.cloned();
    }
    let seconds = |at: std::time::SystemTime| at.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let written = seconds(stamp.0);
    let born = meta.created().map_or(written, seconds);
    let mut head = Vec::new();
    let read = std::fs::File::open(path).and_then(|file| file.take(HEAD).read_to_end(&mut head));
    match read.ok().and_then(|_| said(&id, &meters::decode(&head))) {
        Some(said) => Some(Entry { id, said, written, born, stamp }),
        None => before.cloned(),
    }
}

/// A task as the capsule draws it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Task {
    /// The file's name: the page keeps the last percents of each task by it, for a bar that moves
    /// between writes.
    pub id: String,
    pub name: String,
    pub percent: Option<f64>,
    pub status: String,
    /// The writer's own speed, percent a second.
    pub rate: Option<f64>,
    /// When the file was last written, milliseconds since 1970: the page times the percents by it,
    /// not by when it heard of them, so the twice-a-second reading adds no jitter to their speed.
    pub written: u64,
    pub stale: bool,
    /// How long it has been quiet, in words, once it is stale.
    pub ago: Option<String>,
    /// How long it has run, as `3:07` or `1:02:07`: what the capsule shows when there is no percent.
    pub elapsed: String,
}

/// A time of work as a clock shows it.
fn clock(seconds: i64) -> String {
    let (h, m, s) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// The tasks to show at `now`, seconds since 1970, with the words in Russian or not. Stale past the
/// time a file allows, gone `GONE_SECONDS` after that. Made anew each round, so the words follow the
/// clock and a task goes without anything being written. Those that say how far they are come first
/// and the quiet ones last, as the section may have room for few, by file name within that.
pub fn shown(entries: &[Entry], now: i64, russian: bool) -> Vec<Task> {
    let quiet = |e: &Entry| now - e.written > e.said.fresh;
    let mut alive: Vec<&Entry> = entries.iter().filter(|e| now - e.written <= e.said.fresh + GONE_SECONDS).collect();
    alive.sort_by_key(|e| (quiet(e), e.said.percent.is_none()));
    alive
        .into_iter()
        .map(|e| {
            let age = now - e.written;
            let stale = age > e.said.fresh;
            let ago = stale.then(|| {
                let quiet = meters::span((age / 60).max(1), russian);
                if russian { format!("не обновлялась {quiet}, возможно, зависла") } else { format!("no update for {quiet}, it may have hung") }
            });
            let elapsed = clock((now - e.said.started.unwrap_or(e.born)).max(0));
            Task { id: e.id.clone(), name: e.said.name.clone(), percent: e.said.percent, status: e.said.status.clone(), rate: e.said.rate, written: e.stamp.0.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64), stale, ago, elapsed }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder_of(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tapka-tasks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn now_written(entries: &[Entry]) -> i64 {
        entries[0].written
    }

    #[test]
    fn a_file_says_name_percent_status_and_how_long_it_may_be_quiet() {
        let dir = folder_of("says");
        std::fs::write(dir.join("camera.task"), "name: Камера\r\npercent: 46,57 %\r\nstatus: DJI_0032: 3,6 из 4,0 ГБ\r\nfresh: 30\r\nmine: 1\r\n").unwrap();
        let found = scan(&dir, &[]);
        let at = now_written(&found);
        let tasks = shown(&found, at, true);
        assert_eq!(tasks.len(), 1);
        assert_eq!((tasks[0].name.as_str(), tasks[0].percent, tasks[0].status.as_str()), ("Камера", Some(46.5), "DJI_0032: 3,6 из 4,0 ГБ"));
        assert!(!tasks[0].stale);
        // The quiet it allows counts, not the default
        assert!(shown(&found, at + 31, true)[0].stale);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_task_files_count_and_a_task_keeps_its_place_by_file_name() {
        let dir = folder_of("names");
        std::fs::write(dir.join("b.task"), "percent: 1").unwrap();
        std::fs::write(dir.join("a.TASK"), "percent: 2").unwrap();
        std::fs::write(dir.join("c.task.tmp"), "percent: 3").unwrap();
        std::fs::write(dir.join("notes.txt"), "percent: 4").unwrap();
        let found = scan(&dir, &[]);
        let tasks = shown(&found, now_written(&found), false);
        // A task with no name goes by its file's
        assert_eq!(tasks.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), ["a", "b"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_task_without_a_percent_runs_without_saying_how_far_and_no_percent_passes_its_ends() {
        assert_eq!(said("x", "percent: soon").unwrap().percent, None);
        assert_eq!(said("x", "status: идёт").unwrap().percent, None);
        assert_eq!(said("x", "percent: 250").unwrap().percent, Some(100.0));
        assert_eq!(said("x", "percent: -3").unwrap().percent, Some(0.0));
        assert_eq!(said("x", "percent: 99.99").unwrap().percent, Some(99.9));
    }

    #[test]
    fn a_writer_may_say_how_fast_it_goes_and_only_forward_counts() {
        assert_eq!(said("x", "rate: 0,25 %").unwrap().rate, Some(0.25));
        assert_eq!(said("x", "rate: 0").unwrap().rate, None);
        assert_eq!(said("x", "rate: -1").unwrap().rate, None);
        assert_eq!(said("x", "rate: fast").unwrap().rate, None);
        assert_eq!(said("x", "rate: 500").unwrap().rate, Some(100.0));
        assert_eq!(said("x", "percent: 3").unwrap().rate, None);
    }

    #[test]
    fn a_file_caught_half_written_keeps_what_it_said() {
        let dir = folder_of("half");
        let path = dir.join("job.task");
        std::fs::write(&path, "name: Монтаж\npercent: 40\n").unwrap();
        let before = scan(&dir, &[]);
        std::fs::write(&path, "").unwrap();
        let during = scan(&dir, &before);
        assert_eq!(during, before);
        // With nothing known it is no task yet
        assert!(scan(&dir, &[]).is_empty());
        std::fs::write(&path, "name: Монтаж\npercent: 41\n").unwrap();
        assert_eq!(shown(&scan(&dir, &during), now_written(&during) + 1, true)[0].percent, Some(41.0));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_task_that_went_quiet_says_so_and_then_goes() {
        let entries = vec![Entry { id: "x".into(), said: said("x", "percent: 7").unwrap(), written: 1000, born: 1000, stamp: (std::time::UNIX_EPOCH, 1) }];
        let task = |now| shown(&entries, now, true);
        assert!(!task(1000 + FRESH_SECONDS)[0].stale);
        let quiet = task(1000 + FRESH_SECONDS + 60);
        assert!(quiet[0].stale);
        assert_eq!(quiet[0].ago.as_deref(), Some("не обновлялась 3 мин, возможно, зависла"));
        assert_eq!(quiet[0].percent, Some(7.0));
        assert_eq!(task(1000 + FRESH_SECONDS + GONE_SECONDS).len(), 1);
        assert!(task(1000 + FRESH_SECONDS + GONE_SECONDS + 1).is_empty());
    }

    #[test]
    fn a_task_says_how_long_it_runs_from_its_line_or_from_the_files_birth() {
        let entry = |text: &str| Entry { id: "x".into(), said: said("x", text).unwrap(), written: 5000, born: 4000, stamp: (std::time::UNIX_EPOCH, 1) };
        assert_eq!(shown(&[entry("status: идёт")], 4187, true)[0].elapsed, "3:07");
        assert_eq!(shown(&[entry("status: идёт")], 5000, true)[0].elapsed, "16:40");
        assert_eq!(shown(&[entry("started: 1970-01-01T00:00:00Z")], 3727, true)[0].elapsed, "1:02:07");
        // A clock behind the file's birth says nothing odd
        assert_eq!(shown(&[entry("status: идёт")], 3990, true)[0].elapsed, "0:00");
    }

    #[test]
    fn those_with_a_percent_come_first_and_the_quiet_ones_last() {
        let entry = |id: &str, text: &str, written| Entry { id: id.into(), said: said(id, text).unwrap(), written, born: 0, stamp: (std::time::UNIX_EPOCH, 1) };
        let list = [entry("a", "status: идёт", 1000), entry("b", "percent: 5", 1000), entry("c", "percent: 6", 1), entry("d", "percent: 7", 1000)];
        let names: Vec<String> = shown(&list, 1000, true).into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["b", "d", "a", "c"]);
    }

    #[test]
    fn a_file_removed_is_a_task_gone_and_no_folder_is_no_tasks() {
        let dir = folder_of("gone");
        std::fs::write(dir.join("job.task"), "percent: 1").unwrap();
        let before = scan(&dir, &[]);
        assert_eq!(before.len(), 1);
        std::fs::remove_file(dir.join("job.task")).unwrap();
        assert!(scan(&dir, &before).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(scan(&dir, &before).is_empty());
    }
}
