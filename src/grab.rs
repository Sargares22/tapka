//! Carrying the capsule with a finger, a pen or a mouse.
//!
//! The page cannot follow a pointer while its own window moves under it: the coordinates it gets
//! are relative to the window and lag behind (measured 2026-10-05: the capsule travelled about
//! half as far as the finger and flickered). So the carry is given to Windows. A small, all but
//! invisible window lies over the capsule's handle and answers every hit test with "this is a
//! title bar". Windows then moves that window itself, for every kind of pointer, and the capsule
//! is kept at the same offset from it.
//!
//! A second window, the slot, shows where the capsule would dock if it were let go now.
//!
//! Both windows live on their own thread with their own message loop.

use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::OnceLock;
use tauri::AppHandle;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, CreateSolidBrush, DeleteObject, FillRect, SetWindowRgn, HDC};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindowAsync, TranslateMessage, HTCAPTION, HWND_TOPMOST, LWA_ALPHA, MA_NOACTIVATE, MSG, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WM_ENTERSIZEMOVE, WM_ERASEBKGND, WM_EXITSIZEMOVE, WM_MOUSEACTIVATE, WM_MOVING,
    WM_NCHITTEST, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

static APP: OnceLock<AppHandle> = OnceLock::new();
static HANDLE: AtomicIsize = AtomicIsize::new(0);
static SLOT: AtomicIsize = AtomicIsize::new(0);

/// The panel's accent as Windows writes colours (0x00BBGGRR); #8ab4ff until the settings say otherwise.
static ACCENT: AtomicU32 = AtomicU32::new(0x00FF_B48A);

/// The slot is drawn in this accent, `#rrggbb`, from now on.
pub fn set_accent(accent: &str) {
    if let Ok(rgb) = u32::from_str_radix(accent.trim_start_matches('#'), 16) {
        ACCENT.store((rgb & 0xff) << 16 | rgb & 0xff00 | rgb >> 16 & 0xff, Ordering::Relaxed);
    }
}
/// How solid the slot is, out of 255.
const SLOT_ALPHA: u8 = 70;

fn handle() -> HWND {
    HWND(HANDLE.load(Ordering::Relaxed) as _)
}

fn slot() -> HWND {
    HWND(SLOT.load(Ordering::Relaxed) as _)
}

unsafe extern "system" fn handle_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        // Every point of this window is a title bar: Windows carries it for any pointer
        WM_NCHITTEST => LRESULT(HTCAPTION as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ENTERSIZEMOVE => {
            if let Some(app) = APP.get() {
                crate::carry_begin(app, hwnd.0 as isize);
            }
            LRESULT(0)
        }
        WM_MOVING => {
            // Windows proposes where the handle goes next; the capsule's limits may correct it
            let rect = &mut *(lparam.0 as *mut RECT);
            if let Some(app) = APP.get() {
                let (x, y) = crate::carry_step(app, rect.left, rect.top);
                let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);
                *rect = RECT { left: x, top: y, right: x + w, bottom: y + h };
            }
            LRESULT(1)
        }
        WM_EXITSIZEMOVE => {
            hide_slot();
            if let Some(app) = APP.get() {
                crate::carry_end(app);
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn slot_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_ERASEBKGND {
        // The whole slot is one colour: the accent as it is right now
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        let brush = CreateSolidBrush(COLORREF(ACCENT.load(Ordering::Relaxed)));
        FillRect(HDC(wparam.0 as _), &rect, brush);
        let _ = DeleteObject(brush.into());
        return LRESULT(1);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// Creates the handle and slot windows on a thread of their own and returns once they exist.
pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    let (ready, wait) = std::sync::mpsc::channel();
    std::thread::spawn(move || unsafe {
        let module = GetModuleHandleW(None).unwrap_or_default();
        let grab_class = WNDCLASSW {
            lpfnWndProc: Some(handle_proc),
            hInstance: module.into(),
            lpszClassName: w!("TapkaGrab"),
            ..Default::default()
        };
        let slot_class = WNDCLASSW {
            lpfnWndProc: Some(slot_proc),
            hInstance: module.into(),
            lpszClassName: w!("TapkaSlot"),
            ..Default::default()
        };
        RegisterClassW(&grab_class);
        RegisterClassW(&slot_class);
        let style = WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED;
        let make = |class: PCWSTR, alpha: u8| {
            let hwnd = CreateWindowExW(style, class, w!(""), WS_POPUP, 0, 0, 1, 1, None, None, Some(module.into()), None).ok()?;
            // Alpha 1 of 255 cannot be seen but still takes presses; alpha 0 would not
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
            Some(hwnd)
        };
        if let (Some(h), Some(s)) = (make(w!("TapkaGrab"), 1), make(w!("TapkaSlot"), SLOT_ALPHA)) {
            HANDLE.store(h.0 as isize, Ordering::Relaxed);
            SLOT.store(s.0 as isize, Ordering::Relaxed);
        }
        let _ = ready.send(());
        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    });
    let _ = wait.recv_timeout(std::time::Duration::from_secs(2));
}

/// Lays the handle window over the capsule's handle, `(x, y, width, height)` on the screen in
/// physical px, above the capsule. Placing never shows it: it is shown and hidden together with
/// the capsule by `set_visible` alone, so a capsule hidden for a screenshot keeps its handle
/// hidden whatever is placed meanwhile.
pub fn place(rect: (i32, i32, i32, i32)) {
    if HANDLE.load(Ordering::Relaxed) == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowPos(handle(), Some(HWND_TOPMOST), rect.0, rect.1, rect.2, rect.3, SWP_NOACTIVATE);
    }
}

/// Puts the handle window back above the capsule. The capsule is topmost too, and whichever of
/// the two was raised last is in front; a capsule in front swallows every press on the handle
/// (it did on 2026-10-05, when the capsule was shown after the handle was placed).
pub fn raise() {
    if HANDLE.load(Ordering::Relaxed) == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowPos(handle(), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

/// Hides or shows the handle window together with the capsule.
pub fn set_visible(visible: bool) {
    if HANDLE.load(Ordering::Relaxed) == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindowAsync(handle(), if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}

/// Shows the slot at `(x, y, width, height)` on the screen, rounded like the capsule (`radius` in
/// physical px), just under the capsule window `above`.
pub fn show_slot(rect: (i32, i32, i32, i32), radius: i32, above: isize) {
    if SLOT.load(Ordering::Relaxed) == 0 {
        return;
    }
    unsafe {
        let (x, y, w, h) = rect;
        // The system owns the region after the call
        let region = CreateRoundRectRgn(0, 0, w + 1, h + 1, radius * 2, radius * 2);
        let _ = SetWindowRgn(slot(), Some(region), true);
        let _ = SetWindowPos(slot(), Some(HWND(above as _)), x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        // Keep the capsule above its slot
        let _ = SetWindowPos(HWND(above as _), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }
}

pub fn hide_slot() {
    if SLOT.load(Ordering::Relaxed) == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindowAsync(slot(), SW_HIDE);
    }
}
