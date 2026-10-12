# Where things live

Tapka is one binary, two pages and one settings file. Rust + Tauri 2 + WebView2.

## Words

- **Capsule** — the panel on the screen: a borderless window at a screen edge that never takes
  focus. One web page is drawn in it.
- **Pill** — the dark strip with the keys, drawn by the page inside the capsule's window. The
  window is larger than the pill; what is not the pill is cut away (the *region*).
- **Key**, **item** — one button of the pill; an item is its entry in the settings file.
- **Foot** — the end of the pill after the keys. At rest it shows the plus; a tap puts two
  **signs** there for a few seconds: the gear (settings) and the arrow (fold).
- **Handle** — the bar at the head of the pill. Over it lies the **handle window**: a separate
  invisible native window that Windows drags like a title bar. Moving the capsule this way is a
  **carry**; the **slot** is the outline that shows where it will land.
- **Folded** — the capsule shrunk to one key at the edge. The handle window then lies over
  that key.
- **Hidden** — the capsule is not on the screen. `wanted` is what the person asked for (the
  tray, or the rule *only without a keyboard*, `tablet_only`); a **snapshot** (Win+Shift+S)
  hides it for a moment whatever `wanted` says.
- **Held key** — a modifier a key keeps down (the action `hold`) until it is tapped again.
- **Mark** — the dot on a key whose program is running.
- **Label** — the name and hint shown beside a key; also where a failure is said.

## Files

| File | What lives there |
|---|---|
| `src/main.rs` | App state, Tauri commands, tray, the watcher threads, updater |
| `src/config.rs` | The settings file: parsing, writing, ready-made Windows actions (`PRESETS`), ready sets (`BUNDLES`), key sets in files |
| `src/actions.rs` | Pure logic: a tap becomes a list of steps; which items show; the held keys; whose hide a snapshot ends. Tested without Windows |
| `src/win.rs` | Win32: keys, windows, rights, visibility, regions, the desktop scan |
| `src/grab.rs` | The handle window and the slot; tells a tap on the folded key from a carry |
| `src/layout.rs` | Capsule geometry, pure: where the window, the pill, the folded key and the handle go |
| `src/keys.rs` | Shortcut strings like `win+shift+s` |
| `src/apps.rs`, `src/shellicon.rs` | Installed programs and their icons |
| `src/autostart.rs`, `src/glyphs.rs` | Start with Windows (the registry); built-in icons |
| `src/stories.rs` | Tests named after user stories |
| `ui/index.html` | The capsule page |
| `ui/settings.html` | The settings window; all its texts are in the `T` dictionary (ru, en) |
| `glyphs/` | The built-in icons, one SVG each; what each is for is in `glyphs/NOTICE.md` |
| `Cargo.toml`, `tauri.conf.json` | Dependencies; the windows, the bundle and the updater |
| `capabilities/default.json` | What the pages are allowed to call |
| `installer/hooks.nsh` | The installer's own steps: autostart is cleared on uninstall |
| `scripts/` | Checks, the checklist, preview, pictures of the pages, a look at the panel's windows, icons, the release build, the sync of the public copy |

The settings file is JSON, settings.json, in the panel's data folder
(`%APPDATA%\sargares22.tapka`, or the folder named by `TAPKA_DATA_DIR`). The log, panel.log,
is beside it.

# What leads to what

**A tap on a key.** The capsule page (`ui/index.html`) decides what the press was. An ordinary
key calls the command `tap` with the item's place in the settings file. The plus opens the
signs (`foot`), the arrow calls `fold`; the gear calls `tap` with no place, which opens the
settings window. `tap` in `src/main.rs` turns the item into steps with `src/actions.rs` and
runs them (`run_steps`, `run_step`) through `src/win.rs`. A step that failed goes to the log,
the steps after it still run, and the page gets the event `failed` and shows the label.

**The settings file.** `src/config.rs` reads it. `watch_settings` in `src/main.rs` re-reads it
within a second of a change and tells the pages (`reload`). What the panel changes while it
runs goes through `change_settings`; the first file of a first run and the move from an older
data folder are written by `src/config.rs` itself. The settings window (`ui/settings.html`)
reads with `get_settings` and writes with `save_items` and `set_pref`.

**The program in front.** Every half second, while the capsule is shown and not being carried,
`follow_front` asks `win::front_program` which program is active. When the set of items for it
changes (`actions::shown_items`), the page gets the event `set` and then asks for the keys
with `get_view`. Held keys are let go when the program changes. The same loop works out the
marks of running programs separately (`marks`).

**Where the capsule is.** `src/layout.rs` computes; `place_capsule` in `src/main.rs` moves and
sizes the window, cuts it with `apply_region`, and puts the handle window of `src/grab.rs`
over the handle, or over the folded key. A carry ends in `carry_end`, which writes the edge
and the place to the settings file: `top` for the capsule, `folded_at` for the folded key.

**Shown, hidden, folded.** Three separate things. `set_capsule_visible` changes what is asked
for (`wanted`). A snapshot hides and shows the window by itself and leaves `wanted` alone
(`hide_capsule_until_snapshot`). Folding is `set_fold`, kept in the settings file as `folded`;
an edit of the file by hand is taken up at the next read.

**The settings window.** `open_settings_window` makes it on demand and brings it to the front.
It is made anew after each close.

# Commands and events

Every command a page can call and every event Rust sends is named here; the checklist fails
when one is missing.

**Commands of the capsule page:** `get_view` (everything it draws), `tap`, `foot` (the signs
open or close, so the window is sized), `fold`, `unfold`, `card` (the label opens or closes,
so the region includes it), `move_item` (a key was carried to another place in the list).

**Commands of the settings window:** `get_settings`, `set_pref`, `save_items`,
`installed_apps`, `pinned_apps`, `running_programs`, `icon_of`, `import_icon`, `bundle_items`
(a ready set), `save_key_set`, `open_key_set`, `key_set_items` (a key set in a file),
`check_update`, `install_update`, `open_settings_file`, `open_log_folder`, `open_github`,
`quit`.

**Events to the pages:** `reload` (the settings changed), `set` (the items for the program in
front changed), `marks` (which programs run), `held` (which keys are held), `hover` (the
cursor over the capsule, which a window without focus is not told), `carry` (Windows began to
carry the capsule), `click` (a tap the page did not see, on the folded key: for the sound),
`failed` (a step did not work: `at`, `title`, `why`), `section` (which part of the settings
window to show).
