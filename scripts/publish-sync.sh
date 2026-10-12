#!/bin/sh
# Brings the public copy of the repository up to the current commit: only the files that are
# published, plus the one-line CLAUDE.md that points Claude Code at AGENTS.md. Commits there;
# pushing is a separate step. Usage: sh scripts/publish-sync.sh <public copy> "<commit message>"
set -e
cd "$(dirname "$0")/.."
dest="$1"; message="$2"
[ -d "$dest/.git" ] && [ -n "$message" ] || { echo 'usage: sh scripts/publish-sync.sh <public copy> "<commit message>"'; exit 1; }
# A file left lying in the public copy would be published with the rest
[ -z "$(cd "$dest" && git status --porcelain)" ] || { echo "the public copy has changes or stray files of its own; clean it first"; exit 1; }
(cd "$dest" && git rm -rq .)
git archive HEAD -- .gitignore AGENTS.md docs .agents .claude/skills Cargo.toml Cargo.lock build.rs tauri.conf.json capabilities installer src ui glyphs icons assets scripts/check.sh scripts/checklist.sh scripts/preview.sh scripts/shot.mjs scripts/check-page.mjs scripts/make-icons.py scripts/make-release.sh scripts/publish-sync.sh scripts/windows-of.ps1 README.md README.ru.md LICENSE CHANGELOG.md | tar -x -C "$dest"
echo "@AGENTS.md" > "$dest/CLAUDE.md"
cd "$dest"
git add -A
git commit -q -m "$message"
git log --oneline | head -1
