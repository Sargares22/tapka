# Where things live

Tapka is one binary, two pages and one settings file. Rust + Tauri 2 + WebView2.

## Words

- **Capsule** — the panel on the screen: a borderless window at a screen edge that never takes
  focus. One web page is drawn in it.
- **Pill** — the dark strip with the keys, drawn by the page inside the capsule's window. The
  window is larger than the pill; what is not the pill is cut away (the *region*).
- **Key**, **item** — one button of the pill; an item is its entry in the settings file.
- **Foot** — the end of the pill after the keys. At rest it shows the plus; a tap puts the
  **signs** there for a few seconds: the gear (settings), the arrow (fold) and the **end grip**,
  the handle's strip and bar again as the last part of the pill, which stretches the capsule by whole
  key cells (`length`).
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
- **Meter** — a small rounded square under the keys with a percent, filled along its outline:
  the progress of a program the user chose, or the number in a file the user chose (`meters` in
  the settings file). A file may also give a second number, drawn as a thin **outer ring**, a
  second outline just outside the square, and lines of **hint**,
  which the meter's label shows under its percent, and lines of **times**: when its writer last read
  the numbers, how long they stay fresh, when they reset. A meter of a file older than it allows is
  **stale**: grey, with its last percent. The keys and
  the meters are parted by the **divider**, a thin line pulled to share the room between them a
  whole meter at a time (`divider`, the share of the room where the keys' part ends). A meter of a program may also carry `app`, the id the
  program gives its windows. A key whose program has a meter shows it as a **ring**
  along its rim. A key of an `open` item without a meter gets the same ring, a **rim**, while its
  program shows progress on its taskbar button, with no setting at all.
- **Card of programs** — up to eight programs of the machine, all ticked, that *Add* makes keys
  of: the settings window opens it from the quiet line *Find my programs* under the items.
- **Welcome** — the card of programs alone in a small window of its own beside the capsule (the
  **welcome window**), on the first run only. Its end is `tour_done` in the settings file.
- **Tip** — the capsule's label beside the list after the card added keys: *Tap a key*, until the
  first tap on a key or six seconds.
- **Grey key** — a key whose program's exe is gone and was found nowhere else, in several places,
  or only in some other folder (*path repair*, `src/repair.rs`). Its label says so; a tap opens its
  card in the settings window instead of running it, and the card offers the one other place with
  *Use it*. A path found in exactly one place that is a newer version beside the old folder is
  *repaired*: written over the old one in the settings file.
- **Docked panel** — a window of another program that stays above the others and that the user
  ticked to stand under the capsule (`docked` in the settings file: the full path of its program
  and its window class; an older entry is the file name alone). The panel
  moves it there and keeps it there as the capsule moves; one that keeps going back to its own
  place, or that Windows will not let the panel move, is **let be**.

## Files

| File | What lives there |
|---|---|
| `src/main.rs` | App state, Tauri commands, tray, the watcher threads, updater |
| `src/config.rs` | The settings file: parsing, writing, ready-made Windows actions (`PRESETS`), ready sets (`BUNDLES`), key sets in files |
| `src/actions.rs` | Pure logic: a tap becomes a list of steps; which items show; the held keys; whose hide a snapshot ends. Tested without Windows |
| `src/win.rs` | Win32: keys, windows, rights, visibility, regions, the desktop scan, the running processes of a file name |
| `src/grab.rs` | The handle window and the slot; tells a tap on the folded key from a carry |
| `src/layout.rs` | Capsule geometry, pure: where the window, the pill, the folded key and the handle go |
| `src/meters.rs` | Where a docked window draws itself (`Reader::drawn`); the meters: a program's progress through UI Automation (its taskbar button, its windows' progress bars) or its window title, the number in a file; the rims of keys whose program shows taskbar progress |
| `src/dock.rs` | Docking, pure: which part of a docked window shows, where the docked panels stand under the capsule, which windows stand at its edge, when one that fights back is let be |
| `src/repair.rs` | Path repair, pure but for what the `Machine` it is given answers: which targets are exe paths, the newest sibling version folder that holds the exe, whether a find is the same installation, what the look found (`Same`, `Elsewhere`, none, several), the paths that are lost, the settings text with the old paths put right, the lines the settings window shows about it |
| `src/keys.rs` | Shortcut strings like `win+shift+s` |
| `src/apps.rs`, `src/shellicon.rs` | Installed and pinned programs and their icons; what the card of programs offers |
| `src/autostart.rs`, `src/glyphs.rs` | Start with Windows (the registry); built-in icons |
| `src/stories.rs` | Tests named after user stories |
| `ui/index.html` | The capsule page |
| `ui/settings.html` | The settings window, and opened as `settings.html#welcome` the welcome window; all its texts are in the `T` dictionary (ru, en) |
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
runs goes through `change_settings`, which changes the file as it is on the disk then and keeps
the version it replaces as settings.json.bak in the same step (`config::change_file`, `ReplaceFileW`), and
starts over from the file when an editor saved it meanwhile; the first file of a first
run and the move from an older data folder are written by `src/config.rs` itself. The settings window (`ui/settings.html`)
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
The capsule may be as long as its keys, up to four, and all its meters, or `length` cells for the keys
and the meters together (`layout::room_for`), and is never longer than its keys and meters need (`layout::room`).
Where the divider stands does not change the length: `layout::parts` shares that room, the meters a
whole number of meters, one or more and no more than all of them, the keys the rest, a key's cell or more. While the signs are open the window is as long as the most cells from the
pill's start that `layout::reach` gives (`stretched`), its region the whole strip. A press on the
end grip calls `resize` (`start`). The page draws the pill as long as `reach` of `get_view` says for
the cells the pointer is at, from its own pointer events or, for the mouse, the cursor Rust reports
(`hover`), and tells each step (`resize`, `step`, for the log); the window and its region stay as
they are until the release. The release calls `set_length`, which writes
`length`, and `top` too when the pill stood pulled back from the end of the work area; a release
without a move, or a press taken away, calls `resize` (`end`, with why). A change of what the drag
was worked out for ends it: Rust logs it and sends `stretch`. The log says
`length: drag start` with the pointer, `step`, `cancelled` and `saved`.

**Shown, hidden, folded.** Three separate things. `set_capsule_visible` changes what is asked
for (`wanted`). A snapshot hides and shows the window by itself and leaves `wanted` alone
(`hide_capsule_until_snapshot`). Folding is `set_fold`, kept in the settings file as `folded`;
an edit of the file by hand is taken up at the next read.

**The meters.** `watch_meters` in `src/main.rs` reads them about once a second while the capsule
is open on the screen, in two threads (`poll_meters`): the files (`meters::read_files`, UTF-8 or
UTF-16; `meters::file_reading` takes out the lines of times (`updated:`, `fresh:`, `reset:`,
`reset2:`, also in Russian, `meters::moment` for the time), then takes the first line that is not
blank for the percent and the outer ring, the next lines for the hint; `meters::dated` makes of the
times, with the file's own time when it gives no `updated:`, whether the meter is stale and the
lines its label says, in the language of the settings) and the programs (`poll_programs`, `meters::Reader`): the progress bar on the program's
taskbar button, matched by the full path of its exe (`meters::taskbar_names`, a known folder the
button names turned into its path by `win::known_folder`) or by the id it gives its windows (`app`
of the meter, kept from the program list), or for an app by its whole id; else a progress bar in
any visible window of the program, its dialogs included (`win::program_windows`,
`meters::window_of`), else a title of one that starts with a percent. Both put what they found
into `meters::Values`, a `meters::Reading` per meter: the percent, and from a file the outer ring,
the hint, `stale` and `notes`; a program's has none of them. A file is read each round whether it
changed or not, so the words about time change every minute and go to the page as any change. A changed value goes to the page as the event `meters`, with the generation
of the list it is for. The page draws the meters under the keys and the rings on the keys of the
same program (`meters::same_program`). The programs' thread reads the taskbar once a round
(`Reader::taskbar`), for those meters and for the rims: whenever an `open` item names an exe or an
app, with or without meters, while the capsule is open. `meters::rims` gives the items without a
meter whose program's button has progress (matched as a meter's program, with the id the Start
menu knows the exe gives its windows, `start_menu_ids`), kept in `rims` by target; a change of them, or
of the list of items, goes to the page as the event `rims`: the age of the items (`items_age`) and
`[place, percent]` per item. `get_view` gives each item its `rim` and the page the `age` of the
list it draws; the page drops an event of another age. The programs' own windows are never asked for a rim. Letting the divider go after a move calls
`set_divider`, which writes `divider`; the window does not change; a tap on a meter of a program calls `open_meter` with the meter's place, name and
program, which does what a tap on a key of the program does: it opens the program, brings it
forward, or folds it away when it is in front. A meter held and carried to another place calls
`move_meter` with the two places in the settings' list of meters and the meter's name and program
(`config::with_moved`, as `move_item` for the keys, which sends the key's name); the reload keeps the values read so far with their meters
(`meters::Values::renew`). The settings window writes the list with `save_meters`.

**Docking.** In the same half-second loop, while the capsule is shown, open and not carried,
`follow_docked` in `src/main.rs` finds the window of each entry of `docked` among those that stay
above the others (`win::top_windows`, `dock::pick`): the window it went by while that exists, else
the only one of its path and class at the capsule's edge; with several, none, and the settings
window asks to pick the panel again. A panel is often a large transparent window that draws a small
pill in it, so the column is laid out by the part each window draws: `watch_docked`, a thread of its
own, asks UI Automation for it (`meters::Reader::drawn`, `dock::part_of`) and keeps it in `shapes`
as an offset from the window's corner. Until a window's part is known nothing moves. The parts
stand flush to the capsule's edge, 8 logical px apart, in the order of the list: under the pill,
then above it; at the top edge to the right of the pill, then to its left (`dock::column`). Each
window is moved by the offset of its part (`win::place_other`), so a carry, a turned screen, a
restarted program and a move by the program itself are followed within half a second. A window
that fits on neither side stays where it is (`dock::overlap`). One that fights back or that Windows
refuses to move is let be (`dock::Moved`); the settings window, told by `reload`, shows it next to
that panel. The settings window offers each window that stays above the others at the capsule's
edge now (`dock_candidates`, `dock::at_edge`, `dock::shown_rect`) and writes the list with
`save_docked`, with the windows ticked. Removed from the list, a window stays where it is. The
rules of all this are in `docs/traps.md`, Docking.

**Path repair.** `check_paths` in `src/main.rs`, on a thread of its own, at start, after
`reload_settings` applied a changed file, and after a tap whose exe did not open because it is not
there. It hands the exe paths of the settings (`repair::targets`: `open` items, meters of a program,
docked entries with a path) to `repair::look` with `ThisMachine`. A path on a drive whose root is not
there or cannot be read is skipped; for each other one that is no file, the paths of the same file
name come from the Start menu (`apps::installed`, listed at most once a minute, which also refreshes
the list the editor shows) and the running processes (`win::process_paths`), and one from the newest
sibling folder with a version (`repair::newer_folder`, never an older version, and an equal one
counts as several); `repair::find` keeps those that are files. One that is a newer version beside
the old folder (`repair::same_install`) is `Same`: all such are written in one change
(`write_paths`: `change_settings`, `repair::with_paths`), logged, and each adds a line for the
settings window (`repair::Notices`, in memory). One anywhere else is `Elsewhere`; none or several are
what they say: these go into `lost` (`repair::Lost`, with the one other place) and are logged once. A
changed `lost` is told with `reload`: `get_view` marks the item `lost` and the page draws its key grey
with the words of `View::lost` in its label; `get_settings` marks the item with `lost` and `found` and
gives the docked entry the state `missing`. A tap on a grey key (`tap`) first looks whether the exe
is back (`Lost::back`); if so the key is drawn again and the tap opens it, else it puts the item's
place into `card` and opens the settings window, which opens that item's card (`get_settings` gives
`card`, by its place among all the items of the file) unless a card with edits not saved is open.
*Use it* on the card calls `use_path`, which writes the place `lost` holds, not one the page sends.
A card's save reads the settings again and puts only the fields the card changed onto the item as it
is now. A meter of a program that is gone finds nothing and stays empty and grey as any other.

**The settings window.** `open_settings_window` makes it on demand and brings it to the front.
It is made anew after each close; each new one drops the lines about repaired paths the last one
showed (`repair::Notices::opened`).

**The first run.** `load_settings` writes the first settings file with `config::default_text`:
four ready actions, Undo, Snip, the on-screen keyboard and File Explorer, with Redo for the
keyboard when one is attached (`win::tablet_mode` is off) and Hold Shift for File Explorer when the
screen takes a pen (`win::pen_present`), chosen once (`config::default_ids`); a file that is there
is never changed. The start that wrote it opens the welcome window (`open_welcome_window`), not the
settings window: the settings page as `settings.html#welcome`, without a frame or a taskbar button,
always on top, beside the pill on the side away from the edge and whole on the screen
(`layout::beside`, `welcome_place`). Each time its card is drawn the page measures it and calls
`fit_welcome`: the window becomes that tall, no larger than the work area, where the programs then
scroll, and is put beside the pill again. It closes on *Not now* and *Close* (the command `close_welcome`), after *Add*,
and when the page loses the focus; however it closes, `end_welcome` writes `"tour_done": true`
through `change_settings`, once. Nothing opens it again; a later start never does.

**The card of programs.** The welcome window shows it as soon as the settings come; the settings
window opens it as a dialog from the line *Find my programs* under the items. It asks `first_programs`:
the pinned programs (`apps::pinned`), then the programs of the open windows found in the Start menu
by the full path of their exe or the whole id of their app (`apps::of_windows`, `meters::window_of`),
through `apps::first_card` (without the panel, Windows' own programs that have a default key,
services such as uninstallers, updaters, setups, helpers, crash reporters and the Store
(`apps::service`, by file name and arguments), programs the items already open, repeats; eight at
most); the page asks `icon_of` for each. *Add* appends the ticked ones as `open` items through
`save_items`, without those the list holds by then, with `added`, how many it appended; Rust then
sends the capsule their places and the words of the tip as the event `added`, and the capsule
scrolls its list until the last of them is in view and brings them in softly, whichever comes
first, the event or the drawing that has them; once they stand in view it shows the tip. The card
closes only once the save is done; a save that failed keeps the card and says why. With nothing
found the card says so and offers only *Close*.

# Commands and events

Every command a page can call and every event Rust sends is named here; the checklist fails
when one is missing.

**Commands of the capsule page:** `get_view` (everything it draws), `tap`, `foot` (the signs
open or close, so the window is sized), `fold`, `unfold`, `card` (the label opens or closes,
so the region includes it), `move_item` (a key was carried to another place in the list),
`move_meter` (a meter was carried to another place among the meters),
`open_meter` (a tap on a meter of a program, with the meter's name and program), `set_divider` (the
divider was let go after a move: its place goes into the settings file), `resize` (the end grip is pressed, moved a cell, or let go without a move; nothing is saved),
`set_length` (the end grip was let go: the cells go into the settings file).

**Commands of the settings window** (the welcome window is the same page): `get_settings`, `set_pref`, `save_items`, `save_meters`,
`dock_candidates` (the panels at the capsule's edge now, each window apart), `save_docked` (with
the windows ticked), `use_path` (*Use it* on the card of a grey key), `installed_apps`, `pinned_apps`, `first_programs` (the programs the card of programs offers), `close_welcome` (the welcome window closes), `fit_welcome` (the welcome window's card is this tall), `running_programs`, `icon_of`, `import_icon`, `bundle_items`
(a ready set), `save_key_set`, `open_key_set`, `key_set_items` (a key set in a file),
`check_update`, `install_update`, `open_settings_file`, `open_log_folder`, `open_github`,
`quit`.

**Events to the pages:** `reload` (the settings changed, or a docked panel was let be), `set` (the items for the program in
front changed), `marks` (which programs run), `held` (which keys are held), `hover` (the
cursor over the capsule, which a window without focus is not told), `carry` (Windows began to
carry the capsule), `click` (a tap the page did not see, on the folded key: for the sound),
`failed` (a step did not work: `at`, `title`, `why`), `section` (which part of the settings
window to show), `rims` (the age of the items, and the items without a meter whose program shows
taskbar progress, with the percent), `added` (`at`, the places of the keys the card of programs
just added, which the capsule brings into view, and `tip`, the words it then says beside them),
`meters` (the generation of the list, `readings`, the count of changes of the values, and, for
each meter, `value`, `outer`, `hint`, `stale` and `notes`, or null where nothing was found), `stretch` (a drag of the end grip ended without a save:
what changed).

# Setup by an agent

The README gives a phrase for a coding agent with the address of `docs/install.txt`. That note
names the files of the skill `.agents/skills/tapka-setup/SKILL.md` by the release tag. The skill
installs Tapka with the person's help, finds their programs
(`.agents/skills/tapka-setup/scripts/candidates.ps1`), adds keys and meters to the settings file
after a backup of its own, and checks the result by the log and by reading the capsule through UI
Automation (`.agents/skills/tapka-setup/scripts/capsule.ps1`). Scripts that feed meters for other
programs (recipes) live outside this repository.
