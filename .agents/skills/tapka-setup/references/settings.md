# The settings file

`settings.json` in Tapka's data folder: `%APPDATA%\sargares22.tapka\`. A Tapka started with the
environment variable `TAPKA_DATA_DIR` uses that folder instead. Tapka reads the file again within
a second of a change; there is nothing to restart.

## Which folder

An agent that runs inside a packaged app (MSIX) may see `%APPDATA%` redirected into its own
package folder, for example `%LOCALAPPDATA%\Packages\<package>\LocalCache\Roaming`. Tapka, started
by Windows, reads the real folder. Check before writing: the real `settings.json` was written by
Tapka at its last start, and `panel.log` beside it has today's lines. If your `%APPDATA%` shows no
such file while Tapka runs, use the real path through `\\localhost\C$\Users\<user>\AppData\Roaming\sargares22.tapka`.

## Write

1. Read the file as UTF-8. Tapka writes it without a BOM, and Windows PowerShell 5.1 reads such a
   file as ANSI unless told: `Get-Content settings.json -Raw` turns Cyrillic names into garbage,
   and writing that back ruins them. Use `[IO.File]::ReadAllText($path)` or
   `Get-Content -Raw -Encoding UTF8`. With `ConvertTo-Json` give `-Depth 8`: the default depth
   flattens `lit` rules. Parse it as JSON. If it does not parse, stop and tell the person: Tapka
   keeps its keys unchanged while the file is invalid, and the tray tooltip says why.
2. Back up: copy it to `settings.json.agent-<yyyyMMdd-HHmmss>.bak` in the same folder.
3. Append the new entries. Keep everything else as it was.
4. Write to a temporary file in the same folder, then move it over `settings.json` (one step, so
   Tapka never reads half a file). UTF-8 without BOM; the indent does not matter.

## Entries

Keys, in `items`:

```json
{ "name": "Krita", "action": "open", "target": "C:\\Program Files\\Krita (x64)\\bin\\krita.exe" }
{ "name": "Notes", "action": "open", "target": "shell:AppsFolder\\Microsoft.WindowsNotepad_8wekyb3d8bbwe!App" }
{ "name": "Dictation", "icon": "mic", "action": "hotkey", "keys": "ctrl+space",
  "lit": { "program": "handy.exe", "window": "Recording" } }
```

- A key that opens a program needs no `icon`: Tapka takes the program's own.
- `only_in`: show the key only while that program's window is active (`"only_in": "krita.exe"`).
- The full list of fields is in the README, section *The settings file*.

Meters, in `meters`:

```json
{ "name": "Render", "program": "C:\\Tools\\render.exe" }
{ "name": "Render", "program": "shell:AppsFolder\\Vendor.Render_abc123!App" }
{ "name": "Claude", "file": "C:\\Users\\me\\Documents\\Tapka recipes\\limits\\claude.txt" }
```

- `program` is the full path of the exe or the `shell:AppsFolder\…` address, exactly as an `open`
  key would have it. A key with the same target then shows the meter as a ring on its rim.
- `app` (optional): the id the program gives its windows, when its taskbar button is named by it
  rather than by the exe.
- `file`: the full path of the meter file. The file may not exist yet; the meter stays empty and
  grey until it does.

Docked panels, in `docked` (only with the person's yes by name):

```json
{ "exe": "panel.exe", "path": "C:\\Tools\\panel.exe", "class": "PanelWindow" }
```

`exe` in lower case, `path` and `class` from the `edge` list of `candidates.ps1`.

Leave `divider`, `top`, `edge`, `scale`, `theme`, `accent`, `lang`, `folded` and the rest alone.
