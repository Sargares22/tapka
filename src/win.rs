//! The Windows calls behind the steps in `actions`.

use windows::core::{BOOL, GUID, HSTRING, PCWSTR, PWSTR};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SHGetKnownFolderPath, SetWindowSubclass, ShellExecuteW, KF_FLAG_DEFAULT};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, LRESULT, POINT, PROPERTYKEY, RECT, WAIT_TIMEOUT, WPARAM};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Storage::Packaging::Appx::{GetApplicationUserModelId, GetPackageFamilyName};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, GetCurrentThreadId, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, WaitForSingleObject, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
};
use windows::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, DeleteObject, GetMonitorInfoW, GetWindowRgn, GDI_REGION_TYPE, MonitorFromWindow, SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST, RGN_OR,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumChildWindows, EnumWindows, GetClassNameW, GetCursorPos, GetForegroundWindow, InternalGetWindowText, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowRect,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindowAsync,
    GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, WS_EX_APPWINDOW, GW_OWNER, WM_NCACTIVATE, WM_NCPAINT, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SW_MINIMIZE, SW_RESTORE, WS_EX_LAYERED, WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    NID_EXTERNAL_PEN, NID_INTEGRATED_PEN, SM_CONVERTIBLESLATEMODE, SM_DIGITIZER, SW_HIDE, SW_SHOWNOACTIVATE, SW_SHOWNORMAL,
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
    all_taken(sent, inputs.len())
}
/// Presses one key and leaves it down, or lets it go: what a `hold` item does, the same way as
/// `send_keys`.
pub fn send_key(vk: u16, up: bool) -> Result<(), String> {
    let sent = unsafe { SendInput(&[key_input(vk, up)], std::mem::size_of::<INPUT>() as i32) } as usize;
    all_taken(sent, 1)
}

/// SendInput says how many of the events it took. Anything short of all of them is a failure:
/// none when another program blocks input, a part when the batch was cut short.
fn all_taken(sent: usize, events: usize) -> Result<(), String> {
    if sent == events {
        Ok(())
    } else {
        Err(format!("SendInput accepted {sent} of {events} events"))
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

/// The machine takes a pen: its screen has one built in or one is attached.
pub fn pen_present() -> bool {
    let digitizer = unsafe { GetSystemMetrics(SM_DIGITIZER) } as u32;
    digitizer & (NID_INTEGRATED_PEN | NID_EXTERNAL_PEN) != 0
}
pub fn is_visible(hwnd: isize) -> bool {
    unsafe { IsWindowVisible(HWND(hwnd as _)).as_bool() }
}

/// Puts a window that stays above the others back in front of them, without making it active.
/// Windows keeps all such windows in one order, and the one raised last is in front.
pub fn raise(hwnd: isize) {
    unsafe {
        let _ = SetWindowPos(HWND(hwnd as _), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// The active window belongs to another program and covers its whole screen: a game or a video
/// in full screen. A maximized window leaves the taskbar and is not counted; the desktop is not.
pub fn front_is_full_screen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() || owner_of(hwnd) == GetCurrentProcessId() || matches!(class_of(hwnd).as_str(), "Progman" | "WorkerW") {
            return false;
        }
        let Some(((x, y, w, h), _)) = frame_of(hwnd) else { return false };
        let mut screen = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut screen).as_bool() {
            return false;
        }
        let m = screen.rcMonitor;
        x <= m.left && y <= m.top && x + w >= m.right && y + h >= m.bottom
    }
}

/// The active window is one of the panel's own.
pub fn front_is_mine() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        !hwnd.is_invalid() && owner_of(hwnd) == GetCurrentProcessId()
    }
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
/// The program of the active window: its file name, lower case, and its Store package family.
/// `None` when nothing is active, or when it is the desktop, the taskbar or a window of the panel
/// itself: no program's own items belong there.
pub fn front_program() -> Option<(String, Option<String>)> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == unsafe { GetCurrentProcessId() } {
        return None;
    }
    if matches!(class_of(hwnd).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        return None;
    }
    program_of_process(shown_by(hwnd, pid)?)
}

/// Whether a program runs with raised rights, "as administrator".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rights {
    Raised,
    Plain,
    /// Windows would not say.
    Unknown,
}

/// The rights of the program whose window is active. Windows may drop keys sent to a window of a
/// program with rights above the sender's, and says nothing; this is how the panel can tell the
/// person why. A call that fails gives `Unknown`, never `Raised`.
pub fn front_rights() -> Rights {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Rights::Unknown;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    rights_of_process(pid)
}

/// The rights the panel itself runs with.
pub fn own_rights() -> Rights {
    rights_behind(unsafe { GetCurrentProcess() })
}

/// Whether the active window's program is known to have rights above the panel's: it runs raised
/// and the panel is known not to. With either of the two unknown there is nothing to say.
pub fn rights_above(front: Rights, own: Rights) -> bool {
    front == Rights::Raised && own == Rights::Plain
}

fn rights_of_process(pid: u32) -> Rights {
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return Rights::Unknown };
        let rights = rights_behind(process);
        let _ = CloseHandle(process);
        rights
    }
}

/// The same for a process that is already open.
fn rights_behind(process: HANDLE) -> Rights {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return Rights::Unknown;
        }
        let mut raised = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let read = GetTokenInformation(token, TokenElevation, Some(&mut raised as *mut TOKEN_ELEVATION as *mut _), size, &mut len);
        let _ = CloseHandle(token);
        match read {
            Ok(()) if raised.TokenIsElevated != 0 => Rights::Raised,
            Ok(()) => Rights::Plain,
            Err(_) => Rights::Unknown,
        }
    }
}

fn class_of(hwnd: HWND) -> String {
    let mut class = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut class) };
    String::from_utf16_lossy(&class[..len.max(0) as usize])
}

/// The process whose program a window shows. Store apps such as the calculator are drawn inside a
/// frame window that belongs to a process of Windows shared by all of them; the app's own window
/// is a child of the frame, in the app's process. For a frame that holds no app at the moment (a
/// minimized one) there is no program to name. Any other window shows its own process.
fn shown_by(hwnd: HWND, pid: u32) -> Option<u32> {
    if class_of(hwnd) != "ApplicationFrameWindow" {
        return Some(pid);
    }
    struct Find {
        frame: u32,
        app: u32,
    }
    unsafe extern "system" fn each(child: HWND, lp: LPARAM) -> BOOL {
        let find = &mut *(lp.0 as *mut Find);
        let mut pid = 0u32;
        GetWindowThreadProcessId(child, Some(&mut pid));
        if pid != find.frame {
            find.app = pid;
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut find = Find { frame: pid, app: 0 };
    unsafe {
        let _ = EnumChildWindows(Some(hwnd), Some(each), LPARAM(&mut find as *mut Find as isize));
    }
    (find.app != 0).then_some(find.app)
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

/// The full path of an open process's program file.
fn path_behind(process: HANDLE) -> Option<String> {
    let mut path = [0u16; 1024];
    let mut len = path.len() as u32;
    unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(path.as_mut_ptr()), &mut len).ok()? };
    Some(String::from_utf16_lossy(&path[..len as usize]))
}

/// The full paths of the running processes whose program file is named `exe`, in any case. Only
/// the processes of that name are opened.
pub fn process_paths(exe: &str) -> Vec<String> {
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    let exe = exe.to_lowercase();
    let mut found = Vec::new();
    unsafe {
        let Ok(list) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return found };
        let mut entry = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut more = Process32FirstW(list, &mut entry).is_ok();
        while more {
            let len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..len]).to_lowercase() == exe {
                if let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, entry.th32ProcessID) {
                    found.extend(path_behind(process));
                    let _ = CloseHandle(process);
                }
            }
            more = Process32NextW(list, &mut entry).is_ok();
        }
        let _ = CloseHandle(list);
    }
    found
}

/// The same for a process that is already open.
fn program_behind(process: HANDLE) -> Option<(String, Option<String>)> {
    unsafe {
        let path = path_behind(process);
        let mut family = [0u16; 256];
        let mut family_len = family.len() as u32;
        // 0 is ERROR_SUCCESS; an unpackaged program answers APPMODEL_ERROR_NO_PACKAGE
        let packaged = GetPackageFamilyName(process, &mut family_len, Some(PWSTR(family.as_mut_ptr()))).0 == 0;
        let path = path?;
        let exe = path.rsplit(['\\', '/']).next()?.to_lowercase();
        let family = packaged
            .then(|| String::from_utf16_lossy(&family[..family_len.saturating_sub(1) as usize]))
            .filter(|f| !f.is_empty());
        Some((exe, family))
    }
}

/// COM on this thread for as long as this lives. Every start that worked, one that found COM
/// already started included, needs its own end; made before the COM objects of its scope, it ends
/// COM after they are gone.
pub struct Com(bool);

impl Com {
    pub fn new(model: COINIT) -> Com {
        Com(unsafe { CoInitializeEx(None, model) }.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

/// The path of a known folder, named by its id as in `1AC14E77-02E7-4E5D-B744-2EB1AE5198B7`, with
/// or without braces; `None` for an id this machine does not know.
pub fn known_folder(id: &str) -> Option<String> {
    let id = id.trim().trim_start_matches('{').trim_end_matches('}');
    let guid = GUID::try_from(id).ok()?;
    unsafe {
        let path = SHGetKnownFolderPath(&guid, KF_FLAG_DEFAULT, None).ok()?;
        let text = path.to_string().ok();
        CoTaskMemFree(Some(path.0 as *const _));
        text.filter(|t| !t.is_empty())
    }
}

/// System.AppUserModel.ID: the id of the app a window stands for, which the taskbar groups it by.
const APP_ID: PROPERTYKEY = PROPERTYKEY { fmtid: GUID::from_u128(0x9f4c2855_9f79_4b39_a8d0_e1d42de1d5f3), pid: 5 };

/// The id of the app a window was given, which the taskbar then groups it by; `None` when it was
/// given none. Needs COM on this thread.
fn window_app_id(hwnd: HWND) -> Option<String> {
    unsafe { SHGetPropertyStoreForWindow::<IPropertyStore>(hwnd) }
        .and_then(|store| unsafe { store.GetValue(&APP_ID) })
        .ok()
        .map(|value| value.to_string())
        .filter(|id| !id.trim().is_empty())
}

/// The id of the packaged app a process runs; `None` for a plain program.
fn process_app_id(process: HANDLE) -> Option<String> {
    let mut id = [0u16; 256];
    let mut len = id.len() as u32;
    // 0 is ERROR_SUCCESS; a process of no package answers APPMODEL_ERROR_NO_APPLICATION
    let found = unsafe { GetApplicationUserModelId(process, &mut len, Some(PWSTR(id.as_mut_ptr()))) }.0 == 0;
    found.then(|| String::from_utf16_lossy(&id[..len.saturating_sub(1) as usize])).filter(|id| !id.is_empty())
}

/// A window of a program as the meters look for its progress.
pub struct ProgramWindow {
    pub hwnd: isize,
    /// The full path of the program's file.
    pub path: String,
    /// The id of the app the window stands for, when asked for and it has one: the one the window
    /// was given, or else the one of the packaged app its process runs.
    pub app: Option<String>,
}

/// Every visible window of other programs, front to back: dialogs a window owns, tool windows and
/// windows without a title included, since a program may show its progress only in such a
/// dialog. Not the list a key goes by (`app_windows`). With `with_app` each window's app id is
/// read too, which needs COM on this thread.
pub fn program_windows(with_app: bool) -> Vec<ProgramWindow> {
    struct Look {
        with_app: bool,
        /// What is known of each process seen in this look: its program's path and its app id.
        processes: Vec<(u32, Option<String>, Option<String>)>,
        found: Vec<ProgramWindow>,
    }
    unsafe extern "system" fn each(hwnd: HWND, lp: LPARAM) -> BOOL {
        let look = &mut *(lp.0 as *mut Look);
        // The panel's own windows first, before anything is asked of them (see app_windows)
        let pid = owner_of(hwnd);
        if pid == GetCurrentProcessId() || !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }
        let Some(pid) = shown_by(hwnd, pid) else { return BOOL(1) };
        let known = match look.processes.iter().position(|p| p.0 == pid) {
            Some(at) => at,
            None => {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok();
                let path = process.and_then(path_behind);
                let app = if look.with_app { process.and_then(process_app_id) } else { None };
                if let Some(process) = process {
                    let _ = CloseHandle(process);
                }
                look.processes.push((pid, path, app));
                look.processes.len() - 1
            }
        };
        let (_, path, process_app) = &look.processes[known];
        let Some(path) = path.clone() else { return BOOL(1) };
        let app = if look.with_app { window_app_id(hwnd).or_else(|| process_app.clone()) } else { None };
        look.found.push(ProgramWindow { hwnd: hwnd.0 as isize, path, app });
        BOOL(1)
    }
    let mut look = Look { with_app, processes: Vec::new(), found: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut look as *mut Look as isize));
    }
    look.found
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
        // A Store app's frame stands for the app inside it, and an empty frame for nothing
        let Some(pid) = shown_by(hwnd, pid) else { return BOOL(1) };
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
/// that is not in front do this only on behalf of the one that is, so this thread joins the input
/// of the active window's thread for the call. No key is sent: a tapped Alt opens the menu of the
/// window in front, and one held across the change may never come up.
pub fn bring_to_front(hwnd: isize) -> Result<(), String> {
    let window = HWND(hwnd as _);
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindowAsync(window, SW_RESTORE);
        }
        // Windows does not let this thread join a window of a program with rights above the
        // panel's; the call is tried all the same
        let (me, other) = (GetCurrentThreadId(), GetWindowThreadProcessId(GetForegroundWindow(), None));
        let joined = other != 0 && other != me && AttachThreadInput(me, other, true).as_bool();
        let front = SetForegroundWindow(window).as_bool();
        if front {
            let _ = BringWindowToTop(window);
        }
        if joined {
            let _ = AttachThreadInput(me, other, false);
        }
        if front {
            return Ok(());
        }
        // Refused: the window is at least put above the others, where it is seen. One that stays
        // above the others by itself is raised among those and keeps staying above
        let stays_above = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
        let _ = SetWindowPos(window, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        if !stays_above {
            let _ = SetWindowPos(window, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        Err("Windows refused to bring the window to the front".into())
    }
}

pub fn minimize(hwnd: isize) {
    unsafe {
        let _ = ShowWindowAsync(HWND(hwnd as _), SW_MINIMIZE);
    }
}

/// Keeps a window off the taskbar for good: a tool window gets no button, whatever the taskbar
/// remembers. Call before the window has a region; changing the style drops it.
pub fn tool_window(hwnd: isize) {
    unsafe {
        let ex = GetWindowLongPtrW(HWND(hwnd as _), GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(HWND(hwnd as _), GWL_EXSTYLE, ((ex & !WS_EX_APPWINDOW.0) | WS_EX_TOOLWINDOW.0) as isize);
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

/// One window of one run of a program: a handle Windows may give to another window once this one
/// is gone, the process that owns it, and when that process started, since its number may be
/// given to another process too.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowId {
    pub hwnd: isize,
    pub pid: u32,
    pub born: u64,
}

/// A window of another program that stays above the others, on the screen now.
#[derive(Debug, PartialEq)]
pub struct TopWindow {
    pub id: WindowId,
    /// The program's file name, lower case.
    pub exe: String,
    pub family: Option<String>,
    /// The full path of the program's file: where its icon is, and what a docked entry goes by.
    pub path: String,
    /// The window's class: with the path, what tells the ticked window from the program's others.
    pub class: String,
    /// What shows of the window, `(x, y, width, height)` in physical px: its rectangle without
    /// the invisible borders Windows gives a window that can be resized.
    pub rect: (i32, i32, i32, i32),
    /// The window's own corner minus the corner of what shows of it: what a move adds.
    pub border: (i32, i32),
    /// Parts of it can be seen through: it is layered, drawn without a redirection bitmap (as a
    /// transparent web page is) or cut by a window region.
    pub see_through: bool,
}

/// What shows of a window and its invisible borders (see `TopWindow`).
fn frame_of(hwnd: HWND) -> Option<((i32, i32, i32, i32), (i32, i32))> {
    let (mut outer, mut shows) = (RECT::default(), RECT::default());
    unsafe {
        GetWindowRect(hwnd, &mut outer).ok()?;
        if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut shows as *mut RECT as *mut _, std::mem::size_of::<RECT>() as u32).is_err() {
            shows = outer;
        }
    }
    Some(((shows.left, shows.top, shows.right - shows.left, shows.bottom - shows.top), (outer.left - shows.left, outer.top - shows.top)))
}

fn owner_of(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

/// When a process started, as Windows counts time; 0 when it would not say.
fn born_of(process: HANDLE) -> u64 {
    let (mut born, mut ended, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    match unsafe { GetProcessTimes(process, &mut born, &mut ended, &mut kernel, &mut user) } {
        Ok(()) => (born.dwHighDateTime as u64) << 32 | born.dwLowDateTime as u64,
        Err(_) => 0,
    }
}

/// The windows of other programs that stay above the others: visible, not minimized, not cloaked,
/// not the taskbar or the desktop, not covering their whole screen, and big enough to be a panel.
/// Asks nothing of the windows themselves, front to back.
pub fn top_windows() -> Vec<TopWindow> {
    unsafe extern "system" fn each(hwnd: HWND, lp: LPARAM) -> BOOL {
        let found = &mut *(lp.0 as *mut Vec<TopWindow>);
        // The panel's own windows first, before anything is asked of them (see app_windows)
        let pid = owner_of(hwnd);
        if pid == GetCurrentProcessId() || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOPMOST.0 == 0 {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4);
        let class = class_of(hwnd);
        if cloaked != 0 || matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return BOOL(1);
        }
        let Some((rect, border)) = frame_of(hwnd) else { return BOOL(1) };
        if rect.2 < 16 || rect.3 < 16 {
            return BOOL(1);
        }
        let mut screen = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut screen).as_bool() {
            let m = screen.rcMonitor;
            if rect.0 <= m.left && rect.1 <= m.top && rect.0 + rect.2 >= m.right && rect.1 + rect.3 >= m.bottom {
                return BOOL(1);
            }
        }
        // A window region is asked for into a region of our own; ERROR says it has none
        let region = CreateRectRgn(0, 0, 0, 0);
        let cut = GetWindowRgn(hwnd, region) != GDI_REGION_TYPE(0);
        let _ = DeleteObject(region.into());
        let see_through = ex & (WS_EX_LAYERED.0 | WS_EX_NOREDIRECTIONBITMAP.0) != 0 || cut;
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return BOOL(1) };
        let path = path_behind(process);
        let program = program_behind(process);
        let born = born_of(process);
        let _ = CloseHandle(process);
        if let (Some(path), Some((exe, family))) = (path, program) {
            found.push(TopWindow { id: WindowId { hwnd: hwnd.0 as isize, pid, born }, exe, family, path, class, rect, border, see_through });
        }
        BOOL(1)
    }
    let mut found: Vec<TopWindow> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut Vec<TopWindow> as isize));
    }
    found
}

/// Whether a window is still the one seen: its handle belongs to the same process, and that is the
/// same run of a program, since Windows may give a closed process's number to a new one.
pub fn still(id: WindowId) -> bool {
    if owner_of(HWND(id.hwnd as _)) != id.pid {
        return false;
    }
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, id.pid) else { return false };
        let born = born_of(process);
        let _ = CloseHandle(process);
        born == id.born
    }
}

/// Whether a window is still where and as large as it was seen, and still the one seen.
pub fn unchanged(window: &TopWindow) -> bool {
    still(window.id) && frame_of(HWND(window.id.hwnd as _)).is_some_and(|(rect, _)| rect == window.rect)
}

/// Moves a window of another program so what shows of it starts at `(x, y)`: not resized, not
/// raised and not activated, and its place among the windows that stay above the others kept.
/// The move is handed to the window's own thread, so a program that hangs holds up nothing here.
/// A handle that belongs to another process, or to another run of the program, by now is not
/// touched. Returns where the window shows right after, which a quick program has already
/// reached; a refusal says Windows' reason, as a window of a program with rights above the
/// panel's gets.
pub fn place_other(window: &TopWindow, x: i32, y: i32) -> Result<(i32, i32), String> {
    let hwnd = HWND(window.id.hwnd as _);
    if !still(window.id) {
        return Err("the window is gone".into());
    }
    let (bx, by) = window.border;
    unsafe {
        SetWindowPos(hwnd, None, x + bx, y + by, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS)
            .map_err(|e| format!("{} ({:#x})", e.message(), e.code().0))?;
    }
    let (wx, wy, _, _) = window_rect(window.id.hwnd).ok_or("the window is gone")?;
    Ok((wx - bx, wy - by))
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
mod tests {
    use super::*;

    #[test]
    fn keys_windows_took_none_or_a_part_of_are_a_failure() {
        assert!(all_taken(0, 1).is_err());
        assert!(all_taken(0, 6).is_err());
        assert!(all_taken(5, 6).is_err());
    }

    #[test]
    fn a_window_is_above_the_panel_only_when_both_rights_are_known() {
        assert!(rights_above(Rights::Raised, Rights::Plain));
        // The panel could not read its own rights, or runs raised itself: no such reason is given
        assert!(!rights_above(Rights::Raised, Rights::Unknown));
        assert!(!rights_above(Rights::Raised, Rights::Raised));
        assert!(!rights_above(Rights::Unknown, Rights::Plain));
        assert!(!rights_above(Rights::Plain, Rights::Plain));
    }

    #[test]
    fn rights_windows_would_not_tell_are_unknown_not_raised() {
        // No process has the number 0 for a program to open, and no handle is 0
        assert_eq!(rights_of_process(0), Rights::Unknown);
        assert_eq!(rights_behind(HANDLE::default()), Rights::Unknown);
    }

    #[test]
    fn a_running_program_is_found_by_its_file_name() {
        let own = std::env::current_exe().unwrap();
        let name = own.file_name().unwrap().to_string_lossy().to_uppercase();
        let found = process_paths(&name);
        assert!(found.iter().any(|p| p.eq_ignore_ascii_case(&own.to_string_lossy())), "{found:?}");
        assert!(process_paths("no-such-program-here.exe").is_empty());
    }
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
