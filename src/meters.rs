//! The meters: the progress a program the user chose already shows Windows, or the number in a
//! file the user chose. The capsule draws them as small squares under the keys and as rings on the
//! keys; a key whose program shows progress on its taskbar button gets the ring without any meter.

use crate::config::{Action, Item, Meter, Source};
use windows::core::{w, Interface, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationCondition, IUIAutomationElement, IUIAutomationRangeValuePattern,
    IUIAutomationTreeWalker, TreeScope_Descendants, UIA_BoundingRectanglePropertyId, UIA_ControlTypePropertyId, UIA_IsOffscreenPropertyId,
    UIA_IsRangeValuePatternAvailablePropertyId, UIA_ProgressBarControlTypeId, UIA_RangeValuePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, InternalGetWindowText};

/// A number as a title or a file writes it: up to three digits, and a fraction after a point or
/// a comma.
fn number(text: &str) -> Option<f64> {
    let (whole, fraction) = match text.find(['.', ',']) {
        Some(at) => (&text[..at], Some(&text[at + 1..])),
        None => (text, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || whole.len() > 3 || fraction.is_some_and(|f| !digits(f)) {
        return None;
    }
    format!("{whole}.{}", fraction.unwrap_or("0")).parse().ok()
}

/// The percent a window title starts with, as in `36% complete`, `45% - file.zip` or `[45%] …`:
/// a number, a space allowed, a percent sign. A percent further on is something else (the zoom of
/// a page, a name), so a title that does not start with one has no value.
pub fn title_percent(title: &str) -> Option<f64> {
    let rest = title.trim_start();
    let rest = rest.strip_prefix(['[', '(']).unwrap_or(rest);
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ',')).unwrap_or(rest.len());
    let sign = rest[end..].strip_prefix(|c: char| c.is_whitespace()).unwrap_or(&rest[end..]);
    if !sign.starts_with('%') {
        return None;
    }
    number(&rest[..end]).filter(|v| *v <= 100.0)
}

/// The first number in `text`, from where its digits start to where they end, and the text after
/// it; none without a number, or when it has a minus.
fn first_number(text: &str) -> Option<(f64, &str)> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    if text[..start].ends_with('-') {
        return None;
    }
    let rest = &text[start..];
    let whole = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let fraction = rest[whole..]
        .strip_prefix(['.', ','])
        .map(|f| f.find(|c: char| !c.is_ascii_digit()).unwrap_or(f.len()))
        .filter(|&len| len > 0)
        .map_or(0, |len| len + 1);
    let end = whole + fraction;
    Some((rest[..end].replace(',', ".").parse().ok()?, &rest[end..]))
}

/// A file's hint is this many lines at most, each this many characters at most.
const HINT_LINES: usize = 5;
const HINT_WIDTH: usize = 80;

/// What a meter shows: its percent, the outer ring a file may give with it, and the lines of hint
/// a file may give for its label. A meter of a program has neither. `stale` and `notes` are what
/// `dated` makes of the file's times: the meter is drawn grey while the writer has not read what it
/// writes for longer than the file allows, and the label says so and when the numbers reset.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Reading {
    pub value: u8,
    pub outer: Option<u8>,
    pub hint: Vec<String>,
    #[serde(skip)]
    pub times: Times,
    pub stale: bool,
    pub notes: Vec<String>,
}

/// The times a file gives in lines of their own, as seconds since 1970: when its writer last read
/// what it writes (`updated:`, else the file's own time), how many minutes that may be old before
/// the meter is stale (`fresh:`), and when the percent and the outer ring reset (`reset:`,
/// `reset2:`); each key also in Russian.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Times {
    pub updated: Option<i64>,
    pub fresh: Option<f64>,
    pub reset: Option<i64>,
    pub reset2: Option<i64>,
}

impl Reading {
    fn percent(value: f64) -> Reading {
        Reading { value: rounded(value), outer: None, hint: Vec::new(), times: Times::default(), stale: false, notes: Vec::new() }
    }
}

/// How many minutes old a file's numbers may be when it does not say (`fresh:`).
pub const FRESH_MINUTES: f64 = 5.0;

/// A moment as ISO 8601 writes it, `2026-10-26T11:00`, with seconds or not, a space for the `T`
/// allowed: with `Z` or an offset it is that moment; without, the local time of this machine.
pub fn moment(text: &str) -> Option<i64> {
    use chrono::TimeZone;
    let text = text.trim().replacen(' ', "T", 1);
    let text = text.strip_suffix(['Z', 'z']).map_or(text.clone(), |rest| format!("{rest}+00:00"));
    for format in ["%Y-%m-%dT%H:%M:%S%.f%:z", "%Y-%m-%dT%H:%M%:z", "%Y-%m-%dT%H:%M:%S%.f%z", "%Y-%m-%dT%H:%M%z"] {
        if let Ok(at) = chrono::DateTime::parse_from_str(&text, format) {
            return Some(at.timestamp());
        }
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(at) = chrono::NaiveDateTime::parse_from_str(&text, format) {
            return chrono::Local.from_local_datetime(&at).earliest().map(|at| at.timestamp());
        }
    }
    None
}

/// A line of a file that gives one of its times (`Times`), with what it gives; none for any other
/// line, which stays part of the hint. A key whose value cannot be read still takes its line.
fn time_line(line: &str) -> Option<(&'static str, &str)> {
    let (key, value) = line.split_once(':')?;
    let key = match key.trim().to_lowercase().as_str() {
        "updated" | "обновлено" => "updated",
        "fresh" | "свежесть" => "fresh",
        "reset" | "сброс" => "reset",
        "reset2" | "сброс2" => "reset2",
        _ => return None,
    };
    Some((key, value.trim()))
}

/// What a file holds. Its lines that give times (`time_line`) are taken out first. Its first line
/// left that is not blank gives the percent, its first number from 0 to 100, and the outer ring, a
/// second such number right after it; the lines after it that are not blank are the hint. A first
/// line without a percent is no value, whatever follows.
pub fn file_reading(text: &str) -> Option<Reading> {
    let mut times = Times::default();
    let kept: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| {
            let Some((key, value)) = time_line(line) else { return true };
            match key {
                "updated" => times.updated = moment(value).or(times.updated),
                "fresh" => times.fresh = value.replace(',', ".").parse().ok().filter(|m: &f64| m.is_finite() && *m > 0.0).or(times.fresh),
                "reset" => times.reset = moment(value).or(times.reset),
                _ => times.reset2 = moment(value).or(times.reset2),
            }
            false
        })
        .collect();
    let mut lines = kept.into_iter().skip_while(|line| line.is_empty());
    let (value, rest) = first_number(lines.next()?)?;
    if value > 100.0 {
        return None;
    }
    // Only spaces or a percent sign may part the two: in `45/100` or `step 2 of 5: 40 %` the next
    // number is no second percent
    let gap = rest.find(|c: char| !(c.is_whitespace() || c == '%')).unwrap_or(rest.len());
    let outer = (gap > 0 && rest[gap..].starts_with(|c: char| c.is_ascii_digit()))
        .then(|| first_number(&rest[gap..]))
        .flatten()
        .map(|(v, _)| v)
        .filter(|v| *v <= 100.0);
    let hint = lines
        .filter(|line| !line.is_empty())
        .take(HINT_LINES)
        .map(|line| line.chars().take(HINT_WIDTH).collect::<String>().trim_end().to_string())
        .collect();
    let reading = Reading { value: rounded(value), outer: outer.map(rounded), hint, times: Times::default(), stale: false, notes: Vec::new() };
    Some(Reading { times, ..reading })
}

/// A span of time as the label says it: minutes, hours and minutes, or days and hours.
fn span(minutes: i64, russian: bool) -> String {
    let (d, h, m) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
    let [day, hour, min] = if russian { ["д", "ч", "мин"] } else { ["d", "h", "min"] };
    match (d, h, m) {
        (0, 0, m) => format!("{m} {min}"),
        (0, h, 0) => format!("{h} {hour}"),
        (0, h, m) => format!("{h} {hour} {m} {min}"),
        (d, 0, _) => format!("{d} {day}"),
        (d, h, _) => format!("{d} {day} {h} {hour}"),
    }
}

/// When the numbers reset, as the label says it at `now`: the time left, rounded up to a minute,
/// within a day; the day and the local time further on; and once the moment has passed, that the
/// file has not been written anew since. The percent itself stays as the file gives it.
pub fn reset_words(at: i64, now: i64, russian: bool) -> String {
    use chrono::{Datelike, TimeZone};
    let left = at - now;
    if left <= 0 {
        return (if russian { "срок прошёл, ждём обновления" } else { "due, waiting for an update" }).into();
    }
    if left < 24 * 3600 {
        let left = span((left + 59) / 60, russian);
        return if russian { format!("сброс через {left}") } else { format!("resets in {left}") };
    }
    let Some(when) = chrono::Local.timestamp_opt(at, 0).single() else { return String::new() };
    let (day, month, time) = (when.weekday().num_days_from_monday() as usize, when.month0() as usize, when.format("%H:%M"));
    if russian {
        const DAYS: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];
        const MONTHS: [&str; 12] = ["янв", "фев", "мар", "апр", "мая", "июн", "июл", "авг", "сен", "окт", "ноя", "дек"];
        format!("сброс {} {} {}, {time}", DAYS[day], when.day(), MONTHS[month])
    } else {
        const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        format!("resets {} {} {}, {time}", DAYS[day], when.day(), MONTHS[month])
    }
}

/// A reading of a file as it stands at `now`, seconds since 1970: stale when its writer last read
/// what it writes longer ago than the file allows (`FRESH_MINUTES` unless it says), and the lines its
/// label says about time: when the percent and the outer ring reset, and how long ago it was
/// updated, once that is stale. Made anew each round, so the words follow the clock while the file
/// stays as it is.
pub fn dated(mut reading: Reading, now: i64, russian: bool) -> Reading {
    let mut notes = Vec::new();
    if let Some(at) = reading.times.reset {
        notes.push(reset_words(at, now, russian));
    }
    if let Some(at) = reading.times.reset2 {
        notes.push(format!("{} {}", if russian { "обводка:" } else { "outline:" }, reset_words(at, now, russian)));
    }
    let age = reading.times.updated.map(|at| now - at);
    reading.stale = age.is_some_and(|age| age as f64 > reading.times.fresh.unwrap_or(FRESH_MINUTES) * 60.0);
    if let Some(age) = age.filter(|_| reading.stale) {
        let ago = span(age / 60, russian);
        notes.push(if russian { format!("обновлено {ago} назад") } else { format!("updated {ago} ago") });
    }
    reading.notes = notes;
    reading
}

/// The text of a file's start: UTF-16 as Notepad and PowerShell write it, told by its byte-order
/// mark, or UTF-8 with or without one.
pub fn decode(bytes: &[u8]) -> String {
    let utf16 = |rest: &[u8], word: fn([u8; 2]) -> u16| String::from_utf16_lossy(&rest.chunks_exact(2).map(|c| word([c[0], c[1]])).collect::<Vec<_>>());
    match bytes {
        [0xff, 0xfe, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xfe, 0xff, rest @ ..] => utf16(rest, u16::from_be_bytes),
        [0xef, 0xbb, 0xbf, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

const APPS: &str = "shell:appsfolder\\";

/// The app id a `shell:AppsFolder\…` target names, as in `Vendor.Player_8wekyb3d8bbwe!App`.
fn app_of(target: &str) -> Option<&str> {
    let target = target.trim();
    let id = target.get(..APPS.len()).filter(|head| head.eq_ignore_ascii_case(APPS)).map(|_| target[APPS.len()..].trim())?;
    (!id.is_empty()).then_some(id)
}

fn is_exe(path: &str) -> bool {
    let path = path.trim().to_lowercase();
    path.ends_with(".exe") && !path.contains("://")
}

/// A path as two are compared: backslashes, lower case.
fn path_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_lowercase()
}

/// Whether two exe paths are the same program: the same full path, in any case. A target written
/// as a bare file name, as a hand-edited file may have it, has no folder to compare and names
/// every exe of that name.
fn same_exe(a: &str, b: &str) -> bool {
    let (a, b) = (path_key(a), path_key(b));
    let file = |p: &str| p.rsplit('\\').next().unwrap_or("").to_string();
    if a.contains('\\') && b.contains('\\') { a == b } else { file(&a) == file(&b) }
}

/// Whether a target of an `open` item names a program a taskbar button can be told by: an exe or an
/// app.
pub fn watched(target: &str) -> bool {
    app_of(target).is_some() || is_exe(target)
}

/// Whether two targets of the kind an `open` item has name the same program: the same app by its
/// whole `shell:AppsFolder\…` id, or the same exe by its full path (`same_exe`). What a key and a
/// meter share, for the ring.
pub fn same_program(a: &str, b: &str) -> bool {
    match (app_of(a), app_of(b)) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (None, None) => is_exe(a) && is_exe(b) && same_exe(a, b),
        _ => false,
    }
}

/// Whether a window belongs to the program a meter reads: `target` and `app` are the meter's,
/// `path` the full path of the window's program and `window_app` the id of the app the window
/// stands for. An app goes by its whole id, `Family!App`, so two apps of one package stay apart;
/// an exe by its full path, or by the id it gives its windows when the meter knows it, which also
/// takes the windows of another exe the program starts under that id.
pub fn window_of(target: &str, app: Option<&str>, path: &str, window_app: Option<&str>) -> bool {
    let window_app = window_app.map(str::trim).filter(|w| !w.is_empty());
    if let Some(id) = app_of(target) {
        return window_app.is_some_and(|w| w.eq_ignore_ascii_case(id));
    }
    let by_app = app.map(str::trim).filter(|a| !a.is_empty()).zip(window_app).is_some_and(|(a, w)| a.eq_ignore_ascii_case(w));
    is_exe(target) && (same_exe(target, path) || by_app)
}

/// Whether the windows of a meter's program go by the id of the app they stand for, which takes a
/// question to each window: the meter reads an app, or knows the id its program gives its windows.
pub fn by_app(meter: &Meter) -> bool {
    meter.app.is_some() || matches!(&meter.source, Source::Program(target) if app_of(target).is_some())
}

/// The full path a taskbar button names for an exe, `id` being what follows `Appid: `. The button
/// writes a known folder as its id in braces, as in `{1AC14E77-…}\WindowsPowerShell\…`, which
/// `folder` turns into the folder's path; `None` when it cannot, or when the button names no folder.
fn button_path(id: &str, folder: impl Fn(&str) -> Option<String>) -> Option<String> {
    if let Some(rest) = id.strip_prefix('{') {
        let (guid, rest) = rest.split_once('}')?;
        return Some(format!("{}\\{}", folder(guid)?.trim_end_matches(['\\', '/']), rest.trim_start_matches(['\\', '/'])));
    }
    let lower = id.to_lowercase();
    let absolute = matches!(lower.as_bytes(), [drive, b':', b'\\' | b'/', ..] if drive.is_ascii_lowercase()) || lower.starts_with("\\\\");
    absolute.then(|| id.to_string())
}

/// Whether a taskbar button whose AutomationId is `automation_id` belongs to `program`, the
/// target of a meter, or to `app`, the id the program gives its windows when it gives one. The
/// button names its program as `Appid: ` and the path of its exe or that id; `folder` gives the
/// path of a known folder by its id (`win::known_folder`). An exe whose folder cannot be told is
/// matched by its file name alone.
pub fn taskbar_names(automation_id: &str, program: &str, app: Option<&str>, folder: impl Fn(&str) -> Option<String>) -> bool {
    let id = automation_id.trim();
    let Some(id) = id.get(..6).filter(|p| p.eq_ignore_ascii_case("appid:")).map(|_| id[6..].trim()) else { return false };
    if id.is_empty() {
        return false;
    }
    if id.to_lowercase().ends_with(".exe") {
        if !is_exe(program) || app_of(program).is_some() {
            return false;
        }
        return match button_path(id, folder) {
            Some(full) => same_exe(program, &full),
            None => same_exe(program, id.rsplit(['\\', '/']).next().unwrap_or(id)),
        };
    }
    app.is_some_and(|app| app.trim().eq_ignore_ascii_case(id)) || app_of(program).is_some_and(|own| own.eq_ignore_ascii_case(id))
}

/// The rims of the keys whose program shows progress on its taskbar button and has no meter of its
/// own: the target of each such `open` item, with the percent. `taskbar` is what `Reader::taskbar`
/// found. An exe goes by its full path, or by the id the Start menu says its program gives its
/// windows (`apps`), as a meter of it would; a program with a meter keeps the meter's ring.
pub fn rims(items: &[Item], meters: &[Meter], apps: &[crate::apps::App], taskbar: &[(String, f64)], folder: impl Fn(&str) -> Option<String>) -> Vec<(String, u8)> {
    let metered = |target: &str| meters.iter().any(|m| matches!(&m.source, Source::Program(p) if same_program(target, p)));
    let mut found: Vec<(String, u8)> = Vec::new();
    for item in items {
        let Action::Open(target) = &item.action else { continue };
        if !watched(target) || metered(target) || found.iter().any(|(t, _)| t == target) {
            continue;
        }
        let app = apps.iter().find(|a| a.app.is_some() && same_program(&a.target, target)).and_then(|a| a.app.as_deref());
        if let Some((_, value)) = taskbar.iter().find(|(id, _)| taskbar_names(id, target, app, &folder)) {
            found.push((target.clone(), rounded(*value)));
        }
    }
    found
}

/// A file is read this far: its percent and hint are at its start, and a large file is not read
/// whole every second.
const FILE_HEAD: u64 = 4096;

/// What a file holds, with the time it was last written as when it was updated, unless it says.
fn read_file(path: &str) -> Option<Reading> {
    use std::io::Read;
    let mut head = Vec::new();
    let mut file = std::fs::File::open(path).ok()?;
    let written = file.metadata().ok()?.modified().ok();
    (&mut file).take(FILE_HEAD).read_to_end(&mut head).ok()?;
    let mut reading = file_reading(&decode(&head))?;
    let since = |at: std::time::SystemTime| at.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64);
    reading.times.updated = reading.times.updated.or_else(|| written.and_then(since));
    Some(reading)
}

/// What each meter that reads a file shows, by its place among the meters, as it stands now
/// (`dated`), the label's words in Russian or English. Asks nothing of any program, so it never
/// waits for one.
pub fn read_files(meters: &[Meter], russian: bool) -> Vec<(usize, Option<Reading>)> {
    let now = chrono::Utc::now().timestamp();
    let files = meters.iter().enumerate().filter_map(|(i, m)| match &m.source {
        Source::File(path) => Some((i, path)),
        Source::Program(_) => None,
    });
    files.map(|(i, path)| (i, read_file(path).map(|r| dated(r, now, russian)))).collect()
}

fn rounded(v: f64) -> u8 {
    v.clamp(0.0, 100.0).round() as u8
}

/// The values the capsule shows, one per meter of the settings. Two readers fill them, the files
/// and the programs, each in its own thread; the settings may change the list while one of them
/// reads, so each read carries the generation of the list it read, and a read of an older list
/// is dropped.
#[derive(Default)]
pub struct Values {
    generation: u64,
    values: Vec<Option<Reading>>,
    /// Grows with every change of the values, whatever the list: a copy of the values taken for a
    /// drawing may reach the page after a newer event, and the page keeps the newer one by it.
    readings: u64,
}

impl Values {
    /// A new list of `n` meters, with nothing read yet.
    pub fn reset(&mut self, n: usize) {
        self.generation += 1;
        self.values = vec![None; n];
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn values(&self) -> &[Option<Reading>] {
        &self.values
    }

    pub fn readings(&self) -> u64 {
        self.readings
    }

    /// Takes what a read of the list of `generation` found, by place. The values of all the meters
    /// when that changed any; nothing when it changed none or the list is not that one any more.
    pub fn publish(&mut self, generation: u64, found: &[(usize, Option<Reading>)]) -> Option<Vec<Option<Reading>>> {
        if generation != self.generation {
            return None;
        }
        let mut changed = false;
        for (i, value) in found {
            if let Some(slot) = self.values.get_mut(*i).filter(|slot| *slot != value) {
                *slot = value.clone();
                changed = true;
            }
        }
        if changed {
            self.readings += 1;
        }
        changed.then(|| self.values.clone())
    }
}

/// The title from the system's own copy, which asks nothing of the window.
fn title(hwnd: isize) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { InternalGetWindowText(HWND(hwnd as _), &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// The share a progress bar is filled, in percent.
fn percent(bar: &IUIAutomationElement) -> Option<f64> {
    unsafe {
        let range: IUIAutomationRangeValuePattern = bar.GetCurrentPatternAs(UIA_RangeValuePatternId).ok()?;
        let (value, lo, hi) = (range.CurrentValue().ok()?, range.CurrentMinimum().ok()?, range.CurrentMaximum().ok()?);
        (hi > lo && value.is_finite()).then(|| (value - lo) / (hi - lo) * 100.0)
    }
}

/// Reads the meters of programs through UI Automation, and where a docked window draws itself.
/// Each lives on the one thread that asks with it: COM is started there.
pub struct Reader {
    uia: IUIAutomation,
    /// A progress bar that tells its value.
    bars: IUIAutomationCondition,
    walker: IUIAutomationTreeWalker,
    /// An element of a window that is on the screen.
    shown: IUIAutomationCondition,
    /// Questions give up after a second. Without that a program that hangs holds each question
    /// for up to 20 seconds, so its own windows are not asked; the taskbar is Windows' own.
    bounded: bool,
    /// Last, so COM ends after the objects above are gone.
    _com: crate::win::Com,
}

impl Reader {
    pub fn new() -> Result<Reader, String> {
        let com = crate::win::Com::new(COINIT_MULTITHREADED);
        // Right after the panel starts, UI Automation may refuse with E_FAIL; it answers a moment later.
        let mut last = String::new();
        for _ in 0..5 {
            match Self::parts() {
                Ok((uia, bars, walker, shown, bounded)) => return Ok(Reader { uia, bars, walker, shown, bounded, _com: com }),
                Err(e) => last = e,
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
        Err(last)
    }

    fn parts() -> Result<(IUIAutomation, IUIAutomationCondition, IUIAutomationTreeWalker, IUIAutomationCondition, bool), String> {
        let step = |what: &str, e: windows::core::Error| format!("{what}: {e}");
        unsafe {
            let (uia, bounded) = match CoCreateInstance::<_, IUIAutomation2>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) {
                Ok(uia) => {
                    let bounded = uia.SetConnectionTimeout(1000).is_ok() && uia.SetTransactionTimeout(1000).is_ok();
                    (uia.cast::<IUIAutomation>().map_err(|e| step("cast", e))?, bounded)
                }
                Err(_) => (CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(|e| step("create", e))?, false),
            };
            let bar = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(UIA_ProgressBarControlTypeId.0)).map_err(|e| step("bar", e))?;
            let tells = uia.CreatePropertyCondition(UIA_IsRangeValuePatternAvailablePropertyId, &VARIANT::from(true)).map_err(|e| step("tells", e))?;
            let bars = uia.CreateAndCondition(&bar, &tells).map_err(|e| step("and", e))?;
            let walker = uia.RawViewWalker().map_err(|e| step("walker", e))?;
            let shown = uia.CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false)).map_err(|e| step("shown", e))?;
            Ok((uia, bars, walker, shown, bounded))
        }
    }

    /// Whether the questions to programs give up after a second.
    pub fn bounded(&self) -> bool {
        self.bounded
    }

    /// The percent of each meter that reads a program, by its place among the meters; none where
    /// nothing was found. Its taskbar button first, where any program that shows progress there
    /// has it; then a progress bar of its own windows, its dialogs included; then a title that
    /// starts with a percent. `taskbar` is what `taskbar` found in this round.
    pub fn read_programs(&self, meters: &[Meter], taskbar: &[(String, f64)]) -> Vec<(usize, Option<Reading>)> {
        let programs: Vec<(usize, &Meter, &String)> = meters
            .iter()
            .enumerate()
            .filter_map(|(i, m)| match &m.source {
                Source::Program(target) => Some((i, m, target)),
                Source::File(_) => None,
            })
            .collect();
        if programs.is_empty() {
            return Vec::new();
        }
        // A window's app id is read only when some meter goes by one
        let with_app = programs.iter().any(|(_, m, _)| by_app(m));
        let mut windows = None;
        let mut found = Vec::new();
        for (i, meter, target) in programs {
            let on_taskbar = taskbar.iter().find(|(id, _)| taskbar_names(id, target, meter.app.as_deref(), crate::win::known_folder)).map(|(_, v)| *v);
            let value = on_taskbar.or_else(|| {
                let windows = windows.get_or_insert_with(|| crate::win::program_windows(with_app));
                let mine: Vec<isize> = windows.iter().filter(|w| window_of(target, meter.app.as_deref(), &w.path, w.app.as_deref())).map(|w| w.hwnd).collect();
                let bar = if self.bounded { mine.iter().find_map(|&hwnd| self.bar_in(hwnd)) } else { None };
                bar.or_else(|| mine.iter().find_map(|&hwnd| title_percent(&title(hwnd))))
            });
            found.push((i, value.map(Reading::percent)));
        }
        found
    }

    /// The progress bars on the taskbar: the AutomationId of the button each is on, and its
    /// percent. A button has a bar only while its program shows progress.
    pub fn taskbar(&self) -> Vec<(String, f64)> {
        unsafe {
            let Ok(tray) = FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) else { return Vec::new() };
            let Ok(found) = self.uia.ElementFromHandle(tray).and_then(|root| root.FindAll(TreeScope_Descendants, &self.bars)) else { return Vec::new() };
            (0..found.Length().unwrap_or(0))
                .filter_map(|i| {
                    let bar = found.GetElement(i).ok()?;
                    Some((self.button_of(&bar)?, percent(&bar)?))
                })
                .collect()
        }
    }

    /// The AutomationId of the button a bar is on: the nearest element above it that names a
    /// program.
    fn button_of(&self, bar: &IUIAutomationElement) -> Option<String> {
        let mut at = bar.clone();
        for _ in 0..4 {
            at = unsafe { self.walker.GetParentElement(&at) }.ok()?;
            let id = unsafe { at.CurrentAutomationId() }.ok()?.to_string();
            if id.trim_start().to_lowercase().starts_with("appid:") {
                return Some(id);
            }
        }
        None
    }

    /// The rectangles on the screen, in physical px, of every element of a window that is on the
    /// screen, asked in one question: `dock::drawn_part` makes of them where the window draws what
    /// a person sees. Nothing without timeouts: a program that hangs would hold the question.
    pub fn drawn(&self, hwnd: isize) -> Vec<(i32, i32, i32, i32)> {
        if !self.bounded {
            return Vec::new();
        }
        unsafe {
            let asked = self.uia.CreateCacheRequest().and_then(|cache| {
                cache.AddProperty(UIA_BoundingRectanglePropertyId)?;
                self.uia.ElementFromHandle(HWND(hwnd as _))?.FindAllBuildCache(TreeScope_Descendants, &self.shown, &cache)
            });
            let Ok(found) = asked else { return Vec::new() };
            (0..found.Length().unwrap_or(0))
                .filter_map(|i| found.GetElement(i).and_then(|e| e.CachedBoundingRectangle()).ok())
                .map(|r| (r.left, r.top, r.right - r.left, r.bottom - r.top))
                .collect()
        }
    }

    /// The percent of the first progress bar inside a window.
    fn bar_in(&self, hwnd: isize) -> Option<f64> {
        unsafe {
            let root = self.uia.ElementFromHandle(HWND(hwnd as _)).ok()?;
            percent(&root.FindFirst(TreeScope_Descendants, &self.bars).ok()?)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn reader_starts_on_this_machine() {
        if let Err(e) = super::Reader::new() { panic!("{e}"); }
    }

    use super::*;

    #[test]
    fn a_title_gives_the_percent_it_starts_with() {
        for (title, value) in [
            ("36% complete", Some(36.0)),
            ("45% - file.zip", Some(45.0)),
            ("[45%] Render — scene", Some(45.0)),
            ("(7 %) Copying", Some(7.0)),
            ("  12,5\u{a0}% загружено", Some(12.5)),
            ("100%", Some(100.0)),
            ("0% done", Some(0.0)),
            // A percent further on is something else: the zoom of a page, a name
            ("Report — 100% zoom — Browser", None),
            ("Copying: 7 %", None),
            ("Plan 2026 45%", None),
            ("250% zoom", None),
            ("1234%", None),
            ("No percent here 45", None),
            ("% alone", None),
            ("", None),
        ] {
            assert_eq!(title_percent(title), value, "{title}");
        }
    }

    /// The percent a file gives, without its ring and hint.
    fn file_percent(text: &str) -> Option<u8> {
        file_reading(text).map(|r| r.value)
    }

    #[test]
    fn a_file_gives_its_first_number_and_garbage_gives_nothing() {
        for (text, value) in [
            ("42", Some(42)),
            ("  42\r\n", Some(42)),
            ("\r\n\r\n42", Some(42)),
            ("\u{feff}73.5", Some(74)),
            ("progress: 12,5 %", Some(13)),
            ("0", Some(0)),
            ("100", Some(100)),
            ("45/100", Some(45)),
            ("101", None),
            ("-5", None),
            ("abc", None),
            ("", None),
            ("\u{0}\u{1}\u{fffd}", None),
            ("7.", Some(7)),
        ] {
            assert_eq!(file_percent(text), value, "{text:?}");
        }
        // A file that is not there, or a folder: no value and no failure
        assert_eq!(read_file("Z:\\no\\such\\file.txt"), None);
        assert_eq!(read_file(&std::env::temp_dir().to_string_lossy()), None);
        let path = std::env::temp_dir().join(format!("tapka-meter-{}.txt", std::process::id()));
        std::fs::write(&path, [0xff, 0xff, 0x00, b'6', b'0']).unwrap();
        assert_eq!(read_file(&path.to_string_lossy()).map(|r| r.value), Some(60));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_in_utf16_or_with_a_utf8_mark_reads_like_a_plain_one() {
        let le: Vec<u8> = [0xff, 0xfe].into_iter().chain("57 %\r\n".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        let be: Vec<u8> = [0xfe, 0xff].into_iter().chain("57.5".encode_utf16().flat_map(u16::to_be_bytes)).collect();
        assert_eq!(decode(&le), "57 %\r\n");
        assert_eq!(file_percent(&decode(&le)), Some(57));
        assert_eq!(file_percent(&decode(&be)), Some(58));
        assert_eq!(decode(&[0xef, 0xbb, 0xbf, b'9']), "9");
        assert_eq!(decode(b"33"), "33");
        // A broken half of a character at the end of the read is no failure
        assert_eq!(file_percent(&decode(&[0xff, 0xfe, b'8', 0, b'1'])), Some(8));
        let path = std::env::temp_dir().join(format!("tapka-meter-utf16-{}.txt", std::process::id()));
        std::fs::write(&path, &le).unwrap();
        let read = read_files(&[Meter { name: "a".into(), source: Source::File(path.to_string_lossy().into()), app: None }], false);
        assert_eq!(read.iter().map(|(i, r)| (*i, r.as_ref().map(|r| (r.value, r.stale)))).collect::<Vec<_>>(), [(0, Some((57, false)))]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_file_may_give_an_outer_ring_and_lines_of_hint() {
        let reading = |value, outer, hint: &[&str]| Some(Reading { value, outer, hint: hint.iter().map(|l| l.to_string()).collect(), ..Reading::percent(0.0) });
        // One number: as before, no ring and no hint
        assert_eq!(file_reading("42\r\n"), reading(42, None, &[]));
        // Two numbers on the first line, parted by spaces or percent signs
        assert_eq!(file_reading("42 70"), reading(42, Some(70), &[]));
        assert_eq!(file_reading("12,5\t99.6\n"), reading(13, Some(100), &[]));
        assert_eq!(file_reading("40% 85%"), reading(40, Some(85), &[]));
        // A second number out of range, with a minus, or parted by other text gives no ring
        assert_eq!(file_reading("42 170"), reading(42, None, &[]));
        assert_eq!(file_reading("42 -5"), reading(42, None, &[]));
        assert_eq!(file_reading("45/100"), reading(45, None, &[]));
        assert_eq!(file_reading("step 2 of 5: 40 %"), reading(2, None, &[]));
        // The lines after the first are the hint: trimmed, blank ones skipped, five at most, each
        // cut to 80 characters
        let long = "ж".repeat(100);
        let text = format!("42 70\r\n  CPU 42 %  \r\n\r\n   \nGPU 70 %\n{long}\nfour\nfive\nsix\n");
        let cut = "ж".repeat(80);
        assert_eq!(file_reading(&text), reading(42, Some(70), &["CPU 42 %", "GPU 70 %", &cut, "four", "five"]));
        assert_eq!(file_reading("7\nonly a hint"), reading(7, None, &["only a hint"]));
        // A line cut in the middle of a space ends without it
        assert_eq!(file_reading(&format!("7\n{}  x", "a".repeat(79))).unwrap().hint, ["a".repeat(79)]);
        // A first line without a percent is no value, and its hint goes with it
        assert_eq!(file_reading("abc\n42\nhint"), None);
        assert_eq!(file_reading("250 40\nhint"), None);
        // In UTF-16, as PowerShell writes it
        let le: Vec<u8> = [0xff, 0xfe].into_iter().chain("30 60\r\nЗагрузка\r\n".encode_utf16().flat_map(u16::to_le_bytes)).collect();
        assert_eq!(file_reading(&decode(&le)), reading(30, Some(60), &["Загрузка"]));
    }

    #[test]
    fn a_file_may_say_when_it_was_updated_how_fresh_it_must_be_and_when_it_resets() {
        use chrono::TimeZone;
        let local = |y, mo, d, h, mi| chrono::Local.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp();
        let utc = |y, mo, d, h, mi| chrono::Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp();
        // Moments: with an offset or Z that moment, without one the local time; seconds, a fraction
        // and a space for the T allowed; anything else is no moment
        assert_eq!(moment("2026-10-26T11:00:00+03:00"), Some(utc(2026, 10, 26, 8, 0)));
        assert_eq!(moment("2026-10-26T11:00+0300"), Some(utc(2026, 10, 26, 8, 0)));
        assert_eq!(moment("2026-10-26T08:00:00.123Z"), Some(utc(2026, 10, 26, 8, 0)));
        assert_eq!(moment("2026-10-26T08:00Z"), Some(utc(2026, 10, 26, 8, 0)));
        assert_eq!(moment("2026-10-26T11:00"), Some(local(2026, 10, 26, 11, 0)));
        assert_eq!(moment(" 2026-10-26 11:00:00 "), Some(local(2026, 10, 26, 11, 0)));
        for garbage in ["", "soon", "2026-10-26", "26.10.2026 11:00", "2026-13-01T00:00", "11:00"] {
            assert_eq!(moment(garbage), None, "{garbage}");
        }
        // The lines with these keys, in either language and any case, are no hint; the rest stays
        let text = "updated: 2026-10-21T10:00:00Z\nfresh: 15\n42 70\nCPU 42 %\nreset: 2026-10-21T12:33:00Z\nСброс2: 2026-10-26T08:00Z\nGPU";
        let r = file_reading(text).unwrap();
        assert_eq!((r.value, r.outer, r.hint.clone()), (42, Some(70), vec!["CPU 42 %".to_string(), "GPU".to_string()]));
        assert_eq!(r.times, Times { updated: Some(utc(2026, 10, 21, 10, 0)), fresh: Some(15.0), reset: Some(utc(2026, 10, 21, 12, 33)), reset2: Some(utc(2026, 10, 26, 8, 0)) });
        let ru = file_reading("обновлено: 2026-10-21T13:00+03:00\r\nсвежесть: 2,5\r\n7\r\nсброс: никогда").unwrap();
        assert_eq!(ru.times, Times { updated: Some(utc(2026, 10, 21, 10, 0)), fresh: Some(2.5), reset: None, reset2: None });
        assert!(ru.hint.is_empty());
        // A key whose value is garbage still takes its line; a freshness of nothing or below zero is none
        let odd = file_reading("fresh: -3\nupdated: yesterday\n50\nfresh: abc\nnote: kept").unwrap();
        assert_eq!((odd.times.clone(), odd.hint.clone()), (Times::default(), vec!["note: kept".to_string()]));
        // The time lines may come before the percent; time lines alone are no value
        assert_eq!(file_reading("reset: 2026-10-21T12:00Z\n\n33").map(|r| r.value), Some(33));
        assert_eq!(file_reading("updated: 2026-10-21T12:00Z\nreset: 2026-10-21T13:00Z"), None);
    }

    #[test]
    fn a_meter_is_stale_once_older_than_its_file_allows_and_says_so() {
        let now = 1_800_000_000;
        let at = |updated: Option<i64>, fresh: Option<f64>| Reading { times: Times { updated, fresh, ..Times::default() }, ..Reading::percent(42.0) };
        // Five minutes by default: fresh up to them, stale past them, the percent kept
        let fresh = dated(at(Some(now - 300), None), now, true);
        assert!(!fresh.stale && fresh.notes.is_empty());
        let stale = dated(at(Some(now - 301), None), now, true);
        assert!(stale.stale);
        assert_eq!((stale.value, stale.notes.clone()), (42, vec!["обновлено 5 мин назад".to_string()]));
        assert_eq!(dated(at(Some(now - 12 * 60 - 30), None), now, false).notes, ["updated 12 min ago"]);
        assert_eq!(dated(at(Some(now - 2 * 3600 - 5 * 60), None), now, true).notes, ["обновлено 2 ч 5 мин назад"]);
        assert_eq!(dated(at(Some(now - 3 * 86400 - 4 * 3600), None), now, false).notes, ["updated 3 d 4 h ago"]);
        // The file's own freshness
        assert!(!dated(at(Some(now - 14 * 60), Some(15.0)), now, false).stale);
        assert!(dated(at(Some(now - 16 * 60), Some(15.0)), now, false).stale);
        // A writer's clock ahead of this one is fresh; no time at all is fresh too (a program)
        assert!(!dated(at(Some(now + 600), None), now, false).stale);
        assert!(!dated(at(None, None), now, false).stale);
        assert!(!Reading::percent(10.0).stale);
    }

    #[test]
    fn a_reset_is_counted_down_to_the_minute_and_said_as_a_date_further_off() {
        use chrono::TimeZone;
        let now = chrono::Local.with_ymd_and_hms(2026, 10, 21, 8, 27, 0).unwrap().timestamp();
        // Within a day: the time left, rounded up to the minute
        assert_eq!(reset_words(now + 2 * 3600 + 33 * 60, now, true), "сброс через 2 ч 33 мин");
        assert_eq!(reset_words(now + 2 * 3600 + 33 * 60, now, false), "resets in 2 h 33 min");
        assert_eq!(reset_words(now + 2 * 3600 + 32 * 60 + 1, now, false), "resets in 2 h 33 min");
        assert_eq!(reset_words(now + 3 * 3600, now, true), "сброс через 3 ч");
        assert_eq!(reset_words(now + 30, now, false), "resets in 1 min");
        assert_eq!(reset_words(now + 24 * 3600 - 60, now, true), "сброс через 23 ч 59 мин");
        assert_eq!(reset_words(now + 24 * 3600, now, true), "сброс чт 22 окт, 08:27");
        // Further off: the day of the week, the date and the local time
        let monday = chrono::Local.with_ymd_and_hms(2026, 10, 26, 11, 0, 0).unwrap().timestamp();
        assert_eq!(reset_words(monday, now, true), "сброс пн 26 окт, 11:00");
        assert_eq!(reset_words(monday, now, false), "resets Mon 26 Oct, 11:00");
        let may = chrono::Local.with_ymd_and_hms(2027, 5, 1, 9, 5, 0).unwrap().timestamp();
        assert_eq!(reset_words(may, now, true), "сброс сб 1 мая, 09:05");
        // Past it: due, and the percent stays
        assert_eq!(reset_words(now, now, true), "срок прошёл, ждём обновления");
        assert_eq!(reset_words(now - 3600, now, false), "due, waiting for an update");
        // In the label: the gauge's reset, the outline's, then how old a stale file is; the words
        // follow the clock with the file as it was
        let r = file_reading("40 90\nreset: 2026-10-21T10:00\nreset2: 2026-10-26T11:00\nhint").unwrap();
        let r = Reading { times: Times { updated: Some(now - 600), ..r.times.clone() }, ..r };
        let label = dated(r.clone(), now, false);
        assert!(label.stale);
        assert_eq!(label.notes, ["resets in 1 h 33 min", "outline: resets Mon 26 Oct, 11:00", "updated 10 min ago"]);
        assert_eq!((label.value, label.outer, label.hint.clone()), (40, Some(90), vec!["hint".to_string()]));
        let later = dated(r.clone(), now + 60, true);
        assert_eq!(later.notes, ["сброс через 1 ч 32 мин", "обводка: сброс пн 26 окт, 11:00", "обновлено 11 мин назад"]);
        assert_eq!(dated(r, now + 2 * 3600, false).notes[0], "due, waiting for an update");
    }

    #[test]
    fn a_file_without_updated_is_as_old_as_its_last_write_and_one_that_is_gone_is_no_value() {
        let path = std::env::temp_dir().join(format!("tapka-meter-age-{}.txt", std::process::id()));
        std::fs::write(&path, "64\nfresh: 1\n").unwrap();
        let meters = [Meter { name: "a".into(), source: Source::File(path.to_string_lossy().into()), app: None }];
        let now = chrono::Utc::now().timestamp();
        let written = read_file(&path.to_string_lossy()).unwrap();
        assert!((written.times.updated.unwrap() - now).abs() <= 5, "{:?}", written.times);
        assert!(!read_files(&meters, false)[0].1.as_ref().unwrap().stale);
        assert!(dated(written, now + 120, false).stale);
        // The file's own line wins over its time
        std::fs::write(&path, "64\nupdated: 2020-01-01T00:00Z\n").unwrap();
        let old = read_files(&meters, true)[0].1.clone().unwrap();
        assert!(old.stale && old.notes[0].starts_with("обновлено ") && old.value == 64);
        // Garbage, or gone: nothing, which the page draws grey with no percent
        std::fs::write(&path, "updated: 2026-10-21T10:00Z\nnot a number").unwrap();
        assert_eq!(read_files(&meters, false), [(0, None)]);
        let _ = std::fs::remove_file(&path);
        assert_eq!(read_files(&meters, false), [(0, None)]);
    }

    /// The known folders a button may name, as Windows resolves them on a machine with Windows on C:
    fn folder(id: &str) -> Option<String> {
        id.eq_ignore_ascii_case("1AC14E77-02E7-4E5D-B744-2EB1AE5198B7").then(|| "C:\\Windows\\System32".to_string())
    }

    #[test]
    fn a_taskbar_button_is_told_by_its_program_full_path_or_whole_app_id() {
        let shell = "Appid: {1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";
        let names = |id: &str, program: &str, app: Option<&str>| taskbar_names(id, program, app, folder);
        // The known folder is the folder's path: the full path decides, in any case
        assert!(names(shell, "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\PowerShell.exe", None));
        assert!(!names(shell, "D:\\Copy\\WindowsPowerShell\\v1.0\\powershell.exe", None));
        assert!(!names(shell, "C:\\Tools\\shell.exe", None));
        assert!(!names(shell, "shell:AppsFolder\\powershell.exe", None));
        // A folder this machine does not know: the file name alone
        let unknown = "Appid: {00000000-0000-0000-0000-000000000001}\\Tool\\render.exe";
        assert!(names(unknown, "D:\\Apps\\Render.exe", None));
        assert!(!names(unknown, "D:\\Apps\\other.exe", None));
        // A button that names the exe by its full path, and a meter written as a bare file name
        assert!(names("Appid: C:\\Tools\\render.exe", "c:/tools/RENDER.exe", None));
        assert!(!names("Appid: C:\\Tools\\render.exe", "D:\\Tools\\render.exe", None));
        assert!(names(shell, "powershell.exe", None));
        let app = "Appid: Vendor.Player_8wekyb3d8bbwe!App";
        assert!(names(app, "shell:AppsFolder\\vendor.player_8wekyb3d8bbwe!App", None));
        assert!(!names(app, "shell:AppsFolder\\Vendor.Player_8wekyb3d8bbwe!Other", None));
        assert!(!names(app, "shell:AppsFolder\\Vendor.Player_8wekyb3d8bbwe", None));
        assert!(!names(app, "C:\\Tools\\player.exe", None));
        // A desktop program that gives its windows an id of its own: matched by the id the
        // program list knew for it, not by its exe
        let render = "Appid: Vendor.Render";
        assert!(!names(render, "C:\\Tools\\render.exe", None));
        assert!(names(render, "C:\\Tools\\render.exe", Some("vendor.render")));
        assert!(!names(render, "C:\\Tools\\render.exe", Some("Vendor.Other")));
        // A button with no program named, and a meter with no program
        assert!(!names("Start", "powershell.exe", None));
        assert!(!names("Appid: ", "shell:AppsFolder\\", Some("")));
        assert!(!names(shell, "", None));
    }

    #[test]
    fn this_machine_tells_the_folder_a_taskbar_button_names() {
        let windir = std::env::var("WINDIR").unwrap();
        let system = crate::win::known_folder("{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}").unwrap();
        assert!(system.eq_ignore_ascii_case(&format!("{windir}\\System32")), "{system}");
        assert_eq!(crate::win::known_folder("{00000000-0000-0000-0000-000000000001}"), None);
        assert_eq!(crate::win::known_folder("not a folder"), None);
    }

    #[test]
    fn two_programs_of_one_file_name_or_one_package_stay_apart() {
        // A key and a meter of the same program, however its path is written
        assert!(same_program("C:\\Tools\\Render.exe", "c:/tools/render.EXE"));
        assert!(!same_program("C:\\Tools\\Render.exe", "D:\\Other\\render.exe"));
        assert!(same_program("shell:AppsFolder\\Vendor.Player!App", "shell:appsfolder\\vendor.player!app"));
        assert!(!same_program("shell:AppsFolder\\Vendor.Player!App", "shell:AppsFolder\\Vendor.Player!Editor"));
        assert!(!same_program("shell:AppsFolder\\Vendor.Player!App", "C:\\Tools\\player.exe"));
        assert!(!same_program("shell:AppsFolder\\", "shell:AppsFolder\\"));
        assert!(!same_program("https://example.com", "https://example.com"));
        // The windows of a meter's program: by full path for an exe
        let render = "C:\\Tools\\render.exe";
        assert!(window_of(render, None, "C:\\TOOLS\\Render.exe", None));
        assert!(!window_of(render, None, "D:\\Old\\render.exe", None));
        // ...or by the id the program gives its windows, which a helper it starts shares
        assert!(window_of(render, Some("Vendor.Render"), "C:\\Tools\\render-helper.exe", Some("vendor.render")));
        assert!(!window_of(render, Some("Vendor.Render"), "D:\\Old\\render.exe", Some("Vendor.Other")));
        assert!(!window_of(render, None, "C:\\Tools\\render-helper.exe", Some("Vendor.Render")));
        // An app by its whole id, not by its package: another app of the package is not it
        let player = "shell:AppsFolder\\Vendor.Player_8wekyb3d8bbwe!App";
        assert!(window_of(player, None, "C:\\Program Files\\WindowsApps\\player.exe", Some("vendor.player_8wekyb3d8bbwe!app")));
        assert!(!window_of(player, None, "C:\\Program Files\\WindowsApps\\player.exe", Some("Vendor.Player_8wekyb3d8bbwe!Editor")));
        assert!(!window_of(player, None, "C:\\Program Files\\WindowsApps\\player.exe", None));
        // An address names no window
        assert!(!window_of("https://example.com", None, "C:\\Tools\\browser.exe", None));
    }

    #[test]
    fn a_read_of_a_list_the_settings_replaced_is_dropped() {
        let p = |v: f64| Some(Reading::percent(v));
        let mut values = Values::default();
        values.reset(2);
        let first = values.generation();
        // The files' reader and the programs' reader each fill their own meters
        assert_eq!(values.publish(first, &[(1, p(40.0))]), Some(vec![None, p(40.0)]));
        assert_eq!(values.publish(first, &[(0, p(7.0))]), Some(vec![p(7.0), p(40.0)]));
        assert_eq!(values.publish(first, &[(0, p(7.0)), (1, p(40.0))]), None);
        // A file's ring or hint that changed is news, with the percent the same
        let ringed = file_reading("40 10\nCPU");
        assert_eq!(values.publish(first, &[(1, ringed.clone())]), Some(vec![p(7.0), ringed]));
        assert_eq!(values.publish(first, &[(1, file_reading("40 10\nGPU"))]), Some(vec![p(7.0), file_reading("40 10\nGPU")]));
        assert_eq!(values.publish(first, &[(1, file_reading("40 10\nGPU"))]), None);
        // The settings now have another list; a read that began before comes late
        values.reset(1);
        assert_eq!(values.publish(first, &[(0, p(99.0))]), None);
        assert_eq!(values.values(), [None]);
        assert_eq!(values.publish(values.generation(), &[(0, p(5.0)), (3, p(1.0))]), Some(vec![p(5.0)]));
    }

    #[test]
    fn every_change_of_the_values_is_counted_and_nothing_else_is() {
        let p = |v: f64| Some(Reading::percent(v));
        let mut values = Values::default();
        values.reset(1);
        let g = values.generation();
        assert_eq!(values.readings(), 0);
        values.publish(g, &[(0, p(1.0))]);
        assert_eq!(values.readings(), 1);
        // The same read again changes nothing and is not counted; a late read of an old list neither
        values.publish(g, &[(0, p(1.0))]);
        values.publish(g - 1, &[(0, p(9.0))]);
        assert_eq!(values.readings(), 1);
        // A new list keeps counting on, never back
        values.reset(1);
        values.publish(values.generation(), &[(0, p(2.0))]);
        assert_eq!(values.readings(), 2);
    }
}
