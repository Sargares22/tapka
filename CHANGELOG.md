# Changelog

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
