//! The Windows calls behind the steps in `actions`.

use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, ShellExecuteW};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, LRESULT, POINT, RECT, WAIT_TIMEOUT, WPARAM};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Storage::Packaging::Appx::GetPackageFamilyName;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE,
};
use windows::Win32::Graphics::Gdi::{CombineRgn, CreateRectRgn, DeleteObject, SetWindowRgn, RGN_OR};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetCursorPos, GetForegroundWindow, InternalGetWindowText, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindowAsync,
    GWL_EXSTYLE, GW_OWNER, WM_NCACTIVATE, WM_NCPAINT, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_MINIMIZE, SW_RESTORE, WS_EX_TOOLWINDOW,
    SM_CONVERTIBLESLATEMODE, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
};

/// Windows speaks Russian to this user: its display language is Russian.
pub fn system_is_russian() -> bool {
    // The low ten bits are the language; 0x19 is Russian
    unsafe { GetUserDefaultUILanguage() & 0x3ff == 0x19 }
}

/// Opens an address, a program or a file the way Explorer would on a double click.
pub fn open(target: &str) -> Result<(), String> {
    let r = unsafe {
        ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(target), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL)
    };
    // ShellExecute reports success as a value above 32
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute code {}", r.0 as isize))
    }
}

fn key_input(vk: u16, up: bool) -> INPUT {
    // Win, the arrows, the navigation block and the sound and music keys are extended keys
    let extended = matches!(vk, 0x5B | 0x5C | 0x21..=0x28 | 0x2D | 0x2E | 0xAD..=0xB3);
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    if up {
        flags |= KEYEVENTF_KEYUP;
    }
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: scan, dwFlags: flags, ..Default::default() } },
    }
}

/// Presses the keys in order and releases them in reverse, as one uninterrupted batch. The
/// presses go to the system and the active window, like the same keys on a keyboard.
pub fn send_keys(vks: &[u16]) -> Result<(), String> {
    let inputs: Vec<INPUT> = vks
        .iter()
        .map(|&vk| key_input(vk, false))
        .chain(vks.iter().rev().map(|&vk| key_input(vk, true)))
        .collect();
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } as usize;
    if sent == inputs.len() {
        Ok(())
    } else {
        Err(format!("SendInput accepted {sent} of {} events", inputs.len()))
    }
}
/// Hides or shows a window without activating it. `hwnd` is the raw handle.
pub fn set_visible(hwnd: isize, visible: bool) {
    let cmd = if visible { SW_SHOWNOACTIVATE } else { SW_HIDE };
    unsafe {
        let _ = ShowWindowAsync(HWND(hwnd as _), cmd);
    }
}

/// Tablet mode: the keyboard is detached or folded back. Windows reports 0 in slate mode.
pub fn tablet_mode() -> bool {
    unsafe { GetSystemMetrics(SM_CONVERTIBLESLATEMODE) == 0 }
}
pub fn is_visible(hwnd: isize) -> bool {
    unsafe { IsWindowVisible(HWND(hwnd as _)).as_bool() }
}

/// Where the mouse cursor is inside a visible window, in physical px from its top-left corner;
/// `None` when the cursor is elsewhere or the window is hidden.
pub fn cursor_in_window(hwnd: isize) -> Option<(i32, i32)> {
    let hwnd = HWND(hwnd as _);
    let (mut p, mut r) = (POINT::default(), RECT::default());
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || GetCursorPos(&mut p).is_err() || GetWindowRect(hwnd, &mut r).is_err() {
            return None;
        }
    }
    let inside = p.x >= r.left && p.x < r.right && p.y >= r.top && p.y < r.bottom;
    inside.then_some((p.x - r.left, p.y - r.top))
}

/// Limits what of a window is drawn and takes presses to the given rectangles inside it, each
/// `(x, y, width, height)` in physical px from its top-left corner. Presses outside them go to
/// whatever is behind the window. The region is always explicit: handing the whole window back
/// with a null region makes Windows repaint the newly uncovered part, which shows as a flash.
pub fn set_region(hwnd: isize, rects: &[(i32, i32, i32, i32)]) {
    unsafe {
        let region = CreateRectRgn(0, 0, 0, 0);
        for &(x, y, w, h) in rects {
            let part = CreateRectRgn(x, y, x + w, y + h);
            CombineRgn(Some(region), Some(region), Some(part), RGN_OR);
            let _ = DeleteObject(part.into());
        }
        // The system owns the region after a call that worked; one it refused is still ours
        if SetWindowRgn(HWND(hwnd as _), Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
        }
    }
}

/// The file name of the program whose window is active, lower case.
pub fn foreground_program() -> Option<String> {
    program_of(unsafe { GetForegroundWindow() }).map(|(exe, _)| exe)
}
/// A window a person would call a program's window: visible, titled, with no owner, not a tool
/// window and not one Windows keeps cloaked.
pub struct AppWindow {
    pub hwnd: isize,
    /// The program's file name, lower case.
    pub exe: String,
    /// The Store package family the program belongs to, if it is a packaged app.
    pub family: Option<String>,
}

fn program_of(hwnd: HWND) -> Option<(String, Option<String>)> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    program_of_process(pid)
}

/// The file name of a process's program, lower case, and its Store package family if it has one.
fn program_of_process(pid: u32) -> Option<(String, Option<String>)> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let program = program_behind(process);
        let _ = CloseHandle(process);
        program
    }
}

/// The same for a process that is already open.
fn program_behind(process: HANDLE) -> Option<(String, Option<String>)> {
    unsafe {
        let mut path = [0u16; 1024];
        let mut len = path.len() as u32;
        let named = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(path.as_mut_ptr()), &mut len).is_ok();
        let mut family = [0u16; 256];
        let mut family_len = family.len() as u32;
        // 0 is ERROR_SUCCESS; an unpackaged program answers APPMODEL_ERROR_NO_PACKAGE
        let packaged = GetPackageFamilyName(process, &mut family_len, Some(PWSTR(family.as_mut_ptr()))).0 == 0;
        if !named {
            return None;
        }
        let path = String::from_utf16_lossy(&path[..len as usize]);
        let exe = path.rsplit(['\\', '/']).next()?.to_lowercase();
        let family = packaged
            .then(|| String::from_utf16_lossy(&family[..family_len.saturating_sub(1) as usize]))
            .filter(|f| !f.is_empty());
        Some((exe, family))
    }
}

/// What the last look over the desktop learned: which program each window belongs to. The
/// desktop is looked over twice a second, and asking Windows about a process is the costly part
/// of it, so a window that is still there with the same process is not asked about again.
/// Windows may hand the number of a closed process to a new one, so the process is kept open
/// while it is remembered: an open process keeps its number, and one that has ended says so.
struct Known {
    hwnd: isize,
    pid: u32,
    /// The open process, as a number so the note can cross threads.
    process: isize,
    program: (String, Option<String>),
}

impl Known {
    fn alive(&self) -> bool {
        unsafe { WaitForSingleObject(HANDLE(self.process as _), 0) == WAIT_TIMEOUT }
    }
}

impl Drop for Known {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(HANDLE(self.process as _));
        }
    }
}

/// Held for a whole look, so two looks at once do not undo each other's notes.
static KNOWN: std::sync::Mutex<Vec<Known>> = std::sync::Mutex::new(Vec::new());

/// Every program window on the desktop, front to back.
pub fn app_windows() -> Vec<AppWindow> {
    struct Look {
        before: Vec<Known>,
        now: Vec<Known>,
        found: Vec<AppWindow>,
    }
    unsafe extern "system" fn each(hwnd: HWND, lp: LPARAM) -> BOOL {
        let look = &mut *(lp.0 as *mut Look);
        // The panel's own windows are skipped before anything is asked of them: asking a window
        // for its title from another thread waits for the thread that owns it, and that is the
        // panel's main thread, which may itself be waiting for whoever called this.
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return BOOL(1);
        }
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        let plain = cloaked == 0
            // The title is read from the system's own copy: asking the window itself would wait
            // for its program to answer, twice a second, for every window
            && InternalGetWindowText(hwnd, &mut [0u16; 2]) > 0
            && GetWindow(hwnd, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true)
            && GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 == 0;
        if !plain {
            return BOOL(1);
        }
        let key = hwnd.0 as isize;
        if let Some(at) = look.before.iter().position(|k| k.hwnd == key && k.pid == pid && k.alive()) {
            let known = look.before.swap_remove(at);
            look.found.push(AppWindow { hwnd: key, exe: known.program.0.clone(), family: known.program.1.clone() });
            look.now.push(known);
            return BOOL(1);
        }
        // A process that lets itself be watched is remembered; one that does not is asked about
        // at every look
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE, false, pid) {
            Ok(process) => match program_behind(process) {
                Some(program) => {
                    look.found.push(AppWindow { hwnd: key, exe: program.0.clone(), family: program.1.clone() });
                    look.now.push(Known { hwnd: key, pid, process: process.0 as isize, program });
                }
                None => {
                    let _ = CloseHandle(process);
                }
            },
            Err(_) => {
                if let Some((exe, family)) = program_of_process(pid) {
                    look.found.push(AppWindow { hwnd: key, exe, family });
                }
            }
        }
        BOOL(1)
    }
    let mut known = KNOWN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut look = Look { before: std::mem::take(&mut *known), now: Vec::new(), found: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut look as *mut Look as isize));
    }
    // Only what this look saw is kept: windows that are gone are forgotten
    *known = look.now;
    look.found
}

/// The active window.
pub fn foreground() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Brings a window to the front, restoring it first if it is minimized. Windows lets a program
/// that is not in front do this only right after it has sent input, so a lone Alt is tapped
/// first: the usual way around the foreground lock.
pub fn bring_to_front(hwnd: isize) -> Result<(), String> {
    let window = HWND(hwnd as _);
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindowAsync(window, SW_RESTORE);
        }
        let alt = [key_input(0x12, false), key_input(0x12, true)];
        SendInput(&alt, std::mem::size_of::<INPUT>() as i32);
        if SetForegroundWindow(window).as_bool() {
            Ok(())
        } else {
            Err("Windows refused to bring the window to the front".into())
        }
    }
}

pub fn minimize(hwnd: isize) {
    unsafe {
        let _ = ShowWindowAsync(HWND(hwnd as _), SW_MINIMIZE);
    }
}

/// Stops Windows from drawing a title bar over the capsule. A window without decorations still
/// has one in its style, and for a window with a region Windows draws it itself, in the old
/// plain look, whenever another window of the program becomes active (seen 2026-10-05: a white
/// title bar with buttons beside the capsule's handle while the settings window was open). The
/// style has to stay as it is: changing it makes the window lose its region. Call on the thread
/// that owns the window.
pub fn quiet_frame(hwnd: isize) {
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
        match msg {
            WM_NCPAINT => LRESULT(0),
            // -1 tells Windows to change the active state without repainting the frame
            WM_NCACTIVATE => DefSubclassProc(hwnd, msg, wparam, LPARAM(-1)),
            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }
    unsafe {
        let _ = SetWindowSubclass(HWND(hwnd as _), Some(proc), 1, 0);
    }
}

/// A window's rectangle on the screen, `(x, y, width, height)` in physical px.
pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(HWND(hwnd as _), &mut r).ok()? };
    Some((r.left, r.top, r.right - r.left, r.bottom - r.top))
}

/// Moves a window without resizing, raising or activating it. Called straight from the thread
/// that follows the pointer, so a drag does not queue behind the main thread.
pub fn move_window(hwnd: isize, x: i32, y: i32) {
    unsafe {
        let _ = SetWindowPos(HWND(hwnd as _), None, x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

/// For each wanted `(program file name in lower case, window title)`: is such a window visible
/// right now? Any visible window counts, overlays and tool windows included. The title is read
/// from the system's own copy, which asks nothing of the window.
pub fn shown(wanted: &[(String, String)]) -> Vec<bool> {
    struct Scan<'a> {
        wanted: &'a [(String, String)],
        found: Vec<bool>,
    }
    unsafe extern "system" fn each(hwnd: HWND, lp: LPARAM) -> BOOL {
        let scan = &mut *(lp.0 as *mut Scan);
        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut buf = [0u16; 256];
        let len = InternalGetWindowText(hwnd, &mut buf);
        if len <= 0 {
            return BOOL(1);
        }
        let title = String::from_utf16_lossy(&buf[..len as usize]);
        if !scan.wanted.iter().any(|(_, t)| *t == title) {
            return BOOL(1);
        }
        if let Some((exe, _)) = program_of(hwnd) {
            for (i, (program, t)) in scan.wanted.iter().enumerate() {
                if *program == exe && *t == title {
                    scan.found[i] = true;
                }
            }
        }
        BOOL(1)
    }
    let mut scan = Scan { wanted, found: vec![false; wanted.len()] };
    if !wanted.is_empty() {
        unsafe {
            let _ = EnumWindows(Some(each), LPARAM(&mut scan as *mut Scan as isize));
        }
    }
    scan.found
}
#[cfg(test)]
mod live {
    use super::*;

    /// Every visible window with a title: `program|title`.
    fn visible() -> Vec<String> {
        unsafe extern "system" fn each(hwnd: HWND, lp: LPARAM) -> BOOL {
            let found = &mut *(lp.0 as *mut Vec<String>);
            let mut buf = [0u16; 256];
            let len = InternalGetWindowText(hwnd, &mut buf);
            if IsWindowVisible(hwnd).as_bool() && len > 0 {
                let exe = program_of(hwnd).map(|(exe, _)| exe).unwrap_or_default();
                // Windows keeps its input panels open and cloaks them while they are away
                let mut cloaked = 0u32;
                let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
                let hidden = if cloaked == 0 { "" } else { "|cloaked" };
                found.push(format!("{exe}|{}{hidden}", String::from_utf16_lossy(&buf[..len as usize])));
            }
            BOOL(1)
        }
        let mut found: Vec<String> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut Vec<String> as isize));
        }
        found
    }

    fn press(keys: &str) {
        send_keys(&crate::keys::parse(keys).unwrap()).unwrap();
    }

    /// Presses real keys on the desktop and prints what appeared; run by hand:
    /// `cargo test live -- --ignored --nocapture --test-threads=1`
    #[test]
    #[ignore = "manual: presses real keys on the desktop"]
    fn ready_actions() {
        let wait = |ms| std::thread::sleep(std::time::Duration::from_millis(ms));
        for (keys, close) in [("win+shift+s", "esc"), ("win+h", "esc"), ("win+tab", "esc"), ("win+ctrl+o", "win+ctrl+o"), ("win+d", "win+d"), ("win+e", "alt+f4"), ("win+v", "esc"), ("win+.", "esc")] {
            let before = visible();
            press(keys);
            wait(2500);
            let after = visible();
            let new: Vec<_> = after.iter().filter(|w| !before.contains(w)).collect();
            let gone = before.iter().filter(|w| !after.contains(w)).count();
            println!("{keys}: front {:?}, appeared {new:?}, gone {gone}", foreground_program());
            press(close);
            wait(1500);
        }
        // Sound and music: pressed twice, so the volume and the playback end where they began
        for keys in ["volumeup", "volumedown", "volumemute", "playpause", "nexttrack"] {
            press(keys);
            wait(1500);
            press(keys);
            println!("{keys}: sent twice");
            wait(1500);
        }
    }
}
