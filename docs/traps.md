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
- A step that failed reaches the page as the event `failed`. Rust drops a late one itself, if
  the list of items changed after the tap (`items_age`).

## The settings file and key sets

- What the running panel changes in the settings file goes through `change_settings` in
  `src/main.rs`.
- Fields of the settings file the panel does not know must survive every write.
- A key set from a file (`parse_set`, `set_items` in `src/config.rs`) is untrusted: it is read
  strictly, never replaces the settings file, and no icon is looked up for a target on the
  network (`shellicon::remote`). Opening the file only shows its card; the items are written
  when *Add* is pressed, without those already in the list and those whose target is not on
  this machine.

## The checks

- The page tests run in headless Chrome with a stand-in for Tauri. They prove nothing about
  WebView2 or Win32: use `scripts/preview.sh` and look.
