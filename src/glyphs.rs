//! Built-in icons, compiled into the binary. An item's `icon` may name one of them without an
//! extension: `"icon": "snip"`. All of them are drawn for the project, in `currentColor`. There
//! are no marks of other products here: a program's picture comes from the program itself.

const GLYPHS: [(&str, &str); 13] = [
    ("snip", include_str!("../glyphs/snip.svg")),
    ("paste", include_str!("../glyphs/paste.svg")),
    ("copy", include_str!("../glyphs/copy.svg")),
    ("undo", include_str!("../glyphs/undo.svg")),
    ("mic", include_str!("../glyphs/mic.svg")),
    ("tasks", include_str!("../glyphs/tasks.svg")),
    ("desktop", include_str!("../glyphs/desktop.svg")),
    ("folder", include_str!("../glyphs/folder.svg")),
    ("gear", include_str!("../glyphs/gear.svg")),
    ("keyboard", include_str!("../glyphs/keyboard.svg")),
    ("terminal", include_str!("../glyphs/terminal.svg")),
    ("chat", include_str!("../glyphs/chat.svg")),
    ("browser", include_str!("../glyphs/browser.svg")),
];

/// The SVG text of a built-in icon, or `None` if there is no such name. It is drawn in
/// `currentColor`: the page puts it inline, so it takes the ink of the theme.
pub fn builtin(name: &str) -> Option<&'static str> {
    GLYPHS.iter().find(|(n, _)| *n == name).map(|(_, svg)| *svg)
}

/// The names of all built-in icons, in the order the settings window offers them.
pub fn names() -> impl Iterator<Item = &'static str> {
    GLYPHS.iter().map(|(name, _)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_is_an_svg_in_the_ink_of_the_page() {
        for (name, _) in GLYPHS {
            let svg = builtin(name).unwrap();
            assert!(svg.trim_start().starts_with("<svg"), "{name}");
            assert!(svg.contains("currentColor"), "{name}");
            assert!(!svg.contains("<script"), "{name}");
        }
        assert!(builtin("snip.svg").is_none());
        assert!(builtin("nope").is_none());
    }
}