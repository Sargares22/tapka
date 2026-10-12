#!/bin/sh
# What must be filled in before a change is done. The first part is checked here and fails the
# run; the second part cannot be checked by a script, so it is asked, and only for what the
# change touched. Run by scripts/check.sh; on its own: sh scripts/checklist.sh
cd "$(dirname "$0")/.."
bad=0
fail() { echo "  MISSING  $1"; bad=1; }

echo "checklist: what a script can check"

# Every source file and page has its line in the table of the map
for f in src/*.rs ui/*.html; do
  grep '^|' docs/map.md | grep -Fq "\`$f\`" || fail "$f is not in the table of docs/map.md"
done

# Every built-in icon is listed with what it is for
for f in glyphs/*.svg; do
  grep -F "| $(basename "$f") |" glyphs/NOTICE.md | grep -Eq '\|[^|]+\|[^|]*[^| ][^|]*\|' \
    || fail "$(basename "$f") is not in glyphs/NOTICE.md with what it is for"
done

# Every command a page can call and every event Rust sends is named in the map
for name in $(awk '/generate_handler!\[/{f=1} f{print} /\]\)/{if(f)exit}' src/main.rs | tr -d ' \n' \
    | sed 's/.*generate_handler!\[//;s/\].*//' | tr ',' ' '); do
  grep -Fq "\`$name\`" docs/map.md || fail "the command $name is not in docs/map.md"
done
for name in $(grep -ohE '\.emit(_to)?\([^"]*"[a-z_]+"' src/*.rs | grep -oE '"[a-z_]+"' | tr -d '"' | sort -u); do
  grep -Fq "\`$name\`" docs/map.md || fail "the event $name is not in docs/map.md"
done

# The settings window has every text in both languages, and none of them empty
node -e '
  const text = require("fs").readFileSync("ui/settings.html", "utf8").replace(/\r/g, "");
  const from = text.indexOf("const T = {"), to = text.indexOf("\n  };", from);
  if (from < 0 || to < 0) throw new Error("the dictionary T was not found in ui/settings.html");
  const T = new Function("return " + text.slice(from + 10, to + 4))();
  const own = o => Object.keys(o || {});
  if (!own(T.ru).length || !own(T.en).length) throw new Error("the dictionary T has no ru or no en");
  const lost = [];
  for (const [a, b] of [["ru", "en"], ["en", "ru"]])
    for (const k of own(T[a])) {
      if (!Object.prototype.hasOwnProperty.call(T[b], k)) lost.push(k + " has no " + b);
      else if (typeof T[a][k] === "string" && !T[a][k].trim()) lost.push(k + " is empty in " + a);
    }
  if (lost.length) { console.log(lost.join("\n")); process.exit(1); }
' > "${TMPDIR:-/tmp}/tapka-texts.$$" 2>&1 || {
  fail "the texts of ui/settings.html:"
  sed 's/^/           /' "${TMPDIR:-/tmp}/tapka-texts.$$" | head -20
}
rm -f "${TMPDIR:-/tmp}/tapka-texts.$$"

# One version everywhere, and the changelog knows it
version=$(grep -m1 '^version' Cargo.toml | sed 's/^version *= *"\(.*\)".*/\1/')
conf=$(node -p 'require("./tauri.conf.json").version' 2>/dev/null)
[ "$conf" = "$version" ] || fail "tauri.conf.json is at $conf, Cargo.toml at $version"
grep -Eq "^## $(echo "$version" | sed 's/\./\\./g')( |\$)" CHANGELOG.md || fail "CHANGELOG.md has no section for $version"

# What is published never says what was left unchecked
if grep -n -i -E "not (been |yet )?(verified|tested)|untested|unverified|not checked by hand|checked only by|не проверено|не проверял|не проверен[аы]|проверено только" CHANGELOG.md README.md README.ru.md; then
  fail "a published text says what was not checked: that belongs in the chat, not in a release"
fi
# ...and names nobody's machine: no home folder of a real user
if grep -rnE '[A-Za-z]:[\\/]+Users[\\/]+|/home/[a-z]' src ui scripts docs AGENTS.md README.md README.ru.md CHANGELOG.md 2>/dev/null \
    | grep -vE 'Users[\\/]+(Public|Default|example|me|<)' | grep -v '^scripts/checklist.sh:'; then
  fail "a home folder on somebody's machine in what is published"
fi

[ "$bad" = 0 ] && echo "  all filled in"

# What changed: the working copy against the last commit, or the last commit if it is clean
changed=$( { git diff --name-only HEAD 2>/dev/null; git ls-files --others --exclude-standard; } | sort -u)
[ -n "$changed" ] || changed=$(git diff-tree --root --no-commit-id --name-only -r HEAD 2>/dev/null)
touched() { echo "$changed" | grep -q -E "$1"; }

echo "checklist: answer each of these in your final reply, one line each"
asked=0
ask() { echo "  [ ] $1"; asked=1; }
if touched '^(src|ui)/'; then
  if touched '^docs/'; then
    ask "Code and docs/ both changed. Do docs/map.md and docs/traps.md now say what the code does, not what it did?"
  else
    ask "Code changed and docs/ did not. Name the reason: nothing in docs/map.md or docs/traps.md is about what you changed, or say what you will add."
  fi
  ask "What did you see working with your own eyes, and what only in tests? What could you not check here (finger, pen, the real panel)? Leave that to the owner by name."
fi
touched '^src/(main|win|grab|layout)\.rs' && ask "A window, a region or the handle changed. Which edges and sizes were looked at on the real panel, and which were not?"
touched '^ui/' && ask "A page changed. Which pictures did you take and look at (node scripts/shot.mjs), in which theme?"
if touched '^src/config\.rs'; then
  ask "The settings file. Which test shows that an old file still opens and that fields the panel does not know survive a write?"
  ask "A new ready action, set or settings field: where is it in README.md and in README.ru.md?"
fi
touched '^ui/index\.html|^src/main\.rs' && ask "A new text the capsule shows (a label, a failure): where are its ru and its en?"
touched '^glyphs/' && ask "A new icon: drawn for the project, no one else's mark. On which picture did you look at it on a key?"
touched '^(Cargo\.toml|tauri\.conf\.json)$' && ask "The version moved. Does CHANGELOG.md say what changes for the user, and nothing about how it was made or checked?"
[ "$asked" = 0 ] && echo "  nothing to ask for this change"

exit $bad
