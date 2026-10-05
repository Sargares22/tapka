//! Start with Windows: a value under HKCU\...\Run (per user, no administrator needed), written
//! through the registry API. The uninstaller removes it (installer/hooks.nsh).

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ};

const RUN_KEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const NAME: PCWSTR = w!("Tapka");

fn has(key: PCWSTR, name: PCWSTR) -> bool {
    unsafe { RegGetValueW(HKEY_CURRENT_USER, key, name, RRF_RT_REG_SZ, None, None, None) == ERROR_SUCCESS }
}

fn write(key: PCWSTR, name: PCWSTR, text: &str) -> Result<(), String> {
    let text = HSTRING::from(text);
    // The size is in bytes and counts the closing zero
    let bytes = (text.len() as u32 + 1) * 2;
    let done = unsafe { RegSetKeyValueW(HKEY_CURRENT_USER, key, name, REG_SZ.0, Some(text.as_ptr() as *const _), bytes) };
    if done == ERROR_SUCCESS { Ok(()) } else { Err(format!("registry: error {}", done.0)) }
}

fn remove(key: PCWSTR, name: PCWSTR) -> Result<(), String> {
    if !has(key, name) {
        return Ok(());
    }
    let done = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key, name) };
    if done == ERROR_SUCCESS { Ok(()) } else { Err(format!("registry: error {}", done.0)) }
}

pub fn is_enabled() -> bool {
    has(RUN_KEY, NAME)
}

/// Registers the running exe to start at sign-in, or removes the registration.
pub fn set(enabled: bool) -> Result<(), String> {
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        write(RUN_KEY, NAME, &format!("\"{}\"", exe.display()))
    } else {
        remove(RUN_KEY, NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Registry::RegDeleteKeyW;

    #[test]
    fn a_value_is_written_found_and_removed() {
        // A key of the test's own, so the real list of programs that start with Windows is not touched
        let key = HSTRING::from(format!(r"Software\TapkaTest{}", std::process::id()));
        let key = PCWSTR(key.as_ptr());
        assert!(!has(key, NAME));
        write(key, NAME, r#""C:\Program Files\Tapka\tapka.exe""#).unwrap();
        assert!(has(key, NAME));
        remove(key, NAME).unwrap();
        assert!(!has(key, NAME));
        // Removing what is not there is not an error
        remove(key, NAME).unwrap();
        unsafe {
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, key);
        }
    }
}
