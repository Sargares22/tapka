# Showing a running task in the capsule

Tapka has a section of tasks between the meters and the plus. It is there only while a task is, and
goes when the last task goes. An agent or a script adds a task by writing a small file and removes it
by deleting the file. No settings, no restart, nothing for the person to do. Use it for work the
person waits for: a download, a render, a long run of an agent. Do not use a meter for that: meters
are for what is always there.

## The file

`%USERPROFILE%\.tapka\tasks\<id>.task`, one file per task. Create the folder if it is missing.
`<id>` is a name of your choice; two writers with one id share a task. Text, UTF-8 or UTF-16, lines
of `key: value`, all optional:

```text
name: Camera
percent: 46.5
status: clip 3 of 35, 3.6 of 4.0 GB
fresh: 120
rate: 0.05
started: 2026-10-31T15:04:00+07:00
```

| Key | Meaning |
|---|---|
| `name` | What the capsule shows, about 9 letters fit; the label shows up to 40. Without it, the id |
| `percent` | 0 to 100, one decimal counts. Without it, the task shows a running bar and its time of work. Do not make a percent up |
| `status` | One line, up to 160 characters, in the label on a press or a hover |
| `fresh` | How many seconds the task may be quiet before it counts as stale. Default 120 |
| `rate` | How many percent a second the work adds, if you know it. Optional: without it Tapka works the speed out from your writes |
| `started` | When the work began, ISO 8601, for the time of work. Without it, when the file appeared |

Tapka reads the folder twice a second, so a change shows within a second. Only `*.task` files count.
The section has up to four cells; with more tasks the last cell says how many more. Tasks with a
percent come first, quiet ones last, by file name within that.
Tapka never writes or deletes in this folder.

## A bar between writes

You may write the percent rarely, every ten seconds say: the bar does not stand and then jump. Tapka
keeps the last few writes that changed the percent, timed by the file's own time, and moves the bar
on at their speed, or at `rate` if the file has it. The number in the cell is always the one you
wrote. The bar never goes back, goes no further than about one usual step past your last percent,
and stands once you are quieter than that. So write at an even pace, and write the percent with
one decimal on a long job, so that each write moves it. A lower percent than the last means the
work began anew. With reduced motion on in Windows, the bar moves only on your writes.

## The life of a task

- **Add or update**: write the file. Write to `<id>.task.tmp` and move it over the file if you can.
  A file caught half written keeps what it said before, so a plain write works too.
- **Remove**: delete the file when the work ends, also when it fails.
- **Stale**: every write counts as a sign of life, the file's own time is what counts. Past `fresh`
  the task turns grey with an orange bar, and its label says how long it has been quiet. Write only
  when something really moved, then a hung job goes grey by itself. A job with long quiet steps sets
  a larger `fresh`.
- **Gone**: an hour past `fresh` the task leaves the section, even if its file is still there. The
  file stays; the next write brings the task back. A leftover file costs nothing, but delete it.

## Writing it

`scripts/task.ps1` does it in one line:

```powershell
powershell -NoProfile -File task.ps1 -Id render -Name Render -Percent 12 -Status "frame 120 of 1000"
powershell -NoProfile -File task.ps1 -Id render -Done
```

From any other shell:

```bash
mkdir -p ~/.tapka/tasks
printf 'name: Render\npercent: 12\nstatus: frame 120 of 1000\n' > ~/.tapka/tasks/render.task
rm ~/.tapka/tasks/render.task
```

Not `%APPDATA%`: an agent inside a packaged app writes there to a private copy no other program sees.

## Checking it

Run `scripts/capsule.ps1`: while the task is there it lists a button with its name, percent and status.
