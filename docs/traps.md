# Traps

Things in this code that look wrong and are not, and things that break when touched carelessly.
The words are explained in `docs/map.md`. Read this before changing any code.

## The windows

- The capsule never takes focus. A window that never takes focus gets no hover, so Rust polls
  the cursor (`watch_cursor`) and tells the page.
- The capsule window is larger than what shows; the rest is cut away with a window region
  (`win::set_region`). Changing the window's style drops the region: see `win::quiet_frame`.
- Carrying is done by Windows through the handle window in `src/grab.rs`. Only
  `grab::set_visible` shows or hides it.
- Every window that stays above the others shares one order with the capsule, and the one raised
  last is in front. The half-second loop of `watch_settings` raises the capsule (`win::raise`) and
  then the handle, in this order: a capsule above its handle swallows the presses on it. It raises
  neither while a window of another program covers its whole screen, and skips the capsule while
  one of the panel's own windows is active, so the welcome window stays above the capsule.
- What the person asked for goes through `set_capsule_visible` in `src/main.rs`: it remembers
  the choice (`wanted`), which the tray, the keyboard rule (`tablet_only`) and the return after
  a snapshot all go by. A snapshot hides the window for a moment without touching `wanted`.
- Geometry is in physical pixels: CSS pixels times `px_per_css`.
- Do not hold a lock across a Win32 call; `win::app_windows` skips the panel's own windows for
  the same reason.
- A window is brought to the front by attaching to the input of the thread in front
  (`win::bring_to_front`), with no key sent: a synthetic Alt tap opens the menu of the window
  in front, and Windows gives the foreground to nobody while a menu is open.

## The snapshot

- Win+Shift+S hides the capsule until the selection closes. Each hide has its number; only
  the latest one may bring the capsule back, and only if it is still wanted
  (`actions::ends_hide`). If the keys could not be sent, the capsule comes back first and the
  failure is said after.

## The folded capsule

- The commands fold and unfold through `set_fold` in `src/main.rs`; an edit of `folded` in the
  file is taken up when the settings are read again. Folded is kept apart from hidden
  (`wanted`) and from a snapshot in progress.
- Before folding, the held keys are let go. If Windows refuses, the capsule stays unfolded.
- The handle window lies over the folded key and carries it. A press that was not carried, or
  moved no more than the slack of `layout::is_tap`, is a tap and unfolds; the page does not
  get that press.
- A carry of the folded key writes `edge` and `folded_at`; a carry of the capsule writes
  `edge` and `top`.
- The folded window's region is the pill alone (`layout::folded_region`): a press beside the
  key must reach the window under it.

## The held keys

- A held key counts as held only once Windows took the press; a release Windows refused
  leaves it held and glowing (`src/actions.rs`, `Holds`).
- Held keys are let go on the next tap of their key, after 20 seconds without a tap on the
  capsule, when the program in front changes, when the capsule hides or folds, and on quit.
- A shortcut sent while a key is held leaves that modifier down.

## The page

- The page draws the pill as long as Rust says (`layout::pill_length`), not as long as the
  window happens to be: the window is resized a frame later than the page is drawn, and a pill
  tied to the window's height jumps.
- A key with less than two thirds of it clear of the fading end of the list does not fire on
  a tap; the list scrolls to show it.
- The page takes the pill's length for each number of cells from Rust (`reach` in the view), never
  from its own sums: the peek of the next key comes and goes with the cells. The divider never
  changes that length; the page shares the `room` Rust sends with `parts`, the same rule as
  `layout::parts`: the two must stay one rule.
- The signs stay while the mouse is over the pill (`overPill`), not over the window: the window is
  wider by the label's room. A finger or a pen moves the cursor to where it touched, so the cursor
  counts only when no finger or pen pressed since it last left (`afterTouch`); without that the signs
  never went after a touch.
- While the end grip is dragged the pill's start must not move: the page reads the cells from the
  pointer in window coordinates, and a window that moved under it would change them back and forth.
  So the window of a drag is placed from the start `layout::reach` gives, not from `top` with the
  pulled-back rule, and the most cells are what fits from there. A capsule pulled back against the
  end of the work area is only shortened in place; it is carried higher to be made longer.
- The signs make the pill one sign and the end grip longer (`ADD + STRETCH`); `fold_spot` counts the
  grip when it finds the fold sign's middle. The room of the keys and the meters is worked out with
  the plus and kept with the signs (`layout::room`): worked out again against the smaller room the
  work area leaves beside the signs, it was rounded down to fewer whole keys, and opening the signs
  made the pill shorter (found 22.10.2026). Only a pill with the signs past the work area is cut,
  to its end.
- `set_length` changes the settings in memory before the file, like `set_divider`; the page draws
  anew itself once the call returns. It applies in turn with `reload_settings` (`APPLYING`) and
  places the capsule from the settings as they are then: a copy taken before would undo an edge
  set meanwhile.
- While the signs are open the window is as long as the longest pill the end grip may reach, its
  region the whole strip, and neither changes during a drag: grown a step at a time it lagged
  behind the pointer, which left the window (seen 21.10.2026 with the mouse); grown on the press,
  or its region cut anew at each step, it changed under the finger, and Windows ended the touch
  after one step (seen 21.10.2026). So for the few seconds the signs are open the transparent rest
  of the window takes presses meant for what is behind it. The docked panels wait meanwhile.
- The end grip is the handle's 44 px strip inside the pill (`STRETCH` is `GRIP`). A target past the
  pill's end sent a finger to the panel under the capsule (seen 21.10.2026), and 18 px were missed.
- A press taken away (`pointercancel`) ends a drag without a save; a lost capture does not while
  the pointer's events still come, and ends it after `LOST_MS` when they stop. The log tells the
  three apart. The capsule never
  takes focus, so the mouse's movement may not reach the page even with a button down: the cursor
  Rust reports (`hover`) moves the grip too, for a mouse drag only. A finger or a pen leaves the
  cursor where it was, and with it the length wandered back to where it started (seen 21.10.2026).
- Drawings wait while the grip is dragged; only Rust ends a drag, when what it was worked out for
  changes (`Geometry`: the screen, the scale, the keyboard, the keys shown, the meters, the edge,
  the place, the saved length) and tells the page (`stretch`). Rust sends `set` and `reload` for
  other reasons in the middle of a drag; ending it on any drawing ended it on the first step.
- A step that failed reaches the page as the event `failed`. Rust drops a late one itself, if
  the list of items changed after the tap (`items_age`).

## The settings file and key sets

- What the running panel changes in the settings file goes through `change_settings` in
  `src/main.rs`, which reads the file as it is on the disk at that moment (`config::change_file`),
  never the copy in memory: the owner may have saved it in an editor a moment ago. The version
  replaced is kept as settings.json.bak by `ReplaceFileW` in the same step as the new one goes in:
  a copy and then a rename would lose a save that lands between them. Just before that the file is
  read again, and if an editor saved it while the new text was made, the change starts over from
  it, three times at most, then gives up and logs. `change` may therefore run more than once. A file
  that cannot be read or is not a JSON object is not written at all: the panel never puts defaults
  over the owner's file. The first file of a first run is written plainly: there is nothing to
  replace.
- Fields of the settings file the panel does not know must survive every write.
- A key set from a file (`parse_set`, `set_items` in `src/config.rs`) is untrusted: it is read
  strictly, never replaces the settings file, and no icon is looked up for a target on the
  network (`shellicon::remote`). Opening the file only shows its card; the items are written
  when *Add* is pressed, without those already in the list and those whose target is not on
  this machine.

## The meters

- Right after the panel starts, UI Automation can refuse to make a condition with E_FAIL; a moment
  later it works. `meters::Reader::new` tries five times, two seconds apart, and logs the step
  that failed. Seen on the owner's machine with the release build; a test run never hits it.
- Asking a program through UI Automation costs it something (a program built on Chromium turns
  its accessibility on when asked) and a program that hangs holds the question up to the
  timeout. So the programs are read in a thread of their own (`watch_meters`), apart from the
  files, with one-second timeouts; the taskbar is asked first, and a program's own windows only
  when its taskbar button has no progress, and never when the timeouts could not be set
  (`Reader::bounded`). Nothing is asked of programs no meter names, except the docked panels that
  can be seen through, whose drawn part `watch_docked` asks for (see Docking). The rims of keys
  without a meter come from the taskbar alone, Windows' own, read once a round for the meters and
  the rims together: never a program's windows or titles for a rim.
- A rim goes by the item's target, not its place: the list may change between a read and the
  drawing. `rims` is kept by target; the event names the places of the list as it was then, with
  its `items_age`, and the page drops an event whose age is not the one of the list it drew. A new
  list is told even with the same percents. `items_age` grows with the settings held, so a reader
  never takes a new age with the old items, or the other way round. A key with a meter of its own keeps the
  meter's ring and never gets a rim.
- The Start menu is listed for the rims (the id an exe gives its windows) on a thread of its own:
  the programs' thread runs COM multithreaded, and the shell's listing starts it apartment-threaded.
- Only a percent a title starts with counts: further on it is the zoom of a page or a name.
- A file's second number counts only right after the first, parted by spaces or a percent sign:
  `45/100` and `step 2 of 5: 40 %` are one percent and no ring (`meters::file_reading`). A first line
  without a percent is no value, and its hint is dropped with it.
- The values of a meter, its ring and its hint come as one `meters::Reading`: a changed hint with
  the same percent is news to the page too (`Values::publish` compares them whole).
- A meter's program is its full path, never its file name: two programs of one name in different
  folders are two programs, for the taskbar, the windows, the ring and a tap alike
  (`meters::taskbar_names`, `meters::window_of`, `meters::same_program`). The taskbar writes a
  known folder as its id in braces; `win::known_folder` turns it into a path, and only an id the
  machine does not know falls back to the file name. An app is its whole id, `Family!App`, also for
  its windows (their System.AppUserModel.ID, or the packaged app of their process): two apps of one
  package are two programs. The meters look in every visible window of the program, its dialogs and
  tool windows included (`win::program_windows`), not only in those a key goes by
  (`win::app_windows`, left as it was).
- COM is started on a thread by `win::Com`, which ends it when dropped: made before the COM objects
  of its scope (the last field of `meters::Reader`), so it ends after them.
- The pill is as long as its keys and meters need, up to `layout::room_for` (`layout::room`): every
  place that sizes the capsule window passes the number of meters, or the page draws a pill of
  another length than its window. The divider is not part of the length: when a pulled line made the
  pill shorter or longer, the window lagged behind it and cut the keys at its end (found 21.10.2026);
  now the line only moves the border inside a room of one length, and the window never changes for it.
- With meters the line steps by whole meters, not keys (`layout::parts`): the meters' part is one
  meter or more and no more than all of them, the keys' part the rest, a key's cell or more and no
  more than all keys, and it scrolls with the next key's edge fading. Stepped by whole keys, the
  line could never show three meters in a room of eight cells with eight keys (found 22.10.2026).
  Only a single key may find no such place in a room; `layout::room` then cuts the room back to one.
- A `length` counts cells for the keys and the meters together, with no fixed number of meters
  beside them: room for two meters on top of the cells once kept a third meter cut in half at the
  grip's last step, which stopped once the keys fitted (found 22.10.2026). The last step of `reach`
  is the fewest cells whose room holds every key and meter whole.
- The divider is where the keys' part ends, as a share of the room, and the share saved is the place
  drawn (whole meters, `parts`), never the pointer past the last key: a share past it would make the
  meters jump at the next press. An older share in the file goes to the nearest number of meters.
- The end grip's steps stop at the fewest cells that show every key and meter (`layout::reach`):
  more cells would draw the same pill, and the drag would save a length nobody saw.
- The values in `meter_values` go by the meter's place in the list. When the list changes,
  `reload_settings` starts a new generation with the settings held; a read of an older one is
  dropped (`meters::Values::publish`), and the values are told to the page under their lock, so
  the two readers' news arrive in order. `get_view` copies the settings and the values under both
  locks, and the event `meters` carries its generation: the page drops values of another
  generation than the list it drew. The new generation keeps what was read for each meter still on
  the list, at its new place (`meters::Values::renew`): a meter carried to another place does not
  go grey until the next read, and no value lands on the meter that took its old place.
- A drawing of the capsule waits while the divider is pulled and comes after Rust has the new
  place; otherwise it puts the divider back where it was. It waits too while a meter is pressed
  or carried (a carried meter stays where it was put until the reload after `move_meter`). The
  file may change meanwhile, so `move_item` and `move_meter` send the entry as it was drawn with its
  places, and Rust moves nothing when the file has another there (`config::entry_at`).
  While a meter is pressed the cursor Rust reports does not touch its label (`hover`), and a finger
  that moves closes the label only once the meters really scrolled:
  `open_meter` goes by the meter's place, and the page sends the meter's name and program with it,
  which Rust checks before it opens anything.
- `set_divider` changes the settings in memory before it writes the file, so reading the file
  back finds nothing new and does not redraw the capsule under the finger. A write that failed
  puts the memory back and only logs: the capsule shows the line where the finger left it.
- Each part keeps a key's cell or one whole meter (`layout::parts`, `parts` in `ui/index.html`), when the line
  is pulled, when the place comes from the file and when the list changes; the share in the file
  stays as it was, so a larger screen gets it back.
- A time without an offset in a meter's file is the local time of this machine (`meters::moment`); with
  `Z` or an offset it is that moment. A file is stale by its `updated:` line, else by the time it was
  written: a writer that rewrites the same numbers keeps it fresh, which is what it says. A file
  missing or without a number is no value (grey, a dash), never 0 %; a reset past due changes the
  words only, never the percent.
- A copy of the values for a drawing (`get_view`) may reach the page after a newer event `meters`,
  and a read that changed nothing sends no event to put it right: a fresh meter stayed grey. Both
  carry `readings`, the count of changes `meters::Values` keeps, which only grows, and the page keeps
  the newest it has seen.
- The words about time are made in Rust (`meters::dated`) each round, in the language of the
  settings, and come as any changed value: the page has no clock of its own for them.
- A meter's square stands at the top of its cell, not in its middle: the second outline reaches
  2.5 px past the square, and centred with the percent under it, it would run past the cell into
  the divider or the next meter. At the top edge the cell is as tall as the strip, so there it is
  centred.

## The welcome

- The flag is still called `tour_done`: a file the tour of an earlier build finished must stay
  finished. Only the start that wrote the first settings file opens the welcome window, so a file
  that build wrote with the tour unfinished shows nothing on a later start either.
- `tour_done` is written once, on `WindowEvent::Destroyed` of the welcome window, however it closed:
  `end_welcome` reads the file first and writes nothing when the flag is there, so a close does not
  leave a new settings.json.bak each time.
- WebView2 shows its own menu on a right click (Back, Refresh, Save as, Print), and F5, Ctrl+R and
  Ctrl+P reload or print the page: both pages turn them off, the settings page keeps the menu in
  text fields. A reload in the middle of the welcome card lost it.
- The welcome window closes when its page loses the focus. Tauri makes its `Focused` events from
  WebView2's own focus, not the window's; the capsule never takes the focus, so a tap on the capsule
  does not close it. The close is asked for from a thread of its own: the event comes on the thread
  of the windows. Only a visible window closes so: WebView2 loses the focus once while the window
  is still hidden, and closing then ended the welcome before anyone saw it.
- The welcome window is made hidden, with the page's colour as its background, and shows once, at
  its fitted size and place (`show_welcome`, on the first `fit_welcome`; after `WELCOME_WAIT`, when
  the programs are slow to come, at its starting size, fitted when they come):
  shown at once it flashed grey, white, black and a strip of its title before its size. Tauri
  takes the taskbar button away only when the window is made, so `show_welcome` takes it away
  again once the window shows, and brings it to the front, or its page never gets the focus whose
  loss closes it. A size given while the frameless window is hidden comes out 28 px taller once it
  shows (seen at 200 %: an empty band under the programs), so `show_welcome` gives its
  size and place again once it shows, on both ways of showing.
- The page reports its height only once the programs are found: the card is drawn first before it
  is in the page, and measured then it gave the strip of a title.
- *Add* of the card cannot run twice: while its save runs a tap on a chip must not let it go
  again (`saving`), or a second, shorter list is written over the first and its keys are lost.
- The welcome window's size comes from its page (`fit_welcome`), not a fixed one (420×360 only until the page
  reports): at a high scale or on a short work area a fixed size runs off the screen, and with few
  programs it leaves an empty band. `layout::beside` cuts the size to the work area as well as the
  place. The card is measured whole with its fractions of a pixel (`fitWelcome`), not added up from
  rounded heights: those came out a few pixels short, and the programs scrolled in a window meant
  to hold them.
- The page asks for the programs once, when the settings come: the welcome window shows its card
  at once, without a press.
- The tip is the keys' own label (`openCard`), so the window region lets it through by `card` and
  the keys keep their presses. Every drawing closes the labels: `render` brings the tip back while
  its time lasts, and hover neither closes it nor opens a key's label over it. A note or a failure
  that comes up in its place gives it back when it goes (`closeLabel`), within the tip's own six
  seconds; the end of the tip closes only its own words.

## Docking

- Only the window of an entry in `docked` is ever moved, and only its position: the move keeps the
  window's size and its place among the windows above the others (`SWP_NOZORDER`) and never makes
  it stay above the others (`HWND_TOPMOST`) or active. It is handed to the window's own thread
  (`SWP_ASYNCWINDOWPOS`): the half-second loop must not wait for a program that hangs.
- An entry goes by the program's full path and the window's class, not by the file name alone: a
  program's largest window above the others may be a popup, not the panel that was ticked. Titles
  change and are not used. A handle may be given to another window once the panel's is gone, so a
  window is `WindowId` (handle, process, when the process started), its owner and the start of
  that process are asked again before the move and before the window is kept (`win::still`), and
  the drawn part is kept for that `WindowId`.
- Path and class do not tell two windows of a program apart, so an entry keeps the window it went
  by (its watch's `WindowId`) as long as that exists, even while it is minimized or no longer
  above the others, and only then looks again (`dock::pick`); a change of the list keeps it too
  (`dock::start_again`), a restart of the panel does not. The window ticked in the settings
  window is the one kept from the start (`save_docked` with `chosen`). Looking again, one window of
  the path and class standing at the capsule's edge (by its drawn part, once known) is taken, never
  one elsewhere on the screen: a program's only window above the others may be an overlay at the
  bottom of the screen while its panel is hidden; with several none is, and
  the settings window says to pick the panel again (`several`).
- Whether a window stands at the capsule's edge goes by the part it draws once that is known
  (`dock::shown_rect`): a transparent frame may reach the edge while its pill does not, and the
  other way round.
- Positions are what shows of a window (`DWMWA_EXTENDED_FRAME_BOUNDS`), not its rectangle: a
  window that can be resized has invisible borders, and flush by its rectangle would leave a gap.
  `TopWindow::border` turns one into the other for the move.
- Everything is in physical px. The process is per-monitor DPI aware (v2): tao sets it when the
  event loop starts, so `GetWindowRect`, `SetWindowPos` and UI Automation's rectangles of any
  program's window are in the same pixels as the work area Tauri reports.
- A docked window that can be seen through is placed by the part it draws, not by its rectangle:
  a panel drawn in a web page is a transparent window many times larger than its pill (one
  measured 576 x 1040 around a 112 x 548 pill), and by its rectangle it never fits under the
  capsule. The part is the union of the window's elements on the screen without those over half the
  window's area (the page, its document, groups that span it); the leaves alone (icons, texts) miss
  the pill's padding, so groups count. An opaque window shows all of itself whatever its elements
  say, so only a layered window, one without a redirection bitmap, or one with a window region is
  shrunk (the two measured panels: one layered, one with a region).
- The question goes to the program through UI Automation, which turns on a Chromium program's
  accessibility and waits for a slow one, so it is asked in `watch_docked`, with one-second
  timeouts and not at all without them, again when the window's size changes and otherwise at most
  every five seconds per window; the half-second
  loop only reads the answer and waits for it before it moves a new window. The window is looked
  at again after the answer: one that moved meanwhile gave rectangles of two places, and its answer
  is dropped. Without UI Automation the thread still runs and takes every window whole.
- A move shows a moment after it is asked for. Still where it stood is a move on its way, waited
  for two seconds, never a return; a move that never showed, or that Windows refused (a window of
  a program with rights above the panel's), counts as refused, and the third refusal in a row lets
  the window be with its own note. Once the move took, a window found anywhere but where it was put
  was moved by something else: its program heading back to its own saved place, which it need not
  reach exactly (seen: a panel moved itself back every second and never landed on its
  old place, so "back where it stood" never fired and the two tossed it ten times in thirty
  seconds), or a hand. The second such move within ten seconds lets it be, with the note that it
  returns to its own place. A window left behind by the capsule's move still stands where it was
  put and is only put at its new place. A window let be stays so until the `docked` list changes or the panel
  restarts; a theme or a carry does not start it again, a new window of the program, once the
  one let be is gone, does (`dock::watch_for`), because such a program puts its new window at its
  own saved place once.
- The column hangs from the pill as it is drawn (`layout::pill_on_screen` of the capsule window as
  it is on the screen), not from the window: the window has room for the shadow at each end of the
  pill, and a gap measured from the window would be that much wider. So the column steps down by
  one sign and the end grip while the signs show in the foot, and moves when the program in front brings other
  keys.
- What does not fit under the capsule goes above it, never on top of a panel already placed; a
  window that fits on neither side is left where it is, which may be over a docked one: the log says
  so once (`Watch::told`), and moving the capsule makes room. The transparent rest of a window may
  run past the work area.
- Nothing is looked at with an empty `docked`; the desktop is looked over for windows above the
  others only while the capsule is shown and open.

## Path repair

- Only the full path of an exe on a drive is repaired (`repair::repairable`): a `shell:AppsFolder\…`
  target and an app id keep their id wherever the files go, a network path is never gone to, and a
  bare file name is found by Windows itself.
- The write goes through `change_settings` and touches only the target of an `open` item, the
  `program` of a meter and the `path` of a docked entry that name the old path (`repair::with_paths`);
  the name, the icon, the hint, `app`, `exe`, `class` and fields the panel does not know stay. With
  nothing to put right it hands the file back as it was.
- Only a find that is plainly the same installation is written by itself: one place, in a sibling
  folder of the same kind with a higher version (`repair::same_install`). A single find anywhere
  else may be another program with the same exe name, so it is only offered on the card (*Use it*),
  and the key is grey meanwhile. A version lower than the missing one is never taken; two of the
  newest version, or one equal to the old (`1.2` beside `1.2.0`), count as several.
- Several places are never guessed between, whatever looks newest: a program that keeps a starter of
  the same file name beside its version folders is found twice (the starter from the Start menu,
  the exe from the version folder) and stays as it is, its key grey.
- A path on a drive that is not there or not ready (a card, a stick, a network drive) is neither
  repaired nor grey: it is looked at again at the next look.
- `check_paths` runs after every changed file was applied, and its own write is such a change: the
  second look finds nothing gone and writes nothing, so it ends there. A path still gone looks again
  at each change of the file, with the Start menu listed at most once a minute; the log says it once,
  while it stays in `lost`.
- `lost` holds `repair::key`s (backslashes, lower case), and a key goes grey only when its own target
  is one of them; the list is replaced whole by each look.
- A tap on a grey key runs nothing unless its exe is back. It names the item by its place among those
  the panel uses, the settings window lists every item of the file, broken ones too: `get_settings`
  turns one place into the other, and gives `card` only once. A card with edits not saved is never
  replaced by it.
- A card is open while the panel may write the file (a repair, a carry). Its save must not hand back
  the item as it was when the card opened: it reads the settings again and changes only the fields
  the card changed.
- The lines about repaired paths live in memory only. One shows from the first time a settings
  window asks for it until a new settings window is made; one that came while no window was open
  waits for the next.

## The checks

- The page tests run in headless Chrome with a stand-in for Tauri. They prove nothing about
  WebView2 or Win32: use `scripts/preview.sh` and look.
