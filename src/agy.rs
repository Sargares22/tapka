//! Antigravity's usage limits, from its own command: `agy --print /usage`, run hidden in a pseudo
//! console, as the AI limit meters of Antigravity ask for them (`limits`).
//! Behaviour ported from Codenotch, MIT, Copyright (c) 2026 Vinz.

use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Its output is kept this far; more is no report.
const OUTPUT_CAP: usize = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(70);

/// One window of Antigravity's quota: the group of models it is for, as the command names it
/// (`Gemini Models`), whether it is the five hours or the week, the percent used and when it resets,
/// seconds since 1970.
#[derive(Clone, Debug, PartialEq)]
pub struct Lane {
    pub group: String,
    pub five_hour: bool,
    pub used: f64,
    pub reset: i64,
}

/// Antigravity's command: where its installer puts it, then the PATH; only an exe.
pub fn find_agy() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("agy").join("bin"));
    let path: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).filter(|d| d.is_absolute()).collect()).unwrap_or_default();
    local.into_iter().chain(path).map(|d| d.join("agy.exe")).find(|p| p.is_file())
}

/// The text a terminal shows without its escape sequences (CSI, OSC, two-character ones), each line
/// ended by `\n` alone.
fn sanitize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    // Parameters and intermediates, up to a final byte in 0x40..=0x7e
                    while let Some(next) = chars.next() {
                        if ('\x40'..='\x7e').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // Up to BEL or ST
                    while let Some(next) = chars.next() {
                        if next == '\x07' || (next == '\x1b' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                Some(&next) if ('\x40'..='\x5f').contains(&next) => {
                    chars.next();
                }
                _ => {}
            },
            '\r' if chars.peek() == Some(&'\n') => {}
            '\r' => out.push('\n'),
            c => out.push(c),
        }
    }
    out
}

/// The quota the command reports: after a line `Quota:`, rows such as `Gemini Models Five Hour
/// Limit Remaining 78% 2026-09-10T06:12:31Z`, the percent left. A row of another window is left
/// out; a row that cannot be read, or a report without a row of five hours or a week, is an error,
/// never a 0 %.
fn parse_quota(text: &str) -> Result<Vec<Lane>, String> {
    let clean = sanitize(text);
    if !clean.lines().any(|line| line.trim() == "Quota:") {
        return Err("the command gave no quota report".into());
    }
    let mut lanes = Vec::new();
    for line in clean.lines().filter(|line| line.contains("Limit Remaining")) {
        let (left, reset) = line.rsplit_once('%').ok_or("a quota row without a percent")?;
        let (label, remaining) = left.trim().rsplit_once(char::is_whitespace).ok_or("a quota row without a percent")?;
        let remaining: f64 = remaining.parse().map_err(|_| "a quota percent that is no number")?;
        if !(0.0..=100.0).contains(&remaining) {
            return Err("a quota percent out of range".into());
        }
        let reset = reset_time(reset).ok_or("a quota reset that is no time")?;
        let label = label.split_whitespace().collect::<Vec<_>>().join(" ");
        let label = label.strip_suffix(" Remaining").ok_or("an unknown quota row")?;
        let lane = |suffix: &str, five_hour| label.strip_suffix(suffix).map(|group| Lane { group: group.to_string(), five_hour, used: 100.0 - remaining, reset });
        lanes.extend(lane(" Five Hour Limit", true).or_else(|| lane(" Weekly Limit", false)));
    }
    if lanes.is_empty() {
        return Err("the command gave no quota of five hours or a week".into());
    }
    Ok(lanes)
}

/// A reset as the command writes it, in seconds since 1970: RFC 3339, as older versions did, or
/// `2026-10-23 22:28 +07`, as version 1.4.6 does.
fn reset_time(text: &str) -> Option<i64> {
    let text = text.trim();
    let at = chrono::DateTime::parse_from_rfc3339(text).or_else(|_| chrono::DateTime::parse_from_str(text, "%Y-%m-%d %H:%M %#z")).or_else(|_| chrono::DateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S %#z"));
    at.ok().map(|at| at.timestamp())
}

/// Asks Antigravity for its quota. Its own sign-in does the asking; the panel reads none.
pub fn read_quota() -> Result<Vec<Lane>, String> {
    let agy = find_agy().ok_or("the Antigravity command is not installed")?;
    // A folder of its own, so the command finds no project to look at
    let dir = std::env::temp_dir().join("tapka-agy");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot make a folder for the command ({e})"))?;
    parse_quota(&run(&agy, &["--sandbox", "--print-timeout", "30s", "--print", "/usage"], Some(&dir), TIMEOUT)?)
}

/// An argument as CommandLineToArgvW reads it back.
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '\n', '\x0b', '"']) {
        return arg.to_string();
    }
    let mut quoted = String::from('"');
    let mut backslashes = 0;
    for c in arg.chars() {
        if c == '\\' {
            backslashes += 1;
            continue;
        }
        quoted.extend(std::iter::repeat_n('\\', if c == '"' { backslashes * 2 + 1 } else { backslashes }));
        backslashes = 0;
        quoted.push(c);
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

/// A Windows job that ends every process in it when it closes, the panel's exit included: what a
/// command starts cannot outlive the panel.
pub struct Job(windows::Win32::Foundation::HANDLE);

impl Job {
    pub fn new() -> Result<Job, String> {
        use windows::Win32::System::JobObjects::{CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE};
        unsafe {
            let job = Job(CreateJobObjectW(None, windows::core::PCWSTR::null()).map_err(|e| format!("CreateJobObjectW: {e}"))?);
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(job.0, JobObjectExtendedLimitInformation, &limits as *const _ as *const _, std::mem::size_of_val(&limits) as u32)
                .map_err(|e| format!("SetInformationJobObject: {e}"))?;
            Ok(job)
        }
    }

    /// Puts a process in the job, by its handle; what it starts from then on is in the job too.
    pub fn assign(&self, process: windows::Win32::Foundation::HANDLE) -> Result<(), String> {
        unsafe { windows::Win32::System::JobObjects::AssignProcessToJobObject(self.0, process) }.map_err(|e| format!("AssignProcessToJobObject: {e}"))
    }

    pub fn terminate(&self) {
        unsafe {
            let _ = windows::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

/// Runs a program hidden in a pseudo console: through plain pipes Antigravity's command writes
/// nothing. The program and all it starts live in a job that ends with them, on the timeout too.
/// Its output is read while it runs, so it never blocks on a full pipe, and kept up to `OUTPUT_CAP`.
fn run(program: &Path, args: &[&str], cwd: Option<&Path>, timeout: Duration) -> Result<String, String> {
    use std::io::Read;
    use std::os::windows::io::FromRawHandle;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Console::{ClosePseudoConsole, CreatePseudoConsole, COORD, HPCON};
    use windows::Win32::System::Pipes::CreatePipe;
    use windows::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess, InitializeProcThreadAttributeList, ResumeThread, TerminateProcess, UpdateProcThreadAttribute,
        WaitForSingleObject, CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
        STARTF_USESTDHANDLES, STARTUPINFOEXW,
    };

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    struct Console(HPCON);
    impl Drop for Console {
        fn drop(&mut self) {
            unsafe { ClosePseudoConsole(self.0) }
        }
    }
    struct Attributes(LPPROC_THREAD_ATTRIBUTE_LIST);
    impl Drop for Attributes {
        fn drop(&mut self) {
            unsafe { DeleteProcThreadAttributeList(self.0) }
        }
    }

    if !program.is_file() {
        return Err(format!("{} is not there", program.display()));
    }
    let job = Job::new()?;

    let (mut in_read, mut in_write, mut out_read, mut out_write) = (HANDLE::default(), HANDLE::default(), HANDLE::default(), HANDLE::default());
    unsafe {
        CreatePipe(&mut in_read, &mut in_write, None, 0).map_err(|e| format!("CreatePipe: {e}"))?;
        if let Err(e) = CreatePipe(&mut out_read, &mut out_write, None, 0) {
            let _ = CloseHandle(in_read);
            let _ = CloseHandle(in_write);
            return Err(format!("CreatePipe: {e}"));
        }
    }
    let console = unsafe { CreatePseudoConsole(COORD { X: 160, Y: 60 }, in_read, out_write, 0) };
    // The console holds its own ends now
    unsafe {
        let _ = CloseHandle(in_read);
        let _ = CloseHandle(out_write);
    }
    let _input = Handle(in_write);
    let console = match console {
        Ok(console) => Console(console),
        Err(e) => {
            unsafe {
                let _ = CloseHandle(out_read);
            }
            return Err(format!("CreatePseudoConsole: {e}"));
        }
    };
    let mut output = unsafe { std::fs::File::from_raw_handle(out_read.0 as _) };
    let reader = std::thread::spawn(move || {
        let (mut kept, mut chunk) = (Vec::new(), [0u8; 4096]);
        while let Ok(n) = output.read(&mut chunk) {
            if n == 0 {
                break;
            }
            // One byte over the cap tells a full report from a cut one
            let keep = n.min((OUTPUT_CAP + 1).saturating_sub(kept.len()));
            kept.extend_from_slice(&chunk[..keep]);
        }
        kept
    });

    let mut size = 0usize;
    let _ = unsafe { InitializeProcThreadAttributeList(None, 1, None, &mut size) };
    let mut storage = vec![0u8; size];
    let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr() as *mut _);
    let _attributes = unsafe {
        InitializeProcThreadAttributeList(Some(list), 1, None, &mut size).map_err(|e| format!("InitializeProcThreadAttributeList: {e}"))?;
        Attributes(list)
    };
    unsafe {
        UpdateProcThreadAttribute(list, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize, Some(console.0 .0 as *const core::ffi::c_void), std::mem::size_of::<HPCON>(), None, None)
            .map_err(|e| format!("UpdateProcThreadAttribute: {e}"))?;
    }

    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let application = wide(program.as_os_str());
    let line = std::iter::once(quote_arg(&program.to_string_lossy())).chain(args.iter().map(|a| quote_arg(a))).collect::<Vec<_>>().join(" ");
    let mut line: Vec<u16> = line.encode_utf16().chain(std::iter::once(0)).collect();
    let cwd = cwd.map(|p| wide(p.as_os_str()));
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    // Without it, output handles the panel inherited would pass the pseudo console by
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.lpAttributeList = list;
    let mut process = PROCESS_INFORMATION::default();
    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            Some(PWSTR(line.as_mut_ptr())),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED,
            None,
            cwd.as_ref().map_or(PCWSTR::null(), |c| PCWSTR(c.as_ptr())),
            &startup.StartupInfo,
            &mut process,
        )
        .map_err(|e| format!("CreateProcessW: {e}"))?;
    }
    // Started suspended and put in the job first, so nothing it starts can slip out of the job
    let thread = Handle(process.hThread);
    let process = Handle(process.hProcess);
    if job.assign(process.0).is_err() {
        unsafe {
            let _ = TerminateProcess(process.0, 1);
        }
        return Err("cannot put the command in a job".into());
    }
    if unsafe { ResumeThread(thread.0) } == u32::MAX {
        return Err("cannot start the command".into());
    }
    drop(thread);

    let started = std::time::Instant::now();
    let (mut code, mut timed_out) = (0u32, false);
    loop {
        if unsafe { WaitForSingleObject(process.0, 100) } == WAIT_OBJECT_0 {
            let _ = unsafe { GetExitCodeProcess(process.0, &mut code) };
            break;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            job.terminate();
            break;
        }
    }
    // What it started ends before its console closes, and the console's close ends the reading
    drop(job);
    drop(console);
    let kept = reader.join().map_err(|_| "the reader of the command's output failed".to_string())?;
    if timed_out {
        return Err("the command timed out".into());
    }
    if code != 0 {
        return Err(format!("the command failed with exit code {code}"));
    }
    if kept.len() > OUTPUT_CAP {
        return Err("the command's output is too large".into());
    }
    Ok(sanitize(&String::from_utf8_lossy(&kept)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Quota:\nGemini Models Weekly Limit Remaining 94% 2026-09-12T01:47:23Z\nGemini Models Five Hour Limit Remaining 78% 2026-09-10T06:12:31Z\nClaude and GPT models Weekly Limit Remaining 100% 2026-09-17T01:47:23Z\nClaude and GPT models Five Hour Limit Remaining 100% 2026-09-10T08:13:44Z";

    #[test]
    fn the_quota_report_gives_each_group_its_five_hours_and_week_and_garbage_gives_an_error() {
        let at = |s: &str| chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp();
        let lane = |group: &str, five_hour, used, reset| Lane { group: group.into(), five_hour, used, reset: at(reset) };
        assert_eq!(
            parse_quota(SAMPLE).unwrap(),
            [
                lane("Gemini Models", false, 6.0, "2026-09-12T01:47:23Z"),
                lane("Gemini Models", true, 22.0, "2026-09-10T06:12:31Z"),
                lane("Claude and GPT models", false, 0.0, "2026-09-17T01:47:23Z"),
                lane("Claude and GPT models", true, 0.0, "2026-09-10T08:13:44Z"),
            ]
        );
        // Version 1.4.6 lines its columns up and writes the reset with a short offset
        let newer = "Quota:\r\nGemini Models          Five Hour Limit Remaining  99%   2026-10-23 22:28 +07\r\nClaude and GPT models  Weekly Limit Remaining     100%  2026-10-30 17:53 +07\r\n";
        assert_eq!(
            parse_quota(newer).unwrap(),
            [Lane { group: "Gemini Models".into(), five_hour: true, used: 1.0, reset: at("2026-10-23T15:28:00Z") }, Lane { group: "Claude and GPT models".into(), five_hour: false, used: 0.0, reset: at("2026-10-30T10:53:00Z") }]
        );
        for bad in ["", "authentication required", "Quota:\nunknown", "Quota:\nDaily Limit Remaining 50% 2026-09-12T01:47:23Z"]
            .into_iter()
            .map(String::from)
            .chain(["101%", "-5%", "NaN%"].map(|p| SAMPLE.replace("94%", p)))
            .chain([SAMPLE.replace("2026-09-12T01:47:23Z", "not-a-date")])
        {
            assert!(parse_quota(&bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_terminals_escapes_are_taken_out_and_its_lines_end_plainly() {
        let raw = "\x1b[?25h\x1b[32mQuota:\x1b[0m\r\n\x1b]0;agy\x07Gemini Models Weekly Limit Remaining 94% 2026-09-12T01:47:23Z\r\nend\r";
        assert_eq!(sanitize(raw), "Quota:\nGemini Models Weekly Limit Remaining 94% 2026-09-12T01:47:23Z\nend\n");
        assert_eq!(parse_quota(raw).unwrap().len(), 1);
    }

    #[test]
    fn arguments_are_quoted_as_windows_reads_them_back() {
        assert_eq!(quote_arg(r"C:\Program Files\agy.exe"), r#""C:\Program Files\agy.exe""#);
        assert_eq!(quote_arg("path with space\\"), "\"path with space\\\\\"");
        assert_eq!(quote_arg("a\\\"b"), "\"a\\\\\\\"b\"");
        assert_eq!(quote_arg(""), "\"\"");
        assert_eq!(quote_arg("/usage"), "/usage");
    }

    fn cmd() -> PathBuf {
        std::env::var_os("ComSpec").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("SystemRoot").unwrap()).join("System32").join("cmd.exe"))
    }

    #[test]
    fn a_process_in_a_job_ends_with_what_it_started_when_the_job_closes() {
        use std::os::windows::io::AsRawHandle;
        use std::os::windows::process::CommandExt;
        let job = Job::new().unwrap();
        let mut child = std::process::Command::new(cmd()).args(["/c", "ping -n 10 127.0.0.1 >nul"]).creation_flags(0x0800_0000).spawn().unwrap();
        job.assign(windows::Win32::Foundation::HANDLE(child.as_raw_handle())).unwrap();
        let started = std::time::Instant::now();
        drop(job);
        while child.try_wait().unwrap().is_none() {
            assert!(started.elapsed() < Duration::from_secs(3), "the process outlived its job");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn the_runner_reads_a_program_in_a_pseudo_console_bounded_and_ends_it_on_the_timeout() {
        assert!(run(Path::new(r"C:\non\existent\agy.exe"), &["--print", "/usage"], None, Duration::from_secs(5)).is_err());
        let out = run(&cmd(), &["/c", "echo hello from conpty"], None, Duration::from_secs(10)).unwrap();
        assert!(out.contains("hello from conpty"), "{out}");
        // A lot of output neither blocks the program nor grows past the cap
        let large = run(&cmd(), &["/c", "for /L %i in (1,1,2000) do @echo 012345678901234567890123456789012345678901234567890123456789"], None, Duration::from_secs(20));
        assert!(matches!(&large, Ok(out) if out.len() <= OUTPUT_CAP) || large == Err("the command's output is too large".into()), "{large:?}");
        // Past the timeout the program and what it started end at once
        let started = std::time::Instant::now();
        let slow = run(&cmd(), &["/c", "ping -n 10 127.0.0.1 >nul"], None, Duration::from_millis(400));
        assert_eq!(slow, Err("the command timed out".into()));
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
