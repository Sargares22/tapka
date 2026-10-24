//! The meters of AI limits: how much of its usage limits the Claude Code or the Codex sign-in on
//! this machine has used, asked of Anthropic or OpenAI with that sign-in, and only while a meter
//! names the service. The one place the panel knows other programs, by the owner's word.
//! Behaviour ported from Codenotch, MIT, Copyright (c) 2026 Vinz.

use crate::config::{Meter, Source};
use crate::meters::{dated, rounded, Reading, Times};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

/// A service whose limits a meter shows. Antigravity's two groups of models are two, as their
/// limits are apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Service {
    Claude,
    Chatgpt,
    AntigravityGemini,
    AntigravityClaude,
}

/// The services asked together, a thread each: one run of Antigravity's command tells both its groups.
pub const ASKED: [&[Service]; 3] = [&[Service::Claude], &[Service::Chatgpt], &[Service::AntigravityGemini, Service::AntigravityClaude]];

impl Service {
    pub const ALL: [Service; 4] = [Service::Claude, Service::Chatgpt, Service::AntigravityGemini, Service::AntigravityClaude];

    /// The service a meter names in the settings file: `"limit": "claude"`, `"chatgpt"`,
    /// `"antigravity-gemini"` or `"antigravity-claude"`.
    pub fn named(name: &str) -> Option<Service> {
        Service::ALL.into_iter().find(|s| s.name() == name.trim().to_lowercase())
    }

    fn index(self) -> usize {
        Service::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// The program whose sign-in the service reads, as the person knows it.
    fn program_name(self) -> &'static str {
        match self {
            Service::Claude => "Claude Code",
            Service::Chatgpt => "Codex",
            Service::AntigravityGemini | Service::AntigravityClaude => "Antigravity",
        }
    }

    /// That program's command on this machine, if it is installed.
    fn cli(self) -> Option<PathBuf> {
        match self {
            Service::Claude => claude_cli(),
            Service::Chatgpt => codex_cli(),
            Service::AntigravityGemini | Service::AntigravityClaude => crate::agy::find_agy(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Service::Claude => "claude",
            Service::Chatgpt => "chatgpt",
            Service::AntigravityGemini => "antigravity-gemini",
            Service::AntigravityClaude => "antigravity-claude",
        }
    }

    fn known(self) -> &'static Mutex<Known> {
        match self {
            Service::Claude => &CLAUDE,
            Service::Chatgpt => &CHATGPT,
            Service::AntigravityGemini => &ANTIGRAVITY_GEMINI,
            Service::AntigravityClaude => &ANTIGRAVITY_CLAUDE,
        }
    }
}

/// A window of a limit: the share of it used, in percent, how many minutes it is long, and when it
/// resets, seconds since 1970.
#[derive(Clone, Debug, PartialEq)]
struct Window {
    used: f64,
    minutes: Option<f64>,
    reset: Option<i64>,
}

/// What a service told: its shorter window, the meter's percent, and the longer one, its outer
/// ring; when that was so; and whether it comes from the log of the last Codex run rather than from
/// a question.
#[derive(Clone, Debug, PartialEq)]
struct Found {
    main: Window,
    outer: Option<Window>,
    updated: i64,
    from_log: bool,
}

/// Why the last read gave nothing new.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Trouble {
    SignIn,
    Expired,
    Wait,
    Failed,
}

/// What is known of a service: the last good reading, kept with its own time when a read fails,
/// and why the last read failed.
struct Known {
    found: Option<Found>,
    trouble: Option<Trouble>,
}

static CLAUDE: Mutex<Known> = Mutex::new(Known { found: None, trouble: None });
static CHATGPT: Mutex<Known> = Mutex::new(Known { found: None, trouble: None });
static ANTIGRAVITY_GEMINI: Mutex<Known> = Mutex::new(Known { found: None, trouble: None });
static ANTIGRAVITY_CLAUDE: Mutex<Known> = Mutex::new(Known { found: None, trouble: None });

/// Until when each service is asked every few seconds rather than every five minutes, in seconds
/// since 1970: a sign-in was just started (`sign_in`), so its meter turns live soon after it ends.
static HURRY: [AtomicI64; 4] = [const { AtomicI64::new(0) }; 4];
/// A service *Check again* asked for, until its thread reads it: a flag, not a time, as a run of
/// Antigravity's command may outlast any time given.
static ASK: [AtomicBool; 4] = [const { AtomicBool::new(false) }; 4];
/// The sign-in the panel started for each group of services asked together (`ASKED`), while it
/// may still be running: a second one for the same group is not started.
static SIGNING: Mutex<[Option<std::process::Child>; 3]> = Mutex::new([None, None, None]);
const HURRY_SECS: i64 = 180;

/// A service is asked this often while a meter names it.
const POLL_SECS: i64 = 300;
/// How many minutes a reading asked of a service stays fresh: past two missed reads, so it does
/// not turn grey while the next one is on its way. A reading from Codex's log keeps the default.
const FRESH_MINUTES: f64 = 12.0;
const BACKOFF_BASE_SECS: u64 = 60;
const BACKOFF_CAP_SECS: u64 = 900;
/// Renew the Claude token when this close to its expiry. Under Claude Code's own five minutes: its
/// start renews the token only then, so a launch any earlier does nothing
const RENEW_MARGIN_MS: u64 = 4 * 60 * 1000;
const RENEW_COOLDOWN_MS: u64 = 10 * 60 * 1000;
/// A token that did not renew is tried again, each wait twice the last, an hour apart at most
const RENEW_RETRY_CAP_MS: u64 = 60 * 60 * 1000;
const RENEW_TIMEOUT: Duration = Duration::from_secs(30);
/// Codex's log is read from its end, this far.
const TAIL_BYTES: u64 = 256 * 1024;

const CLAUDE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const CHATGPT_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

/// What each meter of a limit shows, by its place among the meters, as it stands now (`dated`):
/// the last reading of its service, nothing while there has never been one.
pub fn read(meters: &[Meter], russian: bool) -> Vec<(usize, Option<Reading>)> {
    let now = chrono::Utc::now().timestamp();
    let limits = meters.iter().enumerate().filter_map(|(i, m)| match m.source {
        Source::Limit(service) => Some((i, service)),
        _ => None,
    });
    limits
        .map(|(i, service)| {
            let known = service.known().lock().unwrap();
            // The program is looked for only when its sign-in is what is wrong
            let installed = !signs_in(known.trouble) || service.cli().is_some();
            (i, reading(service, &known, installed, russian).map(|r| noted(dated(r, now, russian))))
        })
        .collect()
}

/// Whether what went wrong is the sign-in, which the program's own sign-in puts right.
fn signs_in(trouble: Option<Trouble>) -> bool {
    matches!(trouble, Some(Trouble::SignIn | Trouble::Expired))
}

/// That a reading comes from Codex's log is said only once it is old: while fresh it is as good as
/// asked.
fn noted(mut reading: Reading) -> Reading {
    if !reading.stale {
        reading.hint.clear();
    }
    reading
}

/// A reading of what is known of a service: each window's percent in a row named by its length,
/// whether it comes from Codex's log, and why it is not new. Never read but failed: no percent, only
/// why. When the sign-in is what is wrong and its program is `installed`, a tap on the meter starts
/// the sign-in, and the problem says so.
fn reading(service: Service, known: &Known, installed: bool, russian: bool) -> Option<Reading> {
    let problem = known.trouble.map(|trouble| reason(service, trouble, installed, russian));
    let sign_in = signs_in(known.trouble) && installed;
    let Some(found) = known.found.as_ref() else {
        return Some(Reading { problem: Some(problem?), sign_in, ..Reading::default() });
    };
    let times = Times {
        updated: Some(found.updated),
        fresh: (!found.from_log).then_some(FRESH_MINUTES),
        reset: found.main.reset,
        reset2: found.outer.as_ref().and_then(|w| w.reset),
    };
    Some(Reading {
        value: Some(rounded(found.main.used)),
        outer: found.outer.as_ref().map(|w| rounded(w.used)),
        hint: found.from_log.then(|| (if russian { "по последнему запуску Codex" } else { "from the last Codex run" }).to_string()).into_iter().collect(),
        rows: std::iter::once(&found.main).chain(&found.outer).map(|w| length_words(w.minutes, russian)).collect(),
        times,
        problem,
        sign_in,
        ..Reading::default()
    })
}

/// How long a window is, as its row in the label names it: `5 часов`, `Неделя`, `Месяц`.
fn length_words(minutes: Option<f64>, russian: bool) -> String {
    let Some(minutes) = minutes.filter(|m| m.is_finite() && *m > 0.0).map(|m| m.round() as i64) else {
        return (if russian { "Лимит" } else { "Limit" }).into();
    };
    let count = |n: i64, ru: [&str; 3], en: &str| {
        if !russian {
            return format!("{n} {en}{}", if n == 1 { "" } else { "s" });
        }
        let form = match (n % 10, n % 100) {
            (1, r) if r != 11 => ru[0],
            (2..=4, r) if !(12..=14).contains(&r) => ru[1],
            _ => ru[2],
        };
        format!("{n} {form}")
    };
    let days = (minutes as f64 / 1440.0).round() as i64;
    match minutes {
        m if m < 60 => count(m, ["минута", "минуты", "минут"], "minute"),
        m if m < 1440 => count(m / 60, ["час", "часа", "часов"], "hour"),
        _ if days == 7 => (if russian { "Неделя" } else { "Week" }).into(),
        _ if days == 30 => (if russian { "Месяц" } else { "Month" }).into(),
        _ => count(days, ["день", "дня", "дней"], "day"),
    }
}

fn reason(service: Service, trouble: Trouble, installed: bool, russian: bool) -> String {
    let program = service.program_name();
    match (trouble, russian) {
        (Trouble::SignIn | Trouble::Expired, true) if !installed => format!("{program} не установлен"),
        (Trouble::SignIn | Trouble::Expired, false) if !installed => format!("{program} is not installed"),
        (Trouble::SignIn | Trouble::Expired, true) => "Нажмите, чтобы подключить".into(),
        (Trouble::SignIn | Trouble::Expired, false) => "Tap to connect".into(),
        (Trouble::Wait, true) => "Сервер просит подождать".into(),
        (Trouble::Wait, false) => "The server asks to wait".into(),
        (Trouble::Failed, true) => "Не удалось узнать расход".into(),
        (Trouble::Failed, false) => "Could not read the usage".into(),
    }
}

/// Where a service stands, as the settings window shows it beside its meter and its choice.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "reason")]
pub enum Status {
    /// Signed in: the last read was good, or, never read, its sign-in is on this machine.
    Ok,
    SignIn,
    /// Its program is not installed.
    Missing,
    /// The last read failed for another reason, said in words.
    Error(String),
    /// A meter names it and it has not been read yet: its thread reads it within seconds.
    Checking,
    /// No meter names it and nothing on this machine tells without asking (Antigravity).
    Unknown,
}

/// Where a service stands: by its last read when it has one; never read, `used` by a meter, about to
/// be; else by its sign-in files and its program alone, without asking anything over the network.
pub fn status(service: Service, used: bool, russian: bool) -> Status {
    let (found, trouble) = {
        let known = service.known().lock().unwrap();
        (known.found.is_some(), known.trouble)
    };
    if found && trouble.is_none() {
        return Status::Ok;
    }
    if service.cli().is_none() {
        return Status::Missing;
    }
    match trouble {
        Some(Trouble::SignIn | Trouble::Expired) => Status::SignIn,
        Some(trouble) => Status::Error(reason(service, trouble, true, russian)),
        None if used => Status::Checking,
        None => {
            let home = home();
            let signed = match service {
                Service::Claude => home.and_then(|h| claude_credential(&h.join(".claude"))).map(|c| !c.expires.is_some_and(|at| at <= now_ms())),
                Service::Chatgpt => home.map(|h| chatgpt_credential(&h.join(".codex")).is_some()),
                Service::AntigravityGemini | Service::AntigravityClaude => return Status::Unknown,
            };
            if signed == Some(true) { Status::Ok } else { Status::SignIn }
        }
    }
}

/// Has a service asked again within seconds, and those asked with it: *Check again* in the settings
/// window after a read that failed.
pub fn check_now(service: Service) {
    ASK[service.index()].store(true, Ordering::Relaxed);
}

/// Whether *Check again* asked for any of these services since the last time, which this takes back.
fn asked_again(services: &[Service]) -> bool {
    services.iter().fold(false, |any, s| ASK[s.index()].swap(false, Ordering::Relaxed) || any)
}

/// Whether a sign-in the panel started is still running; one that ended is let go.
fn running(slot: &mut Option<std::process::Child>) -> bool {
    let still = slot.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None)));
    if !still {
        *slot = None;
    }
    still
}

/// The official page that installs the program a service reads: *Install* when it is missing.
pub fn install_page(service: Service) -> &'static str {
    match service {
        Service::Claude => "https://code.claude.com/docs/en/setup",
        Service::Chatgpt => "https://developers.openai.com/codex/cli",
        Service::AntigravityGemini | Service::AntigravityClaude => "https://antigravity.google/",
    }
}

/// Starts the sign-in of a service's own program in a console window of its own, which the person
/// sees and may answer: `claude auth login`, `codex login`, or Antigravity's `agy`, which signs in
/// as it starts. The panel sees no password and no token. Not in a job: it must outlive the panel's
/// question. The service is then asked every few seconds for three minutes, so its meter turns
/// live soon after the sign-in ends. The program it started, for the log; none when one for the
/// same group is still open, which is not started twice; or why it could not.
pub fn sign_in(service: Service, russian: bool) -> Result<Option<PathBuf>, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    let group = ASKED.iter().position(|asked| asked.contains(&service)).unwrap_or(0);
    let mut signing = SIGNING.lock().unwrap();
    if running(&mut signing[group]) {
        return Ok(None);
    }
    let program_name = service.program_name();
    let program = service.cli().ok_or_else(|| if russian { format!("{program_name} не установлен") } else { format!("{program_name} is not installed") })?;
    let mut command = std::process::Command::new(&program);
    command.creation_flags(CREATE_NEW_CONSOLE);
    match service {
        Service::Claude => command.args(["auth", "login"]),
        Service::Chatgpt => command.arg("login"),
        Service::AntigravityGemini | Service::AntigravityClaude => &mut command,
    };
    // Antigravity takes the folder it starts in as its project: one of its own
    let antigravity = matches!(service, Service::AntigravityGemini | Service::AntigravityClaude);
    let dir = if antigravity { Some(std::env::temp_dir().join("tapka-agy")) } else { home() };
    if let Some(dir) = dir.filter(|d| std::fs::create_dir_all(d).is_ok()) {
        command.current_dir(dir);
    }
    // Started from inside a Claude Code session, it would take that session's sign-in, or another
    // account's folder, instead of the browser's
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name == "CLAUDECODE" || name.starts_with("CLAUDE_CODE_") || matches!(name.as_ref(), "CLAUDE_CONFIG_DIR" | "ANTHROPIC_API_KEY" | "ANTHROPIC_AUTH_TOKEN") {
            command.env_remove(&key);
        }
    }
    let child = command.spawn().map_err(|e| if russian { format!("Не удалось запустить {program_name}: {e}") } else { format!("Could not start {program_name}: {e}") })?;
    signing[group] = Some(child);
    let until = chrono::Utc::now().timestamp() + HURRY_SECS;
    ASKED[group].iter().for_each(|s| HURRY[s.index()].store(until, Ordering::Relaxed));
    Ok(Some(program))
}

/// Why a read gave nothing. `Refused` is a 401 or a 403 before the service tells which it means.
#[derive(Debug)]
enum Failure {
    SignIn,
    Expired,
    Refused(u16),
    /// Too many questions: the seconds the reply's Retry-After asks for, 0 without one.
    Limited(u64),
    Failed(String),
}

impl Failure {
    fn trouble(&self) -> Trouble {
        match self {
            Failure::SignIn | Failure::Refused(_) => Trouble::SignIn,
            Failure::Expired => Trouble::Expired,
            Failure::Limited(_) => Trouble::Wait,
            Failure::Failed(_) => Trouble::Failed,
        }
    }

    fn words(&self) -> String {
        match self {
            Failure::SignIn => "no sign-in".into(),
            Failure::Expired => "the token has expired, nothing sent".into(),
            Failure::Refused(code) => format!("sign-in refused (HTTP {code})"),
            Failure::Limited(_) => "too many requests, waiting".into(),
            Failure::Failed(why) => why.clone(),
        }
    }
}

/// Takes what a read gave into what is known of a service: a reading at least as new as the kept
/// one replaces it, a failure keeps it and says why. A line for the log when that changed whether
/// there is a reading or what is wrong; nothing for a read like the last.
fn settle(known: &mut Known, found: Option<Found>, failure: Option<&Failure>) -> Option<String> {
    let before = (known.found.is_some(), known.trouble);
    if let Some(found) = found.filter(|f| known.found.as_ref().is_none_or(|k| f.updated >= k.updated)) {
        known.found = Some(found);
    }
    known.trouble = failure.map(Failure::trouble);
    if (known.found.is_some(), known.trouble) == before {
        return None;
    }
    Some(match failure {
        None => "read".into(),
        Some(f) if known.found.is_some() => format!("{}; the last reading kept", f.words()),
        Some(f) => f.words(),
    })
}

/// The wait after a run of `limited` refusals for too many questions in a row before this one:
/// a minute, doubling, a quarter of an hour at most; the reply's own Retry-After only makes it longer.
fn backoff_secs(limited: u32, retry_after: u64) -> u64 {
    BACKOFF_BASE_SECS.saturating_mul(1 << limited.min(4)).clamp(BACKOFF_BASE_SECS, BACKOFF_CAP_SECS).max(retry_after)
}

/// Asks services that are asked together (`ASKED`), on a thread of their own, every five minutes
/// while `used` says a meter names one of them: soon after the panel starts and soon after such a
/// meter is added; one question at a time. What it finds is kept for `read`. `log` gets what
/// changed, never a token.
pub fn watch(services: &[Service], used: impl Fn(Service) -> bool, log: impl Fn(&str)) {
    let agent = agent();
    let say = |service: Service, line: &str| log(&format!("limits: {}: {line}", service.name()));
    // When the next read is due, and the wait asked for after too many questions, in seconds
    let (mut next, mut held, mut limited, mut last) = (0i64, 0i64, 0u32, 0i64);
    let mut renewer = Renewer::default();
    loop {
        std::thread::sleep(Duration::from_secs(5));
        if !services.iter().any(|s| used(*s)) {
            next = 0;
            continue;
        }
        if services.contains(&Service::Claude) && renewer.renew(&|line: &str| say(Service::Claude, line)) == Some(true) {
            // A fresh token deserves a fresh try, but not inside a wait the service asked for
            next = 0;
        }
        // Check again: the next read is now, after the one that may have been running, but not
        // inside a wait the service asked for
        if asked_again(services) {
            next = 0;
        }
        let now = chrono::Utc::now().timestamp();
        // Just after a sign-in started: every few seconds, Antigravity's slow command less often
        if services.iter().any(|s| HURRY[s.index()].load(Ordering::Relaxed) > now) {
            next = next.min(last + if services.contains(&Service::Claude) || services.contains(&Service::Chatgpt) { 5 } else { 30 });
        }
        if now < next.max(held) {
            continue;
        }
        (next, last) = (now + POLL_SECS, now);
        let read: Vec<(Service, Option<Found>, Option<Failure>)> = match services[0] {
            Service::Claude => match home().ok_or(Failure::SignIn).and_then(|h| read_claude(&agent, &h.join(".claude"))) {
                Ok(found) => vec![(Service::Claude, Some(found), None)],
                Err(f) => vec![(Service::Claude, None, Some(f))],
            },
            Service::Chatgpt => {
                let (found, failure) = read_chatgpt(&agent);
                vec![(Service::Chatgpt, found, failure)]
            }
            Service::AntigravityGemini | Service::AntigravityClaude => read_antigravity(crate::agy::read_quota(), now),
        };
        if let Some(after) = read.iter().find_map(|(_, _, f)| if let Some(Failure::Limited(after)) = f { Some(*after) } else { None }) {
            held = now + backoff_secs(limited, after) as i64;
            limited += 1;
        } else {
            (held, limited) = (0, 0);
        }
        for (service, found, failure) in read {
            if found.is_some() && failure.is_none() {
                HURRY[service.index()].store(0, Ordering::Relaxed);
            }
            if let Some(line) = settle(&mut service.known().lock().unwrap(), found, failure.as_ref()) {
                say(service, &line);
            }
        }
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from).filter(|p| !p.as_os_str().is_empty())
}

fn agent() -> ureq::Agent {
    let builder = ureq::AgentBuilder::new().timeout(Duration::from_secs(15)).try_proxy_from_env(true);
    match native_tls::TlsConnector::new() {
        Ok(tls) => builder.tls_connector(std::sync::Arc::new(tls)),
        Err(_) => builder,
    }
    .build()
}

/// One question to a service, its reply as JSON.
fn ask(agent: &ureq::Agent, url: &str, headers: &[(&str, &str)]) -> Result<Value, Failure> {
    let request = headers.iter().fold(agent.get(url), |request, (key, value)| request.set(key, value));
    match request.call() {
        Ok(reply) => reply.into_json().map_err(|e| Failure::Failed(format!("the reply is not JSON ({e})"))),
        Err(ureq::Error::Status(code @ (401 | 403), _)) => Err(Failure::Refused(code)),
        Err(ureq::Error::Status(429, reply)) => Err(Failure::Limited(reply.header("retry-after").and_then(|s| s.trim().parse().ok()).unwrap_or(0))),
        Err(ureq::Error::Status(code, _)) => Err(Failure::Failed(format!("HTTP {code}"))),
        Err(e) => Err(Failure::Failed(e.to_string())),
    }
}

/// A moment as the services write it, RFC 3339, in seconds since 1970.
fn instant(v: &Value) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(v.as_str()?).ok().map(|at| at.timestamp())
}

/// The reading of a service's two windows: the shorter one is the percent and the longer one the
/// outer ring; with the longer one alone, it is the percent.
fn pair(main: Option<Window>, outer: Option<Window>, updated: i64, from_log: bool) -> Option<Found> {
    let (main, outer) = match (main, outer) {
        (Some(main), outer) => (main, outer),
        (None, Some(outer)) => (outer, None),
        (None, None) => return None,
    };
    Some(Found { main, outer, updated, from_log })
}

// ---------- Claude

/// Claude Code's sign-in: its token, and when the token expires, in ms since 1970.
struct Credential {
    token: String,
    expires: Option<u64>,
}

/// Claude Code's own credential file, read only. An empty token is signed out, not expired.
fn claude_credential(dir: &Path) -> Option<Credential> {
    [".credentials.json", "credentials.json"].iter().find_map(|name| {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(name)).ok()?).ok()?;
        let oauth = v.get("claudeAiOauth").unwrap_or(&v);
        let token = oauth.get("accessToken")?.as_str()?.trim();
        (!token.is_empty()).then(|| Credential { token: token.to_string(), expires: oauth.get("expiresAt").and_then(Value::as_f64).map(|ms| ms as u64) })
    })
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

/// Asks Anthropic with Claude Code's sign-in. An expired token is never sent: the service answers it
/// with 429 and an hour's Retry-After, not 401, which would read as too many questions for as long
/// as the token stays old. A refused token is read again once, as Claude Code may just have renewed it.
fn read_claude(agent: &ureq::Agent, dir: &Path) -> Result<Found, Failure> {
    let expired = |c: &Credential| c.expires.is_some_and(|at| at <= now_ms());
    let credential = claude_credential(dir).ok_or(Failure::SignIn)?;
    if expired(&credential) {
        return Err(Failure::Expired);
    }
    let ask_with = |token: &str| ask(agent, CLAUDE_URL, &[("Authorization", &format!("Bearer {token}")), ("anthropic-beta", "oauth-2025-04-20")]);
    let reply = match ask_with(&credential.token) {
        Err(Failure::Refused(401)) => match claude_credential(dir) {
            Some(again) if again.token != credential.token && expired(&again) => Err(Failure::Expired),
            Some(again) if again.token != credential.token => ask_with(&again.token),
            _ => Err(Failure::SignIn),
        },
        other => other,
    };
    // A 403 is access denied, not proof the sign-in is gone
    let reply = reply.map_err(|f| match f {
        Failure::Refused(401) => Failure::SignIn,
        Failure::Refused(code) => Failure::Failed(format!("HTTP {code}")),
        f => f,
    })?;
    claude_found(&reply, chrono::Utc::now().timestamp()).ok_or_else(|| Failure::Failed("the reply has no limits".into()))
}

/// Anthropic's reply: the windows in `limits`, by kind, and `five_hour` and `seven_day` for a window
/// that just rolled over and left `limits`. The five hours are the percent, the week the outer ring.
fn claude_found(v: &Value, now: i64) -> Option<Found> {
    let listed = |kinds: &[&str]| {
        v.get("limits")?.as_array()?.iter().find_map(|l| {
            if !kinds.contains(&l.get("kind")?.as_str()?) {
                return None;
            }
            // As Claude's own client: a window without a reset time is not shown
            Some(Window { used: l.get("percent")?.as_f64()?, minutes: None, reset: Some(instant(l.get("resets_at")?)?) })
        })
    };
    let named = |field: &str| {
        let w = v.get(field)?;
        Some(Window { used: w.get("utilization")?.as_f64()?, minutes: None, reset: w.get("resets_at").and_then(instant) })
    };
    let session = listed(&["session", "five_hour"]).or_else(|| named("five_hour")).map(|w| Window { minutes: Some(300.0), ..w });
    let week = listed(&["weekly_all", "seven_day", "weekly"]).or_else(|| named("seven_day")).map(|w| Window { minutes: Some(7.0 * 1440.0), ..w });
    pair(session, week, now, false)
}

/// Anything under these belongs to the Claude desktop app, whose Claude Code keeps its token in the
/// app's own store and never writes the file read here: renewing with it would change nothing.
fn desktop_owned(path: &Path) -> bool {
    let path = path.to_string_lossy().to_lowercase().replace('/', "\\");
    path.contains("\\anthropicclaude\\") || path.contains("\\claude\\claude-code\\") || path.contains("\\windowsapps\\")
}

/// The standalone Claude Code command: where its own installer puts it, then npm, pnpm, Volta and
/// Bun, then the PATH.
fn claude_cli() -> Option<PathBuf> {
    let var = |key: &str| std::env::var_os(key).map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = [
        home().map(|h| h.join(".local").join("bin")),
        var("APPDATA").map(|d| d.join("npm")),
        var("LOCALAPPDATA").map(|d| d.join("pnpm")),
        home().map(|h| h.join(".volta").join("bin")),
        home().map(|h| h.join(".bun").join("bin")),
    ]
    .into_iter()
    .flatten()
    .collect();
    dirs.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
    dirs.iter().flat_map(|d| [d.join("claude.exe"), d.join("claude.cmd")]).find(|p| p.is_file() && !desktop_owned(p))
}

/// Codex's command: where its installer puts it, its own folder, npm, then the PATH.
fn codex_cli() -> Option<PathBuf> {
    let var = |key: &str| std::env::var_os(key).map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = [
        var("LOCALAPPDATA").map(|d| d.join("Programs").join("OpenAI").join("Codex").join("bin")),
        home().map(|h| h.join(".codex").join("bin")),
        var("APPDATA").map(|d| d.join("npm")),
    ]
    .into_iter()
    .flatten()
    .collect();
    dirs.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
    dirs.iter().flat_map(|d| [d.join("codex.exe"), d.join("codex.cmd")]).find(|p| p.is_file())
}

/// Whether a launch to renew the token is worth making: the token expires within the margin, or
/// has, and the last launch is long enough ago. A launch that left the same expiry is tried again
/// on a doubling wait, so one failed launch does not stop the meter and a token that cannot renew
/// is not a launch every round.
fn should_renew(expires: Option<u64>, now: u64, attempted_for: Option<u64>, last_attempt: Option<u64>, failures: u32) -> bool {
    let Some(expires) = expires else { return false };
    if expires > now + RENEW_MARGIN_MS {
        return false;
    }
    let Some(last) = last_attempt else { return true };
    let wait = if attempted_for == Some(expires) { RENEW_COOLDOWN_MS.saturating_mul(1 << failures.min(16)).min(RENEW_RETRY_CAP_MS) } else { RENEW_COOLDOWN_MS };
    now.saturating_sub(last) >= wait
}

#[derive(Default)]
struct Renewer {
    attempted_for: Option<u64>,
    last_attempt: Option<u64>,
    /// Launches in a row that left `attempted_for` where it was
    failures: u32,
}

impl Renewer {
    /// Renews the token when it is about to expire. Some(true) when its expiry moved: judged by the
    /// file, never by the exit code, as `claude -p` without a prompt exits with an error after it
    /// renewed.
    fn renew(&mut self, say: &impl Fn(&str)) -> Option<bool> {
        let dir = home()?.join(".claude");
        let credential = claude_credential(&dir)?;
        let now = now_ms();
        if !should_renew(credential.expires, now, self.attempted_for, self.last_attempt, self.failures) {
            return None;
        }
        if self.attempted_for != credential.expires {
            self.failures = 0;
        }
        self.last_attempt = Some(now);
        self.attempted_for = credential.expires;
        self.failures = self.failures.saturating_add(1);
        let Some(cli) = claude_cli() else {
            say("the token is about to expire and no standalone claude command was found to renew it");
            return Some(false);
        };
        if let Err(e) = run_renewal(&cli, &dir) {
            say(&format!("could not start {} to renew the token ({e})", cli.display()));
            return Some(false);
        }
        let renewed = matches!((claude_credential(&dir).and_then(|c| c.expires), credential.expires), (Some(after), Some(before)) if after > before);
        say(&if renewed { format!("token renewed by {}", cli.display()) } else { format!("ran {}, the token's expiry did not move", cli.display()) });
        Some(renewed)
    }
}

/// `claude -p` with an empty input starts, which is where it renews an aged token, and exits for
/// want of a prompt: no conversation, no transcript. Its output goes nowhere, as a token could be
/// in it. No window; given up after `RENEW_TIMEOUT`, with the node it may have started. It runs in
/// a job, so it ends with the panel too.
fn run_renewal(cli: &Path, dir: &Path) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new(cli);
    command.arg("-p").stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).creation_flags(CREATE_NO_WINDOW);
    // Started from inside a Claude Code session, it would take the session's sign-in and leave the file alone
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy();
        if name == "CLAUDECODE" || name.starts_with("CLAUDE_CODE_") {
            command.env_remove(&key);
        }
    }
    // The account renewed is the one read here, not one a shell pointed the panel at
    command.env("CLAUDE_CONFIG_DIR", dir);
    let job = crate::agy::Job::new().map_err(std::io::Error::other)?;
    let mut child = command.spawn()?;
    // Put in the job as soon as it exists, so the node a claude.cmd starts under cmd is in it too
    if let Err(e) = job.assign(windows::Win32::Foundation::HANDLE(child.as_raw_handle())) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(std::io::Error::other(e));
    }
    let deadline = std::time::Instant::now() + RENEW_TIMEOUT;
    while child.try_wait()?.is_none() {
        if std::time::Instant::now() >= deadline {
            job.terminate();
            let _ = child.wait();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

// ---------- ChatGPT (Codex)

/// Asks OpenAI with Codex's sign-in; when that gives nothing, the limits Codex wrote into the log of
/// its last run stand in, as old as that line. The failure of the question goes along either way.
fn read_chatgpt(agent: &ureq::Agent) -> (Option<Found>, Option<Failure>) {
    let Some(codex) = home().map(|h| h.join(".codex")) else { return (None, Some(Failure::SignIn)) };
    match read_chatgpt_live(agent, &codex) {
        Ok(found) => (Some(found), None),
        Err(failure) => {
            let logged = newest_rollout(&codex.join("sessions")).and_then(|path| tail(&path)).and_then(|text| rollout_found(&text));
            (logged, Some(failure))
        }
    }
}

/// Codex's sign-in, read only and never renewed here: Codex renews it itself.
fn chatgpt_credential(codex: &Path) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(codex.join("auth.json")).ok()?).ok()?;
    let tokens = v.get("tokens")?;
    let token = tokens.get("access_token")?.as_str()?.trim();
    let account = tokens.get("account_id")?.as_str()?.trim();
    (!token.is_empty() && !account.is_empty()).then(|| (token.to_string(), account.to_string()))
}

fn read_chatgpt_live(agent: &ureq::Agent, codex: &Path) -> Result<Found, Failure> {
    let (token, account) = chatgpt_credential(codex).ok_or(Failure::SignIn)?;
    let headers = [
        ("Authorization", format!("Bearer {token}")),
        ("ChatGPT-Account-Id", account),
        ("Accept", "application/json".into()),
        ("Cache-Control", "no-cache, no-store".into()),
        ("User-Agent", concat!("tapka/", env!("CARGO_PKG_VERSION"), " (Windows)").into()),
    ];
    let headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let reply = ask(agent, CHATGPT_URL, &headers)?;
    let now = chrono::Utc::now().timestamp();
    chatgpt_found(&reply, now).ok_or_else(|| Failure::Failed("the reply has no limits".into()))
}

/// A window as Codex gives it: the percent used, its length, and its reset as a moment (`at`) or
/// as seconds after `now` (`after`). None without a percent.
fn codex_window(w: &Value, minutes: Option<f64>, at: &str, after: &str, now: i64) -> Option<Window> {
    let w = w.as_object()?;
    // Garbage in a reply must not wrap a time around
    let seconds = |key: &str| w.get(key)?.as_f64().filter(|s| s.is_finite() && (0.0..1e11).contains(s)).map(|s| s as i64);
    Some(Window { used: w.get("used_percent")?.as_f64()?, minutes, reset: seconds(at).or_else(|| seconds(after).map(|s| now + s)) })
}

/// OpenAI's reply: `rate_limit.primary_window` is the percent, `secondary_window` the outer ring.
/// The primary window is not always five hours, so each is named by its length.
fn chatgpt_found(v: &Value, now: i64) -> Option<Found> {
    let window = |key: &str| {
        let w = v.get("rate_limit")?.get(key)?;
        codex_window(w, w.get("limit_window_seconds").and_then(Value::as_f64).map(|s| s / 60.0), "reset_at", "reset_after_seconds", now)
    };
    pair(window("primary_window"), window("secondary_window"), now, false)
}

/// The newest log of a Codex run, `sessions/YYYY/MM/DD/rollout-*.jsonl`, among the three newest days
/// that have a folder: by the time it was written.
fn newest_rollout(sessions: &Path) -> Option<PathBuf> {
    let dirs = |p: &Path| {
        let mut found: Vec<PathBuf> = std::fs::read_dir(p).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
        found.sort_by(|a, b| b.cmp(a));
        found
    };
    let days: Vec<PathBuf> = dirs(sessions).iter().flat_map(|y| dirs(y)).flat_map(|m| dirs(&m)).take(3).collect();
    days.iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl")))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(written, _)| *written)
        .map(|(_, path)| path)
}

fn tail(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL_BYTES))).ok()?;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw).ok()?;
    Some(String::from_utf8_lossy(&raw).into_owned())
}

/// The last limits a Codex log holds: its last line with `rate_limits` of Codex's own bucket and a
/// time, which is when the reading was so.
fn rollout_found(text: &str) -> Option<Found> {
    text.lines().rev().filter(|line| line.contains("rate_limits")).find_map(|line| {
        let v: Value = serde_json::from_str(line).ok()?;
        let limits = v.get("rate_limits").or_else(|| v.pointer("/payload/rate_limits")).filter(|l| l.is_object())?;
        // Codex writes a line per bucket: another one, Spark say, must not stand in for its own
        if limits.get("limit_id").or_else(|| limits.get("limitId")).and_then(Value::as_str).is_some_and(|id| id != "codex") {
            return None;
        }
        // A reading of no known age would never turn grey
        let at = instant(v.get("timestamp")?)?;
        let window = |key: &str| {
            let w = limits.get(key)?;
            codex_window(w, w.get("window_minutes").and_then(Value::as_f64), "resets_at", "resets_in_seconds", at)
        };
        pair(window("primary"), window("secondary"), at, true)
    })
}

// ---------- Antigravity

/// What one run of Antigravity's command tells each of its two services: the five hours and the
/// week of its group of models, by the group's name; a run that failed fails both.
fn read_antigravity(quota: Result<Vec<crate::agy::Lane>, String>, now: i64) -> Vec<(Service, Option<Found>, Option<Failure>)> {
    [Service::AntigravityGemini, Service::AntigravityClaude]
        .into_iter()
        .map(|service| {
            let found = quota.as_ref().map_err(|why| Failure::Failed(why.clone())).and_then(|lanes| {
                let mine = |lane: &&crate::agy::Lane| {
                    let group = lane.group.to_lowercase();
                    if service == Service::AntigravityGemini { group.starts_with("gemini") } else { group.contains("claude") }
                };
                let window = |five_hour: bool| {
                    let lane = lanes.iter().filter(mine).find(|l| l.five_hour == five_hour)?;
                    Some(Window { used: lane.used, minutes: Some(if five_hour { 300.0 } else { 7.0 * 1440.0 }), reset: Some(lane.reset) })
                };
                pair(window(true), window(false), now, false).ok_or_else(|| Failure::Failed("the quota report has no such group".into()))
            });
            match found {
                Ok(found) => (service, Some(found), None),
                Err(failure) => (service, None, Some(failure)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn one_run_of_antigravity_feeds_both_its_groups_and_a_failed_run_fails_both() {
        let lane = |group: &str, five_hour, used, reset| crate::agy::Lane { group: group.into(), five_hour, used, reset };
        let lanes = vec![lane("Gemini Models", false, 6.0, 900), lane("Gemini Models", true, 22.0, 100), lane("Claude and GPT models", true, 40.0, 200)];
        let read = read_antigravity(Ok(lanes), 50);
        let (gemini, claude) = (read[0].1.clone().unwrap(), read[1].1.clone().unwrap());
        assert_eq!((read[0].0, gemini.main, gemini.outer), (Service::AntigravityGemini, win(22.0, 300.0, Some(100)), Some(win(6.0, 10080.0, Some(900)))));
        assert_eq!((read[1].0, claude.main, claude.outer, claude.updated), (Service::AntigravityClaude, win(40.0, 300.0, Some(200)), None, 50));
        let only_gemini = read_antigravity(Ok(vec![lane("Gemini Models", true, 1.0, 100)]), 50);
        assert!(only_gemini[0].1.is_some() && only_gemini[1].1.is_none() && matches!(only_gemini[1].2, Some(Failure::Failed(_))));
        let failed = read_antigravity(Err("the command timed out".into()), 50);
        assert!(failed.iter().all(|(_, found, failure)| found.is_none() && matches!(failure, Some(Failure::Failed(why)) if why == "the command timed out")));
        assert_eq!(Service::named(" Antigravity-Claude "), Some(Service::AntigravityClaude));
        assert_eq!(Service::named("antigravity"), None);
    }

    fn win(used: f64, minutes: f64, reset: Option<i64>) -> Window {
        Window { used, minutes: Some(minutes), reset }
    }

    #[test]
    fn claudes_reply_gives_five_hours_and_the_week_from_limits_or_the_named_fields() {
        let now = 1_800_000_000;
        let at = |s: &str| chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp();
        let reply = json!({
            "limits": [
                { "kind": "weekly_scoped", "percent": 3.0, "resets_at": "2026-10-26T08:00:00Z" },
                { "kind": "session", "percent": 64.4, "resets_at": "2026-10-23T18:00:00.5+00:00" },
                { "kind": "weekly_all", "percent": 88.0, "resets_at": "2026-10-26T08:00:00Z" }
            ],
            "five_hour": { "utilization": 10.0, "resets_at": null }
        });
        let found = claude_found(&reply, now).unwrap();
        assert_eq!(found.main, win(64.4, 300.0, Some(at("2026-10-23T18:00:00Z"))));
        assert_eq!(found.outer, Some(win(88.0, 10080.0, Some(at("2026-10-26T08:00:00Z")))));
        assert_eq!((found.updated, found.from_log), (now, false));
        // A window that just rolled over left `limits`: its named field stands in; a window of
        // `limits` without a reset time is not one
        let rolled = json!({
            "limits": [{ "kind": "session", "percent": 50.0, "resets_at": null }],
            "five_hour": { "utilization": 0.0, "resets_at": null },
            "seven_day": { "utilization": 41.0, "resets_at": "2026-10-26T08:00:00Z" }
        });
        let found = claude_found(&rolled, now).unwrap();
        assert_eq!((found.main.clone(), found.outer.clone()), (win(0.0, 300.0, None), Some(win(41.0, 10080.0, Some(at("2026-10-26T08:00:00Z"))))));
        // The week alone is the percent; nothing usable is no reading, never 0 %
        assert_eq!(claude_found(&json!({ "seven_day": { "utilization": 7.0 } }), now).map(|f| (f.main.used, f.outer)), Some((7.0, None)));
        for nothing in [json!({}), json!({ "limits": "x", "five_hour": { "utilization": "high" } }), json!([1, 2])] {
            assert_eq!(claude_found(&nothing, now), None, "{nothing}");
        }
    }

    #[test]
    fn openais_reply_gives_the_primary_window_and_the_secondary_with_their_resets() {
        let now = 1_800_000_000;
        let reply = json!({
            "plan_type": "plus",
            "rate_limit": {
                "primary_window": { "used_percent": 25, "limit_window_seconds": 18000, "reset_at": 1_800_001_000 },
                "secondary_window": { "used_percent": 10, "limit_window_seconds": 604800, "reset_after_seconds": 3600 }
            },
            "additional_rate_limits": [{ "limit_name": "Spark", "rate_limit": { "primary_window": { "used_percent": 99 } } }]
        });
        let found = chatgpt_found(&reply, now).unwrap();
        assert_eq!(found.main, win(25.0, 300.0, Some(1_800_001_000)));
        assert_eq!(found.outer, Some(win(10.0, 10080.0, Some(now + 3600))));
        // A free plan's month is the percent all the same; garbage times are no reset
        let free = json!({ "rate_limit": { "primary_window": { "used_percent": 16, "limit_window_seconds": 2592000, "reset_after_seconds": 1e20 }, "secondary_window": null } });
        let found = chatgpt_found(&free, now).unwrap();
        assert_eq!((found.main.clone(), found.outer.clone()), (win(16.0, 43200.0, None), None));
        assert_eq!(chatgpt_found(&json!({ "rate_limit": { "primary_window": { "used_percent": null } } }), now), None);
        assert_eq!(chatgpt_found(&json!({ "detail": "Unauthorized" }), now), None);
    }

    #[test]
    fn codex_logs_give_the_last_line_of_their_own_bucket_as_old_as_it_is() {
        let at = |s: &str| chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp();
        let core = r#"{"timestamp":"2026-10-23T07:00:00Z","type":"event_msg","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":32.0,"window_minutes":300,"resets_at":1790585719},"secondary":{"used_percent":45,"window_minutes":10080,"resets_in_seconds":60},"plan_type":"plus"}}}"#;
        let spark = r#"{"timestamp":"2026-10-23T07:00:01Z","payload":{"rate_limits":{"limit_id":"codex_bengalfox","primary":{"used_percent":0,"window_minutes":300}}}}"#;
        let undated = r#"{"rate_limits":{"primary":{"used_percent":70}}}"#;
        let text = format!("{core}\nnot json, rate_limits\n{spark}\n{undated}\n{{\"payload\":{{\"type\":\"other\"}}}}");
        let found = rollout_found(&text).unwrap();
        assert_eq!(found.main, win(32.0, 300.0, Some(1790585719)));
        assert_eq!(found.outer, Some(win(45.0, 10080.0, Some(at("2026-10-23T07:01:00Z")))));
        assert_eq!((found.updated, found.from_log), (at("2026-10-23T07:00:00Z"), true));
        assert_eq!(rollout_found(&format!("{spark}\n{undated}")), None);
        assert_eq!(rollout_found(""), None);
    }

    #[test]
    fn the_newest_codex_log_is_looked_for_in_the_three_newest_days() {
        let root = std::env::temp_dir().join(format!("tapka-limits-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for day in ["2026/09/30", "2026/10/01", "2026/10/02", "2026/10/03"] {
            std::fs::create_dir_all(root.join(day)).unwrap();
        }
        let write = |path: &str, secs: u64| {
            let file = std::fs::File::create(root.join(path)).unwrap();
            file.set_modified(std::time::UNIX_EPOCH + Duration::from_secs(secs)).unwrap();
        };
        write("2026/09/30/rollout-old.jsonl", 1_900_000_000);
        write("2026/10/01/rollout-a.jsonl", 1_700_000_000);
        write("2026/10/02/rollout-b.jsonl", 1_700_000_300);
        write("2026/10/02/notes.jsonl", 1_800_000_000);
        write("2026/10/03/rollout-c.jsonl", 1_700_000_100);
        // The newest written of the three newest days; an older day is not looked at however new its file
        assert_eq!(newest_rollout(&root), Some(root.join("2026/10/02/rollout-b.jsonl")));
        assert_eq!(newest_rollout(&root.join("missing")), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_meter_shows_the_last_good_reading_with_why_it_is_not_new_and_never_an_invented_percent() {
        let now = chrono::Utc::now().timestamp();
        let found = Found { main: win(64.4, 300.0, Some(now + 3600)), outer: Some(win(88.0, 10080.0, Some(now + 3 * 86400))), updated: now, from_log: false };
        let mut known = Known { found: None, trouble: None };
        // Nothing ever read and nothing tried: no reading; tried and failed: no percent, only why,
        // which the label shows under "no data"
        assert_eq!(reading(Service::Claude, &known, true, true), None);
        assert_eq!(settle(&mut known, None, Some(&Failure::SignIn)), Some("no sign-in".into()));
        let why = reading(Service::Claude, &known, true, true).unwrap();
        assert_eq!(why, Reading { problem: Some("Нажмите, чтобы подключить".into()), sign_in: true, ..Reading::default() });
        assert!(!dated(why, now, true).stale);
        // Its program not installed: a tap can sign nothing in, and the problem says what is missing
        let missing = reading(Service::AntigravityClaude, &known, false, false).unwrap();
        assert_eq!((missing.problem.as_deref(), missing.sign_in), (Some("Antigravity is not installed"), false));
        settle(&mut known, None, Some(&Failure::Failed("the command timed out".into())));
        assert_eq!(reading(Service::AntigravityGemini, &known, true, false).unwrap().problem.as_deref(), Some("Could not read the usage"));
        settle(&mut known, None, Some(&Failure::Limited(0)));
        assert_eq!(reading(Service::Chatgpt, &known, true, true).unwrap().problem.as_deref(), Some("Сервер просит подождать"));
        assert_eq!(settle(&mut known, Some(found.clone()), None), Some("read".into()));
        // Read: a row per window, named by its length; no line repeats the percents
        let r = reading(Service::Claude, &known, true, true).unwrap();
        assert_eq!((r.value, r.outer, r.rows.clone(), r.hint.clone(), r.problem.clone()), (Some(64), Some(88), vec!["5 часов".to_string(), "Неделя".to_string()], vec![], None));
        assert_eq!(r.times, Times { updated: Some(now), fresh: Some(FRESH_MINUTES), reset: Some(now + 3600), reset2: Some(now + 3 * 86400) });
        assert_eq!(dated(r, now, true).when[0], "сброс через 1 ч");
        // The same read again is nothing for the log
        assert_eq!(settle(&mut known, Some(found.clone()), None), None);
        // A failure keeps the reading with its own time and says why, in the label's language
        assert_eq!(settle(&mut known, None, Some(&Failure::Expired)), Some("the token has expired, nothing sent; the last reading kept".into()));
        let r = reading(Service::Claude, &known, true, false).unwrap();
        assert_eq!((r.value, r.times.updated, r.rows.clone(), r.problem.as_deref()), (Some(64), Some(now), vec!["5 hours".to_string(), "Week".to_string()], Some("Tap to connect")));
        // ...and turns grey once old, as a file does
        let old = dated(r, now + 13 * 60, false);
        assert!(old.stale && old.ago.as_deref() == Some("updated 13 min ago"), "{:?}", old.ago);
        // An older reading from Codex's log does not replace a newer one; a newer one does, with its
        // default freshness, and says it comes from the log only once it is old
        let logged = |updated| Found { main: win(10.0, 300.0, None), outer: None, updated, from_log: true };
        settle(&mut known, Some(logged(now - 60)), Some(&Failure::Refused(401)));
        assert_eq!(reading(Service::Chatgpt, &known, true, true).unwrap().value, Some(64));
        settle(&mut known, Some(logged(now + 60)), Some(&Failure::Refused(401)));
        let r = reading(Service::Chatgpt, &known, true, true).unwrap();
        assert_eq!((r.value, r.outer, r.times.fresh, r.rows.clone(), r.problem.as_deref()), (Some(10), None, None, vec!["5 часов".to_string()], Some("Нажмите, чтобы подключить")));
        assert!(noted(dated(r.clone(), now, true)).hint.is_empty());
        assert_eq!(noted(dated(r, now + 10 * 60, true)).hint, ["по последнему запуску Codex"]);
        // Only meters of a limit are read, each by its place
        let meters = [Meter { name: "f".into(), source: Source::File("x.txt".into()), app: None }, Meter { name: "c".into(), source: Source::Limit(Service::Claude), app: None }];
        assert_eq!(read(&meters, false).iter().map(|(i, _)| *i).collect::<Vec<_>>(), [1]);
    }

    #[test]
    fn the_settings_window_gets_a_status_as_a_state_with_its_reason() {
        assert_eq!(json!(Status::Ok), json!({ "state": "ok" }));
        assert_eq!(json!(Status::SignIn), json!({ "state": "sign_in" }));
        assert_eq!(json!(Status::Missing), json!({ "state": "missing" }));
        assert_eq!(json!(Status::Unknown), json!({ "state": "unknown" }));
        assert_eq!(json!(Status::Checking), json!({ "state": "checking" }));
        assert_eq!(json!(Status::Error("Не удалось узнать лимит".into())), json!({ "state": "error", "reason": "Не удалось узнать лимит" }));
    }

    #[test]
    fn check_again_waits_for_the_thread_of_its_service_however_long_its_read_takes() {
        check_now(Service::AntigravityGemini);
        assert!(!asked_again(&[Service::Claude]), "another service is not asked");
        assert!(asked_again(&[Service::AntigravityGemini, Service::AntigravityClaude]));
        assert!(!asked_again(&[Service::AntigravityGemini, Service::AntigravityClaude]), "taken back once read");
    }

    #[test]
    fn a_sign_in_still_running_is_not_started_twice() {
        use std::os::windows::process::CommandExt;
        let mut slot = None;
        assert!(!running(&mut slot));
        let cmd = std::env::var_os("ComSpec").map(PathBuf::from).unwrap();
        slot = Some(std::process::Command::new(&cmd).args(["/c", "ping -n 10 127.0.0.1 >nul"]).creation_flags(0x0800_0000).spawn().unwrap());
        assert!(running(&mut slot));
        slot.as_mut().unwrap().kill().unwrap();
        slot.as_mut().unwrap().wait().unwrap();
        assert!(!running(&mut slot));
        assert!(slot.is_none(), "an ended sign-in is let go");
    }

    #[test]
    fn a_window_is_named_by_its_length() {
        let words = |m: f64, ru| length_words(Some(m), ru);
        assert_eq!([words(300.0, true), words(60.0, true), words(180.0, true), words(30.0, true)], ["5 часов", "1 час", "3 часа", "30 минут"]);
        assert_eq!([words(300.0, false), words(60.0, false), words(10080.0, false), words(43200.0, false)], ["5 hours", "1 hour", "Week", "Month"]);
        assert_eq!([words(10080.0, true), words(43200.0, true), words(2.0 * 1440.0, true), words(21.0 * 1440.0, true)], ["Неделя", "Месяц", "2 дня", "21 день"]);
        assert_eq!(length_words(None, true), "Лимит");
    }

    #[test]
    fn too_many_questions_wait_a_minute_doubling_to_a_quarter_hour_and_retry_after_only_raises_it() {
        assert_eq!([backoff_secs(0, 0), backoff_secs(1, 0), backoff_secs(3, 0), backoff_secs(9, 0)], [60, 120, 480, 900]);
        assert_eq!([backoff_secs(0, 3600), backoff_secs(1, 30)], [3600, 120]);
    }

    #[test]
    fn the_claude_token_is_renewed_shortly_before_it_expires_and_a_failed_renewal_waits_longer_each_time() {
        const EXP: u64 = 1_000_000_000;
        assert!(!should_renew(None, EXP, None, None, 0), "never on a guess");
        assert!(!should_renew(Some(EXP), EXP - RENEW_MARGIN_MS - 1, None, None, 0));
        assert!(should_renew(Some(EXP), EXP - RENEW_MARGIN_MS, None, None, 0));
        assert!(should_renew(Some(EXP), EXP + 3_600_000, None, None, 0), "an expired token is renewed too");
        let now = EXP + 1;
        // A new token waits out the cooldown; the same one waits twice as long after each failure, an hour at most
        assert!(!should_renew(Some(EXP + 5), now, Some(EXP), Some(now - 1000), 1));
        assert!(should_renew(Some(EXP + 5), now, Some(EXP), Some(now - RENEW_COOLDOWN_MS), 1));
        assert!(!should_renew(Some(EXP), now, Some(EXP), Some(now - RENEW_COOLDOWN_MS), 1));
        assert!(should_renew(Some(EXP), now, Some(EXP), Some(now - 2 * RENEW_COOLDOWN_MS), 1));
        assert!(!should_renew(Some(EXP), now, Some(EXP), Some(now - 2 * RENEW_COOLDOWN_MS), 2));
        assert!(!should_renew(Some(EXP), now, Some(EXP), Some(now - RENEW_RETRY_CAP_MS + 1), 30));
        assert!(should_renew(Some(EXP), now, Some(EXP), Some(now - RENEW_RETRY_CAP_MS), 30));
    }

    #[test]
    fn only_a_signed_in_credential_is_read_and_the_desktop_apps_claude_never_renews() {
        let dir = std::env::temp_dir().join(format!("tapka-limits-cred-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".credentials.json"), r#"{"claudeAiOauth":{"accessToken":"  ","expiresAt":0}}"#).unwrap();
        assert!(claude_credential(&dir).is_none(), "an empty token is signed out");
        std::fs::write(dir.join("credentials.json"), r#"{"accessToken":"t","expiresAt":1800000000000}"#).unwrap();
        assert_eq!(claude_credential(&dir).map(|c| (c.token, c.expires)), Some(("t".into(), Some(1_800_000_000_000))));
        std::fs::write(dir.join("auth.json"), r#"{"tokens":{"access_token":"a","account_id":" "}}"#).unwrap();
        assert_eq!(chatgpt_credential(&dir), None);
        std::fs::write(dir.join("auth.json"), r#"{"tokens":{"access_token":"a","account_id":"b"}}"#).unwrap();
        assert_eq!(chatgpt_credential(&dir), Some(("a".into(), "b".into())));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(desktop_owned(Path::new(r"C:\Users\example\AppData\Local\AnthropicClaude\app-1.2.3\claude.exe")));
        assert!(desktop_owned(Path::new(r"C:\Users\example\AppData\Roaming\Claude\claude-code\2.1.0\claude.exe")));
        assert!(!desktop_owned(Path::new(r"C:\Users\example\.local\bin\claude.exe")));
    }

    /// Asks both services with the sign-ins of this machine and prints the percents, never a token.
    #[test]
    #[ignore]
    fn live_limits_on_this_machine() {
        let agent = agent();
        let show = |r: Result<Found, Failure>| match r {
            Ok(f) => format!("{} % / {:?} %, resets {:?} / {:?}{}", rounded(f.main.used), f.outer.as_ref().map(|w| rounded(w.used)), f.main.reset, f.outer.as_ref().and_then(|w| w.reset), if f.from_log { ", from the log" } else { "" }),
            Err(e) => format!("failed: {}", e.words()),
        };
        let home = home().unwrap();
        eprintln!("claude: {}", show(read_claude(&agent, &home.join(".claude"))));
        eprintln!("chatgpt: {}", show(read_chatgpt_live(&agent, &home.join(".codex"))));
        let logged = newest_rollout(&home.join(".codex").join("sessions")).and_then(|p| tail(&p)).and_then(|t| rollout_found(&t));
        eprintln!("chatgpt log: {}", show(logged.ok_or(Failure::Failed("no log".into()))));
        for (service, found, failure) in read_antigravity(crate::agy::read_quota(), chrono::Utc::now().timestamp()) {
            eprintln!("{}: {}", service.name(), show(found.ok_or_else(|| failure.unwrap())));
        }
    }
}
