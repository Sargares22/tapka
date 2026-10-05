<p align="center"><b>EN</b> · <a href="README.ru.md">RU</a></p>

<p align="center"><img src="assets/logo.png" width="112" alt="Tapka"></p>

<h1 align="center">Tapka</h1>

<p align="center">
  <a href="https://github.com/Sargares22/tapka/releases/latest"><img src="https://img.shields.io/github/v/release/Sargares22/tapka?color=8ab4ff" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/Windows%2011-ARM64%20%7C%20x64-8ab4ff" alt="Windows 11, ARM64 and x64">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-8ab4ff" alt="MIT"></a>
</p>

<p align="center">
  <a href="#install">Install</a> · <a href="#what-it-does">What it does</a> · <a href="#your-own-keys">Your own keys</a> · <a href="#recipes">Recipes</a> · <a href="#privacy">Privacy</a> · <a href="CHANGELOG.md">What's new</a>
</p>

Tapka is a touch panel for Windows 11: a narrow capsule at the edge of the screen with large keys. A key sends a keyboard shortcut to the window you were working in, or launches a program, opens a site, a file or a folder. Made for tablets and 2-in-1s: when the keyboard is detached, snipping, paste, dictation and volume go with it, and the taskbar, even placed at the side, sends no shortcuts. The capsule can hide itself while a keyboard is attached. Works with a finger, a pen and a mouse.

<p align="center">
  <img src="assets/capsule-dark.png" width="300" alt="The capsule, dark theme">
  <img src="assets/capsule-light.png" width="300" alt="The capsule, light theme with another accent">
</p>

https://github.com/user-attachments/assets/895c44cc-11f4-4ec9-96a5-64749f640b62

## Install

1. Download the installer from [Releases](https://github.com/Sargares22/tapka/releases/latest): `Tapka_…_arm64-setup.exe` for ARM (Snapdragon), `Tapka_…_x64-setup.exe` for Intel and AMD.
2. Run it. The installer is not signed, so Windows shows "Windows protected your PC": press **More info**, then **Run anyway**.
3. No administrator rights needed. Tapka appears at the right edge and opens its settings once.

To remove it: Windows Settings → Installed apps. Antivirus nervous because the panel sends key presses? See [Privacy](#privacy).

## What it does

- **Never takes focus.** The shortcut goes to the window you were in.
- **Only without a keyboard.** With this setting on (Settings → Look), the capsule hides while a keyboard is attached. The tray icon shows or hides it at any moment.
- **Snip without the capsule in the shot.** The Snip key hides the capsule while you select the area.
- **Four keys in sight.** The rest scroll under your finger, and the **+** at the foot opens the editor.
- **Stays where you put it.** Drag the capsule by its handle to the left, right or top edge; it stays there, even after a rotation.
- **Like the taskbar, but for fingers.** Without a keyboard the keys spread apart. A running program's key carries a mark, and a second tap brings its window forward or minimizes it; Tapka never closes anything.
- **Your look.** Dark, light or system theme, any accent, three sizes, Russian and English. Updates are quiet, once per start, and can be switched off.

Ready-made Windows actions: snip an area, paste, copy, undo, voice typing, task view, show desktop, File Explorer, Settings, on-screen keyboard, clipboard history, emoji, louder, quieter, mute, play/pause, next track.

## Your own keys

Tap the **+** on the capsule, or open the settings from the tray icon.

<p align="center"><img src="assets/settings.png" width="760" alt="The settings window"></p>

**Add** offers four kinds of keys:

- **Program:** pick an installed one; its name and icon come along.
- **Site, file or folder:** type an address, or choose a file or folder.
- **Keyboard shortcut:** press it, or build it with the Ctrl, Shift, Alt and Win buttons.
- **Windows action:** one of the ready-made ones.

Drag keys by the dots to reorder; the pencil edits name, hint and icon; the bin removes, with an Undo.

### The settings file

Everything lives in `%APPDATA%\sargares22.tapka\settings.json`; the settings window opens it for you (General → Settings file). Edit it by hand, with a script or through an AI agent: the panel picks changes up within a second.

```json
{
  "scale": 1.0,
  "top": 0.3,
  "edge": "right",
  "theme": "system",
  "accent": "#8ab4ff",
  "items": [
    { "name": "Snip", "icon": "snip", "hint": "Select a screen area", "action": "hotkey", "keys": "win+shift+s" },
    { "name": "Mail", "icon": "chat", "action": "open", "target": "https://mail.example.com" }
  ]
}
```

- `action`: `open` with `target` (program, file, folder, address or `shell:AppsFolder\…`), or `hotkey` with `keys`, like `ctrl+shift+k`.
- `icon`: a built-in icon (all are in the editor) or a PNG or SVG from the `icons` folder next to the file. Without it, a program shows its own icon, anything else its first letter.
- `top` (0 to 1) places the capsule along its edge; `tablet_only: true` hides it while a keyboard is attached. The exact `top` and the `lit` rule are file-only.
- A broken item is skipped, and the tray tooltip says what is wrong.

## Recipes

Tapka is not tied to any other program. Here is what people connect:

- **Dictation (Handy and the like).** Add a *Keyboard shortcut* key with the shortcut it listens to, say `ctrl+space`. To make the key glow while recording, give the item a `lit` rule: the program and the title of its recording window.

  ```json
  { "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space",
    "lit": { "program": "handy.exe", "window": "Recording" } }
  ```

- **AI chats.** Add an app (ChatGPT, Claude, Copilot) as *Program*, a web chat as *Site, file or folder*.
- **A folder you live in.** *Add → Site, file or folder → Folder…*
- **Deep links.** The address field takes any link Windows opens: `ms-settings:bluetooth`, `ms-availablenetworks:` (Wi‑Fi), `ms-actioncenter:` (notifications), `claude://code/new?folder=C:\Projects\mine` and `codex://threads/new?path=C:\Projects\mine` (a new session in that folder), `tg://resolve?domain=name`. Schemes depend on what is installed; see [awesome-deeplinks](https://github.com/f/awesome-deeplinks).
- **Another program's shortcut.** Ctrl, Shift, Alt, Win plus a letter, digit, F1–F24, navigation key, media key, period or comma all work. The numpad is not supported yet, and keys do not reach programs running as administrator.

## Privacy

What the panel does:

- sends key presses to the active window when you tap a key that carries a shortcut;
- reads the list of windows and their titles twice a second, to mark the programs that are running;
- with update checks on, contacts GitHub once per start.

What it does not do: it does not record what you type, does not send window titles anywhere, and collects no statistics. The log stays on your computer and is limited in size.

## Build it yourself

You need Rust with the MSVC toolchain and Node.js.

```
cargo test
node --test scripts/check-page.mjs
npx @tauri-apps/cli@2 build --target aarch64-pc-windows-msvc --no-bundle
```

The exe lands in `target/aarch64-pc-windows-msvc/release`; use `x86_64-pc-windows-msvc` for Intel and AMD. Your own build does not update itself: signed updates come from `scripts/make-release.sh` with the project's key.

## License

[MIT](LICENSE), no warranty; the icons are drawn for this project, see [glyphs/NOTICE.md](glyphs/NOTICE.md).
