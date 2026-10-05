---
name: tapka-preview
description: Start Tapka on a throwaway data folder and take pictures of its pages, to see a change working. Use after any change to the capsule, the settings window, or Win32 behaviour.
---

Two tools, for two kinds of looking.

**Pictures of the pages, no Windows calls.** `node scripts/shot.mjs` renders `ui/index.html`
and `ui/settings.html` with sample data in both themes into `target/shots`. Open the PNGs and
look: layout, texts, icons, themes.

**The real thing.** `sh scripts/preview.sh` builds the release exe and starts it with
`TAPKA_DATA_DIR` pointing at an empty temporary folder, so it is a first run and the user's
own settings are untouched. It prints the folder; `panel.log` in it shows taps, errors and
settings reloads, and an edit of `settings.json` there is picked up within a second.

Tapka is single-instance. If it is already running, the script stops and says so: quit Tapka
from its tray icon, or pass `--replace` to let the script end it.

What only a person or a screenshot can confirm: the capsule does not take focus, the handle
drags it to another edge, the label appears on hover, presses beside the capsule reach the
window behind it.
