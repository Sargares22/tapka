<p align="center"><b>EN</b> · <a href="README.ru.md">RU</a></p>

<p align="center"><img src="assets/logo.png" width="112" alt="Tapka"></p>

<h1 align="center">Tapka</h1>

<p align="center">
  <a href="https://github.com/Sargares22/tapka/releases/latest"><img src="https://img.shields.io/github/v/release/Sargares22/tapka?color=8ab4ff" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/Windows%2011-ARM64%20%7C%20x64-8ab4ff" alt="Windows 11, ARM64 and x64">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-WTFPL-8ab4ff" alt="WTFPL"></a>
</p>

<p align="center">
  <a href="#install">Install</a> · <a href="#what-it-does">What it does</a> · <a href="#your-own-keys">Your own keys</a> · <a href="#recipes">Recipes</a> · <a href="#privacy">Privacy</a> · <a href="CHANGELOG.md">What's new</a>
</p>

Tapka is a touch panel for Windows 11: a narrow capsule at the screen edge with big keys. A key starts a program, opens a site, a file or a folder, or sends a keyboard shortcut to the window you were working in. It is made for tablets and 2-in-1s, where the keyboard is folded away and with it go snipping, pasting and dictation. Works with a finger, a pen and a mouse.

<p align="center">
  <img src="assets/capsule-dark.png" width="300" alt="The capsule, dark theme">
  <img src="assets/capsule-light.png" width="300" alt="The capsule, light theme with another accent">
</p>

https://github.com/user-attachments/assets/e760fc27-4987-4b22-ab8c-c1023a3250f2

## Install

1. Open [Releases](https://github.com/Sargares22/tapka/releases/latest) and download the installer for your processor: `Tapka_…_arm64-setup.exe` for ARM devices (Snapdragon), `Tapka_…_x64-setup.exe` for Intel and AMD.
2. Run it. The installer is not signed, so Windows shows "Windows protected your PC": press **More info**, then **Run anyway**.
3. Tapka installs for the current user, without administrator rights, and starts.

On the first start you get a capsule at the right edge with Windows actions that work at once, and the settings window opens one time. To remove Tapka, use "Installed apps" in Windows Settings.

Some antivirus programs are wary of anything that sends key presses, and Tapka does exactly that when you tap a shortcut key. If yours raises an alarm, see [Privacy](#privacy) for what the panel does and does not do; the source is all here.

## What it does

- **Never takes focus.** You tap a key, the shortcut goes to the window you were in, and that window stays active.
- **Snip without the capsule in the shot.** The Snip key hides the capsule while you select the area and brings it back when you are done.
- **Behaves like the taskbar.** A key whose program is running carries a mark; tapping it again brings the window forward or minimizes it. Tapka never closes anything.
- **Four keys in sight, the rest a scroll away.** The list scrolls under your finger; a faded edge says there is more. A small **+** at the foot of the capsule opens the editor.
- **Stays where you put it.** Drag the capsule by its handle to the left, right or top edge. After a screen rotation or a resolution change it is back at its edge.
- **Wider apart without a keyboard.** When the keyboard is detached, the keys stand further apart for fingers.
- **Your look.** Dark, light or the same as Windows; any accent colour; three sizes.
- **Russian and English.** Follows the language of Windows; can be switched in the settings.
- **Quiet updates.** One check per start, a line in the settings and an Update button. No pop-ups. Can be switched off.

Ready-made Windows actions: snip an area, paste, copy, undo, voice typing, task view, show desktop, File Explorer, Settings, on-screen keyboard.

## Your own keys

Tap the **+** at the foot of the capsule, or open the settings from the tray icon.

<p align="center"><img src="assets/settings.png" width="760" alt="The settings window"></p>

**Add** offers four kinds of keys:

| Kind | What you do |
|---|---|
| Program | Pick it from the list of installed programs. The name and the icon come from the program itself. |
| Site, file or folder | Type an address or choose a file or a folder. |
| Keyboard shortcut | Press it on a keyboard, or build it with the Ctrl, Shift, Alt and Win buttons when there is no keyboard. |
| Windows action | Pick one of the ready-made actions. |

Drag a key by the dots to change the order; the pencil changes its name, hint and icon; the bin removes it, with an Undo.

### The settings file

Everything lives in one file, `settings.json`, in `%APPDATA%\sargares22.tapka`; the settings window opens it for you (General → Settings file). You, a script or an AI agent can edit it by hand: the panel picks the change up within a second, and the settings window shows the same list.

```json
{
  "scale": 1.0,
  "top": 0.3,
  "edge": "right",
  "theme": "system",
  "accent": "#8ab4ff",
  "items": [
    { "name": "Snip", "icon": "snip", "hint": "Select a screen area", "action": "hotkey", "keys": "win+shift+s" },
    { "name": "Notes", "action": "open", "target": "C:\\Tools\\notes.exe" },
    { "name": "Mail", "icon": "chat", "action": "open", "target": "https://mail.example.com" }
  ]
}
```

- `action` is `open` (with `target`: a program, a file, a folder, an address, or a `shell:AppsFolder\…` app address) or `hotkey` (with `keys`, like `ctrl+shift+k`).
- `icon` is a built-in name (`snip`, `paste`, `copy`, `undo`, `mic`, `tasks`, `desktop`, `folder`, `gear`, `keyboard`, `terminal`, `chat`, `browser`) or a PNG/SVG file from the `icons` folder next to the settings file. Without it, a program shows its own icon and anything else shows its first letter.
- `top` is where the capsule starts along its edge, from 0 to 1.
- Two things can only be set here: the exact `top`, and the `lit` rule below.

A broken item is skipped and the rest keep working; a broken file changes nothing, and the tray tooltip says what is wrong.

## Recipes

Tapka is not tied to any other program. These are examples of what people connect.

**A dictation program (Handy and the like).** Add a *Keyboard shortcut* key with the shortcut your dictation program listens to, for example `ctrl+space`. To make the key glow while it records, add a `lit` rule to that item in the settings file, naming the program and the title of the window it shows while recording:

```json
{ "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space",
  "lit": { "program": "handy.exe", "window": "Recording" } }
```

**AI chats.** Installed apps (ChatGPT, Claude, Copilot and so on): *Add → Program* and pick the app. Web chats: *Add → Site, file or folder* and paste the address. A second tap on an app's key brings its window forward, like on the taskbar.

**A folder you live in.** *Add → Site, file or folder → Folder…*

**A shortcut of another program.** If an action has a keyboard shortcut made of Ctrl, Shift, Alt, Win and a letter, a digit, F1–F24 or a navigation key, it can be a key: mute in a call, a screenshot tool. Punctuation, numpad and media keys are not supported yet. Shortcuts do not reach programs running as administrator: Windows does not let an ordinary program send keys to them.

## Privacy

What the panel does:

- sends key presses to the active window when you tap a key that carries a shortcut;
- reads the list of windows and their titles twice a second, to mark the programs that are running;
- with update checks on, contacts GitHub once per start.

What it does not do: it does not record what you type, does not send window titles anywhere, and collects no statistics. The log stays on your computer and is limited in size.

## Build it yourself

You need Rust with the MSVC toolchain and Node.js (for the Tauri command-line tool).

```
cargo test
node --test scripts/check-page.mjs
npx @tauri-apps/cli@2 build --target aarch64-pc-windows-msvc --no-bundle
```

The exe is in `target/aarch64-pc-windows-msvc/release` (use `x86_64-pc-windows-msvc` for Intel and AMD). Installers with signed update files are made by `scripts/make-release.sh` and need the project's update key, so a build of your own does not update itself.

Rust, Tauri 2 and WebView2; one binary, two pages, one settings file.

## License

[WTFPL](LICENSE). The program comes as is, with no warranty of any kind: if something breaks, it is yours to keep. The built-in icons are drawn for this project; see [glyphs/NOTICE.md](glyphs/NOTICE.md).
