# Finding programs (mode A step 3, mode B)

Run the read-only helper beside this skill:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File <skill folder>\scripts\candidates.ps1 > candidates.json
```

It prints JSON with five lists:

| List | What it is | Becomes |
|---|---|---|
| `start` | Start menu shortcuts to an exe | a key (`open`, `target` = `exe`) |
| `store` | Store apps, `target` = `shell:AppsFolder\…` | a key (`open` with that target) |
| `running` | running programs with a window | a key; ask nothing, just offer |
| `progress` | taskbar buttons that show a progress bar right now | a `program` meter for that program |
| `edge` | windows that stay above the others at a screen edge | look for a number to read (`meters.md`); docking only as the last resort |

## Mode A: what to offer

Do not offer the whole Start menu. Offer at most 6 keys: programs in `running`, and programs the
person pins or uses for drawing, notes, dictation or chats if they are in `start` or `store`.
Skip uninstallers, helpers, updaters, consoles and everything under `C:\Windows`. Skip programs
already in the settings file (same `target`).

## Mode B: a named program

Match the name the person said against `start[].name`, `store[].name`, `running[].name` and the
exe file names, ignoring case. One match: use it. Several (two installs, or a program and its
helper): show them in the one question. None: ask where it is, once.

A `progress` button is named by an AutomationId like `Appid: C:\Tools\render.exe` (a desktop
program) or `Appid: <Package>!<App>` (a Store app). An empty `progress` list only means no program
shows progress at this moment; ask the person to start a long task (a copy, a render, a download)
if you need to see it.
