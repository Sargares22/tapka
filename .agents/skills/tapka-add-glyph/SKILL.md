---
name: tapka-add-glyph
description: Add a built-in icon (glyph) to Tapka. Use when a new action needs an icon or the user asks for another built-in icon.
---

Icons are drawn for this project. No logos or marks of other products.

1. `glyphs/<name>.svg`: `viewBox="0 0 24 24"`, `fill="none"`, `stroke="currentColor"`,
   `stroke-width="1.8"`, round caps and joins. No scripts, links or embedded images. Copy the
   opening tag from `glyphs/snip.svg`.
2. `src/glyphs.rs`: add the name to `GLYPHS` and raise the array length. The order is the order
   the settings window offers them in.
3. `glyphs/NOTICE.md`: add a row.
4. `README.md` and `README.ru.md`: add the name to the list of built-in icon names.
5. Run `sh scripts/check.sh`, then `node scripts/shot.mjs` and look at the pictures in
   `target/shots`: the icon must read at 24 px next to the others.
