#!/bin/sh
# Stops the running panel, runs every test, builds the release exe and starts it again.
set -e
cd "$(dirname "$0")/.."
taskkill //IM tapka.exe //F >/dev/null 2>&1 || true
cargo test 2>&1 | grep -E "^test result|FAILED|panicked|error" || true
node --test scripts/check-page.mjs 2>&1 | grep -E "^ℹ (pass|fail)|^✖" || true
cargo build --release 2>&1 | grep -E "^(warning|error)|Finished" || true
(./target/release/tapka.exe &) ; sleep 4
tail -n 4 "$APPDATA/sargares22.tapka/panel.log"
