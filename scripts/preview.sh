#!/bin/sh
# Builds the release exe and starts it on an empty temporary data folder: a first run that
# leaves your own settings alone. Usage: sh scripts/preview.sh [--replace]
set -e
cd "$(dirname "$0")/.."

if tasklist 2>/dev/null | grep -qi '^tapka.exe'; then
  if [ "$1" = "--replace" ]; then
    taskkill //IM tapka.exe //F >/dev/null
  else
    echo "Tapka is already running and only one copy can run. Quit it from the tray icon, or run: sh scripts/preview.sh --replace"
    exit 1
  fi
fi
cargo build --release
data=$(mktemp -d)
TAPKA_DATA_DIR="$(cygpath -w "$data" 2>/dev/null || echo "$data")" ./target/release/tapka.exe >/dev/null 2>&1 &
sleep 4
echo "Tapka is running on a clean data folder: $data"
echo "--- panel.log"
cat "$data/panel.log"
