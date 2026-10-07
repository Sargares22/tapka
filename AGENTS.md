# Tapka — notes for coding agents

Tapka is a touch panel for Windows 11: a capsule of big keys at the screen edge that starts
programs and sends keyboard shortcuts. Rust + Tauri 2 + WebView2. One binary, two pages, one
settings file. What it does for the user is in `README.md`; this file is the map.

## Map

| File | What lives there |
|---|---|
| `src/main.rs` | App state, Tauri commands, tray, the watcher threads, updater |
| `src/config.rs` | The settings file: parsing, writing, ready-made Windows actions (`PRESETS`) |
| `src/actions.rs` | Pure logic: a tap becomes a list of steps. Tested without Windows |
| `src/win.rs` | Every Win32 call: keys, windows, regions, the desktop scan |
| `src/grab.rs` | The invisible handle window Windows drags, and the landing slot |
| `src/layout.rs` | Capsule geometry |
| `src/keys.rs` | Shortcut strings like `win+shift+s` |
| `src/apps.rs`, `src/shellicon.rs` | Installed programs and their icons |
| `src/autostart.rs`, `src/glyphs.rs` | Start with Windows; built-in icons |
| `src/stories.rs` | Tests named after user stories |
| `ui/index.html` | The capsule page |
| `ui/settings.html` | The settings window; all its texts are in the `T` dictionary (ru, en) |

## Commands

- `sh scripts/check.sh` — every automated check. Run it before you say you are done.
- `sh scripts/preview.sh` — build and start on a throwaway data folder (a first run).
- `node scripts/shot.mjs` — pictures of both pages in `target/shots`, no Windows calls.
- `powershell -File scripts/windows-of.ps1` — the running panel's windows, their styles, and whether the taskbar shows a button for it (should be none).
- `sh scripts/make-release.sh` — installers; needs the project's update key, so not for you.

Skills with step-by-step recipes are in `.agents/skills` (Claude Code: `.claude/skills`).

## Rules

- No paths, package names or names of other products in the code. Users add those themselves.
- Fields of the settings file the panel does not know must survive every write.
- Code and comments in English. Interface texts in both `ru` and `en`.
- No abstractions for later. If the task does not need it, it is not there.

## Traps

- The capsule never takes focus. A window that never takes focus gets no hover, so Rust polls
  the cursor (`watch_cursor`) and tells the page.
- The capsule window never changes size; what shows is cut out with a window region
  (`win::set_region`). Changing the window's style drops the region: see `win::quiet_frame`.
- Dragging is done by Windows through the handle window in `src/grab.rs`. Only
  `grab::set_visible` shows or hides it.
- Show or hide the capsule only through `set_capsule_visible` in `src/main.rs`: it remembers
  the choice (`wanted`), which the tray, the keyboard rule (`tablet_only`) and the return after
  a snapshot all go by.
- Geometry is in physical pixels: CSS pixels times `px_per_css`.
- Write the settings file only through `change_settings` in `src/main.rs`.
- Do not hold a lock across a Win32 call; `win::app_windows` skips the panel's own windows for
  the same reason.
- The page tests run in headless Chrome with a stand-in for Tauri. They prove nothing about
  WebView2 or Win32: use `scripts/preview.sh` and look.
