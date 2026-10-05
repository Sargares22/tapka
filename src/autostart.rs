//! Start with Windows: a value under HKCU\...\Run (per user, no administrator needed).
//! Done with reg.exe, so no extra dependency.

use std::os::windows::process::CommandExt;
use std::process::Command;

const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "Tapka";

fn reg(args: &[&str]) -> Result<(), String> {
    let out = Command::new("reg")
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("reg.exe: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn is_enabled() -> bool {
    reg(&["query", RUN_KEY, "/v", NAME]).is_ok()
}

/// Registers the running exe to start at sign-in, or removes the registration.
pub fn set(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        reg(&["add", RUN_KEY, "/v", NAME, "/t", "REG_SZ", "/d", &format!("\"{}\"", exe.display()), "/f"])
    } else if is_enabled() {
        reg(&["delete", RUN_KEY, "/v", NAME, "/f"])
    } else {
        Ok(())
    }
}
