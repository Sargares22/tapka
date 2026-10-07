# Changelog

## 1.2.0 (2026-10-07)

- Keys for one program: an item with `only_in` (a program's file name like `paint.exe`, or its Store package family) is on the capsule only while that program's window is active, right after the first two keys for all programs. The first time in a start a program brings its keys, the capsule says so once ("Keys for …"). The editor sets it in *Where to show*.
- Keys held for the pen: an item with `"action": "hold"` and `"keys"` set to `shift`, `ctrl`, `alt` or `space` keeps that key down until the next tap on it and glows meanwhile. It lets go by itself after 20 seconds without a tap on the capsule, when another program comes to the front, when the capsule hides, and when Tapka quits. Four new ready-made Windows actions add them: Hold Shift, Hold Ctrl, Hold Alt, Hold Space, each with its own icon.
- One *Add* for everything, with five kinds: *Program* (the programs pinned to your taskbar on top, then the whole Start menu), *Windows action* (the ready-made actions and ten Windows Settings pages: Bluetooth, Wi-Fi, sound, display, night light, battery, notifications, printers, Windows Update, pen), *Website or file*, *Keyboard shortcut* and *Ready set* (Tablet, Drawing, Text: several keys at once, for every program or for one; keys already in the list are not added again).
- New ready-made Windows actions: Redo (Ctrl+Y) and Select all (Ctrl+A).
- The settings window, redone: cards stand out from the page by shade, not lines; each item reads in words (*Program*, *Website*, *Shortcut Ctrl+V*, *Settings: Bluetooth*) instead of its path or address, and an item without a picture shows a sign of its kind instead of a letter. A tap on a row opens the item's card, where it is also removed; the path and the shortcut are under *Details*, the built-in icons under *Change…*. *Look* shows the themes and the capsule as pictures; the accent palette is the blue range of Windows 11, `#8ab4ff` still first, and buttons darken it until their white letters read. *Quit* is left to the tray; the settings file and the log are under *Advanced*.
- Feedback on a tap: the key dips and flashes faintly in the accent for a moment. A quiet click can be added with *Sound on tap* (Settings → Look, `click_sound` in the file); it is off by default.
- Tapka's windows no longer show on the taskbar, the settings window included: the tray item *Settings…* or the *+* key brings it back to the front, restored if minimized.

## 1.1.0

- New ready-made Windows actions: louder, quieter, mute, play/pause, next track, clipboard history (Win+V) and emoji (Win+.), each with its own icon.
- A shortcut can use the sound and music keys, the period and the comma.
- The licence is MIT (it was WTFPL).
- The taskbar can now be placed at the side of the screen in Windows 11; the README and the spec say what Tapka still does that the taskbar does not.
- README: a recipe for deep links (ms-settings:, claude://, codex:// and the like as keys).

## 1.0.3

- New switch, *Only without a keyboard* (Settings → Look): the capsule hides while a keyboard is attached and comes back when it is detached or folded back. Off by default. The tray icon still shows or hides the capsule at any moment.
- A capsule hidden from the tray stays hidden after a screenshot taken a moment earlier.

## 1.0.2

- Less processor use at rest: the cursor is polled slowly while it is away from the capsule, the desktop is not looked over while the capsule is hidden, and a window's program is asked about once.
- The handle no longer reappears on its own while the capsule is hidden for a screenshot.
- The intro video plays on the repository page.

## 1.0.1

- Settings saved at the same moment from two places no longer overwrite each other.
- The editor lists and lets you remove items that were typed into the settings file wrongly, instead of failing on them.
- An imported icon no longer replaces another picture with the same file name.
- "Start with Windows" is written through the registry API, and removing Tapka removes the entry.
- Turning update checks off right after start is respected.
- The **+** is a small quiet mark at the foot of the capsule.

## 1.0.0

The first public release.

- A capsule at the left, right or top screen edge with big keys; it never takes focus.
- Keys start programs, open sites, files and folders, and send keyboard shortcuts to the active window.
- Ready-made Windows actions: snip an area, paste, copy, undo, voice typing, task view, show desktop, File Explorer, Settings, on-screen keyboard.
- A settings window in the manner of Windows 11: an editor of the keys (add, edit, reorder, remove), look, general, about.
- A program is added from the list of installed ones and shows its own icon.
- Four keys in sight, the rest scroll; the **+** key at the end opens the editor.
- Dark and light themes or the same as Windows; any accent colour; three sizes.
- Russian and English.
- The capsule stays at its edge after a screen rotation or a resolution change.
- A running program's key carries a mark and works like its taskbar button.
- A key can glow while another program shows a given window (the `lit` rule in the settings file).
- One settings file, picked up within a second after any edit.
- Installers for ARM64 and x64, for the current user, without administrator rights.
- Quiet update checks with an Update button in the settings; can be switched off.
