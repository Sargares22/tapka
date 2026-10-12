# Working on the code

## Commands

Run them from the repository root in a POSIX shell (on Windows: Git Bash).

- `cargo test` — the Rust tests alone; quick, use it while working.
- `node --test scripts/check-page.mjs` — the page tests alone, in headless Chrome.
- `sh scripts/check.sh` — every automated check, then the checklist. Run it before you say you
  are done.
- `sh scripts/checklist.sh` — the checklist alone. What a script can check fails the run. The
  rest it asks, only for what the change touched: answer each question in your final reply,
  one line each, and say plainly what you could not do or see.
- `node scripts/shot.mjs` — pictures of both pages in `target/shots`, no Windows calls.
- `sh scripts/preview.sh` — build and start on a throwaway data folder (a first run). It is a
  release build, heavy on memory: once, at the end, not after every small change.
- `powershell -File scripts/windows-of.ps1` — the running panel's windows, their styles, and
  whether the taskbar shows a button for it (should be none).

`scripts/make-release.sh` and `scripts/publish-sync.sh` build and publish a release. The owner
runs them on their machine, with the project's update key. Do not run them.

## Recipes

Step-by-step recipes are skills in `.agents/skills`, one folder each with its SKILL.md
(Claude Code reads the pointers in `.claude/skills`): adding a ready-made Windows action,
adding a built-in icon, running the checks, starting a preview.

## What holds everywhere

- No paths, package names or names of other products in the code. Users add those themselves.
- Code and comments in English. Interface texts in both `ru` and `en`.
- A comment says why, where the code would otherwise be "fixed" back into a bug. What a line
  does is not a comment.
- When a change adds a command, an event, a file or a settings field, or changes what leads to
  what, `docs/map.md` changes with it. When it finds a new way to break something,
  `docs/traps.md` gets a line.
