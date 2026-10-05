---
name: tapka-check
description: Run every automated check of the Tapka repository (Rust tests, headless page tests, docs consistency). Use before finishing any change to this repository.
---

Run `sh scripts/check.sh` from the repository root. It stops at the first failure.

It covers: `cargo test`, `node --test scripts/check-page.mjs` (needs Google Chrome), and that
the files named in `AGENTS.md` and the skill folders exist.

It does not cover real windows, touch, or key sending. For those use the `tapka-preview` skill.
`cargo test live -- --ignored --nocapture --test-threads=1` presses real keys on the desktop;
run it only when asked.
