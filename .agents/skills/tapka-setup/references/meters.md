# Choosing the way for a program, and writing a recipe

## Which way

| What the program does | Way | Check |
|---|---|---|
| Only needs to be opened | key | the exe or `shell:AppsFolder` target exists |
| Shows progress on its taskbar button, in a progress bar of its window, or as a percent at the start of its title | `program` meter (and its key, which gets the ring) | `candidates.ps1` lists it under `progress` while it works, or its title starts with a number and `%` |
| Is Claude Code, Codex (ChatGPT) or Antigravity, and the person wants its usage limits | `limit` meter: `{ "name": "Claude", "limit": "claude" }`, values `claude`, `chatgpt`, `antigravity-gemini`, `antigravity-claude`; no recipe | the program is installed and signed in (Settings → Indicators shows *Connected*) |
| Keeps a number on the disk (a JSON, an INI, a log, a database the person owns) or prints it from a command | `file` meter fed by a recipe | you can read the number yourself with a short script |
| A panel at the screen edge with nothing to read | docking, only with a yes by name | it stands in the `edge` list |
| None of these | key only; say plainly that there is no number to show | — |

Look for an existing recipe first: a recipes folder the person already has, one folder per
program.

## The meter file

First line: one or two numbers from 0 to 100, separated by a space. The first fills the meter,
the second draws a thin ring around it (for example a weekly limit around a daily one). Then up
to five lines of hint, 80 characters each. A line `-` or anything that is not a number leaves the
meter empty and grey. UTF-8 or UTF-16.

## A recipe

A PowerShell script in the person's own folder, by default
`%USERPROFILE%\Documents\Tapka recipes\<program>\`. Its first lines say what it reads, what it
writes, and the date and versions you checked it with. Model:

```powershell
# Recipe: <what> from <program> into Tapka meter files. Not part of Tapka.
# Reads <path>; writes <name>.txt beside this script. Checked <date>, <program> <version>.
$out = Split-Path -Parent $MyInvocation.MyCommand.Path
while ($true) {
  $lines = @('-')
  try {
    # read the number(s) here; $lines = @("$a $b", "hint line")
  } catch {}
  $text = $lines -join "`r`n"
  $path = Join-Path $out '<name>.txt'
  $old = if (Test-Path $path) { (Get-Content $path -Raw -Encoding UTF8).Trim() } else { '' }
  if ($old -ne $text) { [IO.File]::WriteAllText($path, $text, (New-Object Text.UTF8Encoding $false)) }
  if ($args -contains '-once') { break }
  Start-Sleep -Seconds 30
}
```

Rules:

- Reads only; writes only its own `.txt` files; no network; no admin rights.
- Writes only when the text changes (Tapka redraws on every change).
- On any error writes `-`, so the meter goes grey instead of showing an old number.
- Run it once with `-once` and read the file before you add the meter.

Start it at sign-in with a shortcut in the Startup folder (`shell:startup`):

```powershell
$lnk = Join-Path ([Environment]::GetFolderPath('Startup')) 'Tapka <program>.lnk'
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = "$env:WINDIR\System32\conhost.exe"
$s.Arguments = "--headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"<script path>`""
$s.Save()
```

Then start it now the same way (`Start-Process` with the same target and arguments), so the
person does not wait for the next sign-in.

## Sharing a recipe

Only when the person asks, and only where they say. Never into Tapka's repository: it holds no
code or names of other programs.
