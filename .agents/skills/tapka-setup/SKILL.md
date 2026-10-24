---
name: tapka-setup
description: >
  Install Tapka (the touch panel for Windows 11, github.com/Sargares22/tapka) for a person, or add
  their programs to it: keys that open programs, meters that show a program's progress or numbers
  from a file, and the scripts (recipes) that write such files. Use when someone says "install
  Tapka", "поставь Тапку", "add my programs to Tapka", "добавь в панель такие-то программы",
  "make a meter for X", or pastes the Tapka install phrase. Do NOT use for changing Tapka's own
  code; that is AGENTS.md.
---

# Tapka setup

Two modes. Decide which from what the person said and from the machine:

- **A. First install** — Tapka is not installed, or the person asks to install it.
- **B. Add programs** — Tapka is installed and the person names programs to add, or asks what
  could be added.

Talk to the person in their language, in plain words. The person does not want to approve every
step: collect everything first, then ask **one** question with a short list, then do it all.

## Mode A: first install

| Step | Check, then do only what is missing | Read |
|---|---|---|
| 1. Machine | Windows 11, ARM64 or x64, is Tapka there, which version | `references/install.md` |
| 2. Install | the person runs the release installer; Tapka starts | `references/install.md` |
| 3. Candidates | Start menu, running programs, taskbar progress, panels at the screen edge | `references/find.md` |
| 4. One question | a short list: keys …, meters …; nothing else | below |
| 5. Apply | backup, then add entries to `settings.json` | `references/settings.md` |
| 6. Verify | log, settings file, capsule readout | `references/verify.md` |
| 7. Summary | at most 6 lines | below |

## Mode B: add programs

For each program the person named, find it (`references/find.md`) and choose one way, in this
order (`references/meters.md`):

1. **Key** — an `open` item for the program. Always possible.
2. **Meter from the program** — only if the program shows progress to Windows (a bar on its
   taskbar button, a progress bar in its window, or a percent at the start of its title).
3. **AI usage meter** — for Claude Code, Codex (ChatGPT) and Antigravity Tapka reads the usage
   limits itself: add a `limit` meter, no recipe. If its status says it is not connected, the
   person presses *Connect* in Settings → Indicators.
4. **Meter from a file** — the program keeps the number on the disk or can be asked for it: write
   a recipe script into the person's own folder, start it at sign-in, add a `file` meter.
5. **Docking** — last resort, only for a panel window at the screen edge that has no number to
   read, and only after the person says yes to it by name.

Then the same one question, apply, verify, summary.

## The one question

One message: what you found and what you will do, as a short list, ending in one question.

```text
Found and suggest:
- Keys: Krita, Handy (dictation, ctrl+space, glows while recording)
- Meters: Blender (shows render progress on its taskbar button)
- AI usage: Claude and ChatGPT (built into Tapka, no script)
- Meter from a file: a backup tool's progress it keeps in a log — I write a script into Documents\Tapka recipes, it starts at sign-in
Do all of it?
```

A "yes" covers the whole list. A "no" to one line drops that line only. Ask a second question only
if the answer changes what you can do (for example, two programs of the same name).

## Summary

At most 6 lines: what was added, where the recipe scripts live, where the backup is, that saying
the same phrase again checks and repairs.

## Rules that protect the person

- **Back up before the first write**: copy `settings.json` to
  `settings.json.agent-<yyyyMMdd-HHmmss>.bak` beside it. Not `settings.json.bak`: Tapka rewrites
  that one at its next own save. Why: the person's keys are their work, and the settings window
  cannot undo a file edit.
- **Add, never replace.** Append to `items`, `meters`, `docked`. Keep every field you do not
  know as it is. Do not reorder, rename or remove the person's entries unless they asked.
- **Nothing about a program goes into Tapka's code or repository.** Paths, program names and
  recipe scripts live in the person's settings file and their own folders.
- **Never bypass Windows protection.** The installer is not signed: the person presses "More
  info → Run anyway" themselves. Do not remove the download mark (`Unblock-File`), do not add
  Defender exclusions, do not run anything as administrator.
- **A recipe only reads.** It reads what the program already keeps or shows, writes only its own
  meter files, sends nothing over the network, and runs as the person, hidden, at sign-in. Show
  the person the script's path; they can read it.
- **Docking only with a yes by name**, and say that some panels keep moving back to their own
  place, and then Tapka lets them be.
- **Do not start, stop or replace a running Tapka** to test something. Tapka reads the settings
  file again within a second; that is the test.
- **Done means seen**: the log has no "skipped" line for what you added, and the capsule readout
  shows it, or the person confirms it (`references/verify.md`). An installer that finished is not
  done.

## Common mistakes

| Mistake | Fix |
|---|---|
| Asking about each program in turn | One list, one question |
| Writing to `%APPDATA%` from a packaged agent app and finding Tapka unchanged | Check where your writes go (`references/settings.md`, "Which folder") |
| A `program` meter for a program that never shows progress | Empty grey meter forever: choose a key, or a file meter |
| A meter's file path with forward slashes or `~` | Full Windows path, backslashes doubled in JSON |
| Declaring success while the capsule is hidden | A keyboard attached with "only without a keyboard" hides it; say so and verify by the log and file |
| Reading the settings file with plain `Get-Content` in Windows PowerShell 5.1 | It reads UTF-8 without BOM as ANSI and garbles non-English names; use `-Encoding UTF8` or `[IO.File]::ReadAllText` |
| Saving the settings file as UTF-16 or ANSI | UTF-8 (Tapka also accepts a BOM) |
| A recipe script with non-English text saved without a BOM | Windows PowerShell 5.1 misreads it; save scripts as UTF-8 with BOM |
