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

https://github.com/user-attachments/assets/1622f2fe-5a0e-46c8-a63a-c13895f1acb4

## Install

1. Download the installer from [Releases](https://github.com/Sargares22/tapka/releases/latest): `Tapka_…_arm64-setup.exe` for ARM (Snapdragon), `Tapka_…_x64-setup.exe` for Intel and AMD.
2. Run it. The installer is not signed, so Windows shows "Windows protected your PC": press **More info**, then **Run anyway**.
3. No administrator rights needed. Tapka appears at the right edge with four keys: Undo, Snip, the on-screen keyboard and File Explorer. With a keyboard attached the on-screen keyboard is Redo instead, and on a screen that takes a pen File Explorer is Hold Shift. Beside the capsule a small window offers up to eight of your programs, those pinned to the taskbar first, then those with open windows; *Add* makes keys of the ticked ones, and the capsule shows them and says *Tap a key*. *Not now* or a tap anywhere else closes it for good.

### Install with an AI agent

Paste this into Claude Code, Codex or another coding agent on your computer. It checks the machine, helps you install Tapka, finds your programs, asks you once which to add, and checks that they show up. Later, ask the same agent to add more programs.

```text
Install Tapka for me and add my programs, instructions: https://raw.githubusercontent.com/Sargares22/tapka/main/docs/install.txt
```

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

Ready-made Windows actions: snip an area, paste, copy, undo, redo, select all, voice typing, task view, show desktop, File Explorer, Settings, on-screen keyboard, clipboard history, emoji, louder, quieter, mute, play/pause, next track, hold Shift, Ctrl, Alt or Space, Esc, the four arrows, Space, full screen.

## Your own keys

Tap the **+** on the capsule, or open the settings from the tray icon.

<p align="center"><img src="assets/settings.png" width="760" alt="The settings window"></p>

**Add** offers five kinds of keys, and *Find my programs* under the list brings back the first start's card of your programs:

- **Program:** the programs pinned to your taskbar on top, every installed one below; its name and icon come along.
- **Windows action:** one of the ready-made ones, or a Settings page such as Bluetooth, Wi-Fi, sound, display or Windows Update.
- **Website or file:** type an address, or choose a file or folder.
- **Keyboard shortcut:** press it, or build it with the Ctrl, Shift, Alt and Win buttons.
- **Ready set:** several keys at once: Tablet, Drawing, Text or Video. A set can go to one program right away; keys already in the list are not added twice. *Save my keys…* writes your keys, all or those of one program, to a `.tapka` file to share; *Set from a file…* shows such a file as one more set and adds its keys to yours.

Drag keys by the dots to reorder, or right on the capsule: hold a key until its label shows, then carry it to its new place. A tap on a row opens the key's card: name, hint, icon, where to show, and *Remove*, with an Undo.

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
- `length`: how many keys long the capsule may be, at least 2; dragging the bar at its end after a tap on *+* sets it. The capsule is never longer than its keys and meters need, so it shows no empty cells. Without it the capsule is as long as its keys, up to four, and all its meters.
- `top` (0 to 1) places the capsule along its edge; `tablet_only: true` hides it while a keyboard is attached. The exact `top` and the `lit` rule are file-only. `tour_done: true` is written by Tapka once the first-start window with your programs has closed.
- A broken item is skipped, and the tray tooltip says what is wrong.

### Meters

*Add a meter* under Meters in the settings window puts a small gauge under the keys: a rounded square like a key, whose outline fills from the middle of its top. It shows the progress of a program you pick from the installed ones (what it shows on its taskbar button, a progress bar in its windows, or a percent at the start of its title), or the numbers in a text file that any script or program can write, in UTF-8 or UTF-16.

A key of a program needs no meter for its own progress: while the program shows progress on its taskbar button, a copy in File Explorer for one, the key's edge fills, and it clears when the progress ends.

The file's first line holds one or two numbers from 0 to 100, separated by a space: the first fills the gauge, the second, if there is one, a thin second outline just outside it. Up to five more lines, 80 characters each, show under the meter's name and percent when you hover over it or hold it. Above 80 % the gauge and the outline turn to the warning colour.

```text
64 88
Disk D: 64 %
Copied 1240 of 1930 files
```

A few lines of the file are for Tapka itself and do not show as text. Times are ISO 8601: `2026-10-26T11:00` in this computer's local time, or with `Z` or an offset like `+03:00`.

| Line | What it does |
|---|---|
| `updated: <time>` | When your script last got the numbers. Without it, the time the file was last written |
| `fresh: <minutes>` | How old the numbers may be, 5 minutes if not given. Older, the gauge turns grey, keeps its last percent, and its label adds *updated 12 min ago* |
| `reset: <time>` | When the gauge's number resets. The label counts down, *resets in 2 h 33 min*, or gives the day, *resets Mon 26 Oct, 11:00*, when it is more than a day away; once the time has passed, *due, waiting for an update*, and the percent stays until your file changes |
| `reset2: <time>` | The same for the second outline |

The Russian keys `обновлено:`, `свежесть:`, `сброс:` and `сброс2:` work too. A file that is missing or has no number on its first line shows a grey gauge with *—*, and its label says *no data*; it never shows 0 %.

```text
64 30
updated: 2026-10-21T14:05:00+03:00
fresh: 15
reset: 2026-10-21T19:00
reset2: 2026-10-26T11:00
Week: 30 %
```

The meters show all at once while the capsule has room for them; the keys take the rest and scroll. The thin line between keys and meters moves the border: drag it down and the keys get more room while the meters scroll; drag it up and the meters grow until all of them show, after which the capsule simply gets shorter.

Where a meter gets its number:

| Way | What fits | What to do |
|---|---|---|
| Works by itself | Copying in File Explorer and other programs that show progress on their taskbar button | *Add a meter* → *Program* → pick it |
| Works where the program shows its progress to Windows | Renders, downloads, installers: a progress bar in a window, or a percent at the start of the title | The same. An empty grey meter means the program does not show its progress |
| Needs your script | Limits, battery, free space, any number of your own | A script writes the number to a text file; a *File* meter reads it |

#### Your own source in 10 minutes

1. Write a script that puts the number on the first line of a text file. Hint lines go below it.
2. In Tapka: *Add a meter* → *File*, and pick that file.
3. Start the script when you sign in: a shortcut in the Startup folder (`shell:startup`).

```powershell
$file = "$env:USERPROFILE\Documents\battery.txt"
while ($true) {
  $b = (Get-CimInstance Win32_Battery).EstimatedChargeRemaining
  Set-Content $file "$b`nBattery: $b %" -Encoding UTF8
  Start-Sleep -Seconds 60
}
```

Scripts for other programs live apart from Tapka: Tapka has no code for any of them. An AI agent writes such a script for you (see [Install with an AI agent](#install-with-an-ai-agent)).

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
