#!/bin/sh
# Every automated check, stopping at the first failure: Rust tests, page tests in headless
# Chrome, and that the notes for agents still point at things that exist.
set -e
cd "$(dirname "$0")/.."

cargo test
node --test scripts/check-page.mjs

# Every file AGENTS.md names is there
for path in $(grep -o '`[A-Za-z0-9_./-]*\.[a-z]*`' AGENTS.md | tr -d '`' | sort -u); do
  [ -e "$path" ] || { echo "AGENTS.md names $path, which does not exist"; exit 1; }
done
# Every skill has its pointer for Claude Code, and the pointer names the skill
for skill in .agents/skills/*/; do
  name=$(basename "$skill")
  grep -q ".agents/skills/$name/SKILL.md" ".claude/skills/$name/SKILL.md" 2>/dev/null \
    || { echo "no pointer to the skill $name in .claude/skills"; exit 1; }
done
echo "all checks passed"
