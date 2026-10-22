# Changelog

## 1.4.2 (2026-10-22)

- The capsule no longer stays hidden behind other programs' windows that keep above the others: it comes back in front within half a second. A game or a video in full screen in front is left alone.

## 1.4.1 (2026-10-22)

- The capsule keeps its length while you drag the line between keys and gauges: only the bar at the far end changes it. The line moves one whole gauge at a time: drag it up to show one more gauge, down to one fewer. Every gauge can be shown whole as long as one key still fits; the keys scroll in the rest.
- The bar at the far end now stretches the capsule until every key and every gauge shows whole, whatever their number; the length counts the keys and the gauges together.
- Opening *+* near the end of the screen no longer makes the capsule shorter: it grows as far as the screen lets it.
- A press on a gauge shows its label at once, and the label stays a few seconds after you let go. Hold a gauge and drag it to put the gauges in another order, the way you reorder keys.

## 1.4.0 (2026-10-21)

- Meters: a block of small gauges under the keys, rounded squares of the keys' look whose outline fills from the middle of the top, behind a thin line you drag to share the capsule between keys and gauges. A gauge follows a program you pick from the installed ones (the progress Windows shows on its taskbar button, a progress bar in its windows, or a percent at the start of its title) or a number in a text file. A tap on a gauge works like a key of its program: it opens the program, brings it forward, or folds it away when it is in front.
- A key of a program that has a gauge shows the same progress as a ring along its edge. With no gauge set up at all, a key's edge fills the same way while its program shows progress on its taskbar button, a copy in File Explorer say, and clears when it ends.
- A gauge's text file can hold a second number, drawn as a thin second outline just outside the gauge, and up to five lines of text that show in the gauge's label; the format is in the README.
- A gauge tells fresh numbers from old ones. A file may say when its numbers were last read (`updated:`) and how many minutes they stay fresh (`fresh:`, 5 by default); without `updated:` the time the file was written counts. Older numbers turn the gauge grey with its last percent, and its label says *updated 12 min ago*. A file that is missing or holds no number shows a grey gauge with *—* and *no data*, never 0 %.
- A file may say when its numbers reset (`reset:`, and `reset2:` for the second outline). The label counts down, *resets in 2 h 33 min*, or names the day when it is more than a day off, *resets Mon 26 Oct, 11:00*, every minute without the file changing; once the time has passed it says *due, waiting for an update*.
- The gauges all show when the capsule has room for them, and the keys take the rest. Dragging the line up makes the gauges grow until all of them show, then the capsule gets shorter; dragging it down gives the keys more room and the gauges scroll.
- When a program of a key or a gauge updates into a newer version folder beside the old one, Tapka finds it and puts the new path in the settings; Items in the settings window says "Path updated". When it finds the program nowhere, in several places, or only in some other folder, the key turns grey and says "Program not found — tap to choose"; a tap opens its card in the settings window, which offers a program of that file found elsewhere with a "Use it" button. A program on a drive that is not plugged in is left as it is.
- A first start sets up four keys that all show without scrolling: Undo, Snip, the on-screen keyboard and File Explorer. With a keyboard attached, Redo takes the on-screen keyboard's place; on a screen that takes a pen, Hold Shift takes File Explorer's. A settings file you already have stays as it is.
- A first start no longer opens the settings window. A small window beside the capsule offers up to eight of your programs with their icons, those pinned to the taskbar first, then those with open windows, all ticked. *Add* makes keys of the ticked ones, and the capsule scrolls to them and says *Tap a key* until you tap one. *Not now*, or a tap anywhere else, closes the window for good.
- *Find my programs* under the list of keys in the settings window opens the same card at any time.
- The capsule's length is yours: tap *+* and drag the short bar at the far end of the capsule to make it longer or shorter, one key at a time. The length is kept in the settings as the most the capsule may take: it is never longer than its keys and gauges need, so it shows no empty cells, and it grows back when a program brings more keys. Until you drag, the capsule shows up to four keys and all its gauges.
- The gear, the arrow and the bar after a tap on *+* stay while the mouse is over the capsule, and go a few seconds after it leaves.
- A right click in Tapka's windows no longer opens the browser's menu (text fields keep theirs), and F5, Ctrl+R or Ctrl+P no longer reload or print them.
- README: a phrase that hands the setup to a coding agent, which installs Tapka with you and adds your programs as keys and gauges.

## 1.3.0 (2026-10-11)

- The capsule folds away: a tap on *+* puts two signs in its place for a few seconds, a gear that opens the settings window and an arrow that folds the capsule into one key at the edge. A tap on that key brings the capsule back; drag it, and it goes to the left, right or top edge, like the capsule by its handle. Folded stays folded after a restart.
- A key that did not work says so: a label *Didn't work* shows beside it for a moment, with the reason (Windows did not accept the key press, the target did not open, or the window in front runs as administrator). A *Hold* key glows only once the key really went down, and keeps glowing if it could not be let go.
- New ready-made Windows actions for a tablet without a keyboard: Esc, the four arrows, Space and Full screen (F11), each with its own icon, and a fourth ready set, *Video*.
- Key sets in files: *Ready set* now saves your keys to a `.tapka` file, all of them or those of one program, and adds the keys of someone else's file as one more card. Your own keys stay as they are; keys you already have, and keys that open something not on this machine, are left out. Only keys travel in the file: not the look, the place of the capsule, or picture files.
- A key that is mostly hidden under the fading end of the list no longer fires on a tap: the list scrolls to show it instead.
- Fixed: a press beside the *Add* dialog closed it and then fell on the row under it; the settings window came to the front only every other time.

## 1.2.1 (2026-10-10)

- Keys are reordered right on the capsule: hold a key until its label shows, then carry it along the capsule; the others make room and the release leaves it there. The order is saved to the settings file.
- The capsule shows whole at the large size and without a keyboard: where it did not fit between its place and the end of the screen it was cut short and lost keys; now it is pulled back along the edge.
- An icon that would be lost on its key reads in either theme: a white sign on a light key, or a black one on a dark key, is drawn in the ink of the theme like the built-in icons; a pale picture of several colours gets a plate under it.
- The settings window opens in the middle of the screen, and a second tap on *+* while it is opening does nothing.

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
