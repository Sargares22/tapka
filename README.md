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
- **Keys per app.** Set a key's *Where to show* to one program, and it appears while that program's window is in front. The first time, the capsule says *Keys for …*.
- **Shift stays down for the pen.** A tap on *Hold Shift* (or Ctrl, Alt, Space) keeps the key down while you draw; the next tap lets it go.
- **Touch feedback.** A tapped key flashes and dips; a quiet click can be switched on in Look.
- **Add without typing.** Programs pinned to the taskbar, Windows Settings pages and ready sets go in with a tap.
- **Your look.** Dark, light or system theme, any accent, three sizes, Russian and English. Updates are quiet, once per start, and can be switched off.

Ready-made Windows actions: snip an area, paste, copy, undo, redo, select all, voice typing, task view, show desktop, File Explorer, Settings, on-screen keyboard, clipboard history, emoji, louder, quieter, mute, play/pause, next track, hold Shift, Ctrl, Alt or Space.

## Your own keys

Tap the **+** on the capsule, or open the settings from the tray icon.

<p align="center"><img src="assets/settings.png" width="760" alt="The settings window"></p>

**Add** offers five kinds of keys:

- **Program:** the programs pinned to your taskbar on top, every installed one below; its name and icon come along.
- **Windows action:** one of the ready-made ones, or a Settings page such as Bluetooth, Wi-Fi, sound, display or Windows Update.
- **Website or file:** type an address, or choose a file or folder.
- **Keyboard shortcut:** press it, or build it with the Ctrl, Shift, Alt and Win buttons.
- **Ready set:** several keys at once: Tablet, Drawing or Text. A set can go to one program right away; keys already in the list are not added twice.

Drag keys by the dots to reorder. A tap on a row opens the key's card: name, hint, icon, where to show, and *Remove*, with an Undo.

### The settings file

Everything lives in `%APPDATA%\sargares22.tapka\settings.json`; the settings window opens it for you (General → Advanced → Settings file). Edit it by hand, with a script or through an AI agent: the panel picks changes up within a second.

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

- `action`: `open` with `target` (program, file, folder, address or `shell:AppsFolder\…`), or `hotkey` with `keys`, like `ctrl+shift+k`, or `hold` with `keys` set to `shift`, `ctrl`, `alt` or `space`.
- `icon`: a built-in icon (all are in the editor) or a PNG or SVG from the `icons` folder next to the file. Without it, a program shows its own icon, anything else its first letter.
- `only_in`: a program's file name or Store package family, like `"only_in": "mspaint.exe"`. The key is on the capsule only while that program's window is active, right after the first two keys for all programs. The editor sets it too: *Where to show*.
- `top` (0 to 1) places the capsule along its edge; `tablet_only: true` hides it while a keyboard is attached. The exact `top` and the `lit` rule are file-only.
- A broken item is skipped, and the tray tooltip says what is wrong.

## Recipes

Tapka is not tied to any other program. Here is what people connect:

- **Dictation (Handy and the like).** Add a *Keyboard shortcut* key with the shortcut it listens to, say `ctrl+space`. To make the key glow while recording, give the item a `lit` rule: the program and the title of its recording window.

  ```json
  { "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space",
    "lit": { "program": "handy.exe", "window": "Recording" } }
  ```

- **A pen and a held Shift.** *Add → Windows action → Hold Shift* (or Ctrl, Alt, Space). A tap presses the key and keeps it down while you draw, the key glows, the next tap lets it go. Forgotten, it lets go by itself after 20 seconds without a tap on the capsule, or when another program comes to the front. Meanwhile the other keys work as usual, together with the held one. For one program only:

  ```json
  { "name": "Shift", "icon": "shift", "action": "hold", "keys": "shift", "only_in": "mspaint.exe" }
  ```

- **For artists.** *Add → Ready set*, under *Where to show* pick *In one program* and your drawing app (Paint, Krita, Photoshop), then *Add* next to Drawing. Undo, Redo, Hold Shift, Ctrl and Space and Snip appear on the capsule only while it is in front.
- **AI chats.** Add an app (ChatGPT, Claude, Copilot) as *Program*, a web chat as *Website or file*.
- **A folder you live in.** *Add → Website or file → Folder…*
- **Deep links.** The address field takes any link Windows opens: `ms-settings:bluetooth`, `ms-availablenetworks:` (Wi‑Fi), `ms-actioncenter:` (notifications), `claude://code/new?folder=C:\Projects\mine` and `codex://threads/new?path=C:\Projects\mine` (a new session in that folder), `tg://resolve?domain=name`. Schemes depend on what is installed; see [awesome-deeplinks](https://github.com/f/awesome-deeplinks).
- **Another program's shortcut.** Ctrl, Shift, Alt, Win plus a letter, digit, F1–F24, navigation key, media key, period or comma all work. The numpad is not supported yet, and keys do not reach programs running as administrator.

## Privacy

What the panel does:

- sends key presses to the active window when you tap a key that carries a shortcut or holds a key;
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
