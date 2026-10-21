# Verify without the person's eyes

Three sources, in this order. Wait two seconds after writing the settings file.

## 1. The log

`panel.log` beside `settings.json`. Read the lines written after your change:

| Line | Means |
|---|---|
| `settings: N items` | the file was read; N is the number of keys |
| `settings: item K skipped: …` / `meter K skipped: …` / `docked K skipped: …` | entry K of that list is broken; the reason follows |
| `settings: invalid file, items unchanged: …` | the JSON does not parse; restore your backup at once |
| `path: <old> -> <new>` | Tapka repaired a path itself; fine |
| `capsule: hidden, a keyboard is attached` | the capsule is not on the screen (see below) |
| `dock: … goes back to its own place; left alone …` | a docked panel fights back; tell the person, offer to remove the entry |

No new line at all after a change means one of two things: Tapka read the file and nothing it
draws changed, because every entry you added was skipped (a reload that changes nothing logs
nothing, not even the skipped lines), or Tapka did not see the file at all: check the folder
(`settings.md`, "Which folder"). Check your entries against `settings.md` and the capsule
readout below.

## 2. The files

- A `file` meter: the file exists, its first line is one or two numbers, and it changed within the
  recipe's interval.
- A `program` meter: the target exists.

## 3. The capsule

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File <skill folder>\scripts\capsule.ps1
```

It prints every key and meter of the capsule as UI Automation names it: a key by its name, a
meter by its letter and percent, like `button 'C 64%'`. Compare with what you added.

"no capsule buttons found" means the capsule is not on the screen: folded, hidden from the tray,
or a keyboard is attached while "only without a keyboard" is on. Then the log and the files are
the proof; tell the person where to look once the keyboard is off.

## 4. The person

Ask once, at the end, only if 3 could not see it: "Under the keys, do you see N meters, the first
at X %?"
