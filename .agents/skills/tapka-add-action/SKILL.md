---
name: tapka-add-action
description: Add a ready-made Windows action (a key the user can add from "Add → Windows action") to Tapka. Use when asked to add a new built-in action or shortcut preset.
---

Only actions that work on any Windows 11 with nothing installed belong here.

1. `src/config.rs`: add one `Preset` to `PRESETS` and raise the array length. Fields: `id`,
   `icon` (a built-in name from `src/glyphs.rs`), `action` (`("hotkey", "win+x")` or
   `("open", "ms-settings:")`), and name plus hint in `ru` and `en`. Use the names Windows
   itself uses in each language.
2. The keys must be ones `src/keys.rs` knows: modifiers, letters, digits, F1–F24, navigation keys.
3. To put it into a new user's first capsule, add the id to `DEFAULT_ITEMS`. Usually: do not.
4. Add it to the list of ready-made actions in `README.md` and `README.ru.md`.
5. Run `sh scripts/check.sh`. The test `every_ready_action_makes_a_valid_item_in_both_languages`
   checks the new line.
6. Try it for real with the `tapka-preview` skill: Settings → Add → Windows action → tap the key.
