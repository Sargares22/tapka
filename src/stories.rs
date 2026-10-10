//! One test per user story, so `cargo test -- --list` reads as the list of stories. Stories checked by hand or by scripts/check-page.mjs have no test here.

use crate::actions::{capsule_returns, hides_capsule, steps_for, Running, Snip, Step};
use crate::config::{load_or_create, parse, reload, Action, Item, Settings};
use crate::layout::Edge;

/// What a tap on an item with this action does when its program is not running.
fn tapped(action: Action) -> Vec<Step> {
    steps_for(&action, Running::No)
}

#[test]
fn story_01_snapshot_item_sends_win_shift_s() {
    let steps = tapped(Action::Hotkey("win+shift+s".into()));
    assert_eq!(steps.last(), Some(&Step::SendKeys("win+shift+s".into())));
    assert_eq!(crate::keys::parse("win+shift+s").unwrap(), [0x5B, 0x10, 0x53]);
}

#[test]
fn story_02_capsule_hides_before_snapshot() {
    // Hidden first, keys second; the same hotkey written differently hides too
    assert_eq!(
        tapped(Action::Hotkey("win+shift+s".into())),
        [Step::HideCapsule, Step::SendKeys("win+shift+s".into())]
    );
    assert!(hides_capsule("Shift + Win + S"));
    // No other hotkey hides the capsule
    for keys in ["ctrl+space", "win+s", "ctrl+shift+s", "win+shift+ctrl+s", "printscreen"] {
        assert!(!hides_capsule(keys), "{keys}");
        assert_eq!(tapped(Action::Hotkey(keys.into())), [Step::SendKeys(keys.into())]);
    }
}

#[test]
fn story_03_capsule_returns_after_snapshot() {
    // The rule only; the real return is checked by hand.
    // The selection closed, with a shot or cancelled: back at once
    assert!(capsule_returns(Snip::Closed, 400));
    // While it is open the capsule stays away, up to twenty seconds
    assert!(!capsule_returns(Snip::Open, 19_999));
    assert!(capsule_returns(Snip::Open, 20_000));
    // It never opened: back after three seconds
    assert!(!capsule_returns(Snip::NotYet, 2_999));
    assert!(capsule_returns(Snip::NotYet, 3_000));
}

#[test]
fn story_04_dictation_item_sends_ctrl_space() {
    assert_eq!(tapped(Action::Hotkey("ctrl+space".into())), [Step::SendKeys("ctrl+space".into())]);
    assert_eq!(crate::keys::parse("ctrl+space").unwrap(), [0x11, 0x20]);
}

#[test]
fn story_05_site_item_opens_its_address() {
    let site = Action::Open("https://example.com".into());
    assert_eq!(tapped(site), [Step::Open("https://example.com".into())]);
}

#[test]
fn story_06_app_item_opens_the_app() {
    // Whatever the target is (exe path, file, shell: address), it is handed to Windows as is
    let target = r"shell:AppsFolder\Vendor.Notes_abc123xyz!App";
    assert_eq!(tapped(Action::Open(target.into())), [Step::Open(target.into())]);
}

#[test]
fn story_08_item_is_added_by_one_entry_in_settings() {
    let (s, skipped) = parse(r#"{"items":[{"name":"Сайт","action":"open","target":"https://example.com"}]}"#).unwrap();
    assert!(skipped.is_empty());
    assert_eq!(s.items, vec![Item { name: "Сайт".into(), icon: None, hint: None, lit: None, only_in: None, action: Action::Open("https://example.com".into()) }]);
    // scale and top are optional
    assert_eq!((s.scale, s.top), (1.0, 0.3));
}

#[test]
fn story_09_items_follow_the_order_in_settings() {
    let text = r#"{"scale":0.8,"top":0.5,"items":[
        {"name":"b","action":"hotkey","keys":"ctrl+space"},
        {"name":"a","icon":"a.png","action":"open","target":"x.exe"},
        {"name":"c","action":"open","target":"y"}]}"#;
    let (s, _) = parse(text).unwrap();
    let names: Vec<_> = s.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["b", "a", "c"]);
    assert_eq!(s.items[1].icon.as_deref(), Some("a.png"));
    assert_eq!((s.scale, s.top), (0.8, 0.5));
}

#[test]
fn story_11_invalid_settings_keep_previous_items() {
    let (valid, _) = parse(r#"{"scale":0.8,"items":[{"name":"a","action":"open","target":"x"}]}"#).unwrap();
    // Invalid after valid: nothing changes and the reason is reported
    let (kept, skipped, error) = reload(&valid, "{ oops");
    assert_eq!(kept, valid);
    assert!(skipped.is_empty());
    assert!(error.unwrap().starts_with("not JSON"));
    let (kept, _, error) = reload(&valid, r#"{"scale":1}"#);
    assert_eq!(kept, valid);
    assert_eq!(error.as_deref(), Some("no \"items\" list"));
    // Valid again: the new items replace the old ones and the error is gone
    let (fresh, _, error) = reload(&valid, r#"{"items":[{"name":"b","action":"open","target":"y"}]}"#);
    assert_eq!(fresh.items[0].name, "b");
    assert!(error.is_none());
}

#[test]
fn story_12_first_run_creates_settings_with_windows_actions() {
    let dir = std::env::temp_dir().join(format!("tapka-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("settings.json");
    let (s, skipped, created) = load_or_create(&path, true).unwrap();
    assert!(path.exists() && created);
    assert!(skipped.is_empty());
    let names: Vec<_> = s.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["Снимок", "Вставить", "Голосовой ввод", "Проводник", "Параметры"]);
    assert_eq!(s.items[0].action, Action::Hotkey("win+shift+s".into()));
    assert_eq!(s.items[1].action, Action::Hotkey("ctrl+v".into()));
    assert_eq!(s.items[2].action, Action::Hotkey("win+h".into()));
    // Every item names a built-in icon and carries a hint for the label
    for item in &s.items {
        assert!(item.icon.as_deref().is_some_and(|i| crate::glyphs::builtin(i).is_some()));
        assert!(item.hint.is_some());
    }
    // `lit` comes from the file only; one without both parts is left out
    let lit = r#"{"items":[{"name":"a","action":"hotkey","keys":"f1","lit":{"program":"Recorder.EXE","window":"Recording"}}]}"#;
    assert_eq!(parse(lit).unwrap().0.items[0].lit, Some(("recorder.exe".into(), "Recording".into())));
    let (odd, skipped) = parse(r#"{"items":[{"name":"a","action":"hotkey","keys":"f1","lit":{"program":"X.EXE"}}]}"#).unwrap();
    assert!(skipped.is_empty() && odd.items[0].lit.is_none());
    // an existing file is read, not overwritten
    std::fs::write(&path, r#"{"items":[]}"#).unwrap();
    let (again, _, created) = load_or_create(&path, true).unwrap();
    assert!(again.items.is_empty() && !created);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn story_12a_broken_item_is_skipped_and_logged() {
    let text = r#"{"items":[
        {"action":"open","target":"x"},
        {"name":"ok","action":"open","target":"x"},
        {"name":"n","action":"script","target":"x"},
        {"name":"n","action":"open","target":"  "},
        {"name":"n","action":"hotkey"},
        {"name":"n","action":"hotkey","keys":"ctrl+shift"},
        "not an object"]}"#;
    let (s, skipped) = parse(text).unwrap();
    assert_eq!(s.items.len(), 1);
    assert_eq!(s.items[0].name, "ok");
    assert_eq!(skipped, [
        "item 1 skipped: no name",
        "item 3 skipped: unknown action \"script\"",
        "item 4 skipped: empty target",
        "item 5 skipped: empty keys",
        "item 6 skipped: hotkey \"ctrl+shift\": no key besides modifiers",
        "item 7 skipped: no name",
    ]);
}

#[test]
fn story_12b_invalid_settings_at_start_give_empty_capsule() {
    assert!(parse("{ not json").unwrap_err().starts_with("not JSON"));
    assert_eq!(parse(r#"{"scale":1}"#).unwrap_err(), "no \"items\" list");
    assert_eq!(parse(r#"{"items":"x"}"#).unwrap_err(), "no \"items\" list");
    assert!(Settings::empty().items.is_empty());
}

#[test]
fn story_15_four_items_show_and_the_rest_scroll() {
    use crate::layout::{pill_height, window_rect, ADD, CELL, END, GRIP, MARGIN, PEEK, ROOM};
    // Work area 2880 x 1824 physical at 200 %: 912 CSS px tall, the pill's top at 0.3
    let work = (0, 0, 2880, 1824);
    let height_css = |n, top| window_rect(work, 2.0, top, CELL, n, Edge::Right).3 as f64 / 2.0;
    // Up to four items the pill grows with them: the handle, the items, the foot with the plus
    assert_eq!(pill_height(0, CELL), GRIP + ADD + END);
    assert_eq!(pill_height(4, CELL), GRIP + 4.0 * CELL + ADD + END);
    assert_eq!(height_css(4, 0.3), pill_height(4, CELL) + 2.0 * ROOM);
    // From the fifth on it stays as long, plus the peek of the next item, and the list scrolls
    assert_eq!(pill_height(5, CELL), pill_height(4, CELL) + PEEK);
    assert_eq!(pill_height(30, CELL), pill_height(5, CELL));
    assert_eq!(height_css(7, 0.3), height_css(30, 0.3));
    // Too close to the end of the work area the window is pulled back until the whole pill shows
    let (_, y, _, height) = window_rect(work, 2.0, 0.8, CELL, 10, Edge::Right);
    assert_eq!(height as f64 / 2.0, pill_height(10, CELL) + 2.0 * ROOM);
    assert_eq!(y + height, 1824);
    // The same pill at the largest size, with the wider step of a tablet, still shows whole
    let large = window_rect(work, 2.5, 0.55, CELL * 1.3, 10, Edge::Right);
    assert_eq!(large.3, ((pill_height(10, CELL * 1.3) + 2.0 * ROOM) * 2.5).round() as i32);
    assert!(large.1 >= 0 && large.1 + large.3 <= 1824);
    // Only a pill longer than the work area itself is cut short, MARGIN from its end
    let low = (0, 0, 2880, 500);
    assert_eq!(window_rect(low, 2.0, 0.0, CELL, 10, Edge::Right).3 as f64 / 2.0, 250.0 - MARGIN);
}

#[test]
fn story_20_scale_snaps_to_three_values() {
    use crate::config::snap_scale;
    for (given, used) in [(0.8, 0.8), (1.0, 1.0), (1.25, 1.25), (0.5, 0.8), (0.89, 0.8), (0.95, 1.0), (1.1, 1.0), (1.2, 1.25), (3.0, 1.25)] {
        assert_eq!(snap_scale(given), used, "{given}");
    }
    assert_eq!(snap_scale(f64::NAN), 1.0);
    // The settings file goes through the same rule
    assert_eq!(parse(r#"{"scale":0.85,"items":[]}"#).unwrap().0.scale, 0.8);
    assert_eq!(parse(r#"{"scale":"big","items":[]}"#).unwrap().0.scale, 1.0);
    // The capsule's strip is the same shape at every scale: 78 CSS px thick times the scale
    let width = |scale: f64| {
        let (_, _, w, h) = crate::layout::window_rect((0, 0, 2880, 1824), 2.0 * scale, 0.62, crate::layout::CELL, 4, Edge::Right);
        crate::layout::strip((w, h), 2.0 * scale, Edge::Right).2
    };
    assert_eq!((width(0.8), width(1.0), width(1.25)), (125, 156, 195));
}

#[test]
fn story_21_tablet_mode_widens_the_step() {
    use crate::layout::{cell, pill_height, CELL};
    // With a keyboard the step from key to key is 54; without, 1.3 times wider
    assert_eq!(cell(false), CELL);
    assert_eq!(CELL, 54.0);
    assert!((cell(true) / cell(false) - 1.3).abs() < 1e-9);
    // Four items: four cells wider; the foot with the plus stays as it is
    assert!((pill_height(4, cell(true)) - pill_height(4, cell(false)) - 4.0 * (cell(true) - cell(false))).abs() < 1e-9);
}
#[test]
fn story_22_capsule_hides_with_a_keyboard_when_asked() {
    use crate::actions::keyboard_shows_capsule;
    // Out of the box the keyboard changes nothing: the capsule is on screen either way
    assert!(!parse(r#"{"items":[]}"#).unwrap().0.tablet_only);
    assert!(keyboard_shows_capsule(false, false) && keyboard_shows_capsule(false, true));
    // Asked to show only without a keyboard: away while it is attached, back when it is not
    assert!(!keyboard_shows_capsule(true, false));
    assert!(keyboard_shows_capsule(true, true));
}

#[test]
fn story_23_default_position() {
    use crate::layout::{window_rect, ROOM, CELL};
    // The default file and a file without `top` both put the pill's top edge at 0.3 of the work area
    assert_eq!(parse(r#"{"items":[]}"#).unwrap().0.top, 0.3);
    assert_eq!(parse(&crate::config::default_text(false)).unwrap().0.top, 0.3);
    // Work area 2880 x 1824 at 200 %, the taskbar below it
    let (x, y, width, _) = window_rect((0, 0, 2880, 1824), 2.0, 0.3, CELL, 4, Edge::Right);
    let pill_top = y as f64 + ROOM * 2.0;
    assert!((pill_top - 1824.0 * 0.3).abs() <= 1.0);
    // Pressed to the right edge
    assert_eq!(x + width, 2880);
}

#[test]
fn story_24_top_edge_comes_from_settings() {
    use crate::layout::{window_rect, ROOM, CELL};
    assert_eq!(parse(r#"{"top":0.3,"items":[]}"#).unwrap().0.top, 0.3);
    let work = (0, 40, 2880, 1800);
    let pill_top = |top: f64| window_rect(work, 2.0, top, CELL, 2, Edge::Right).1 as f64 + ROOM * 2.0;
    assert_eq!(pill_top(0.3), 40.0 + 540.0);
    assert_eq!(pill_top(0.5), 40.0 + 900.0);
    // A value that would push the capsule off the work area keeps the whole window inside it
    for top in [-1.0, 0.0, 0.99, 1.0, 7.0, f64::NAN] {
        let (_, y, _, height) = window_rect(work, 2.0, top, CELL, 2, Edge::Right);
        assert!(y >= 40 && y + height <= 40 + 1800, "top {top}: y {y} height {height}");
    }
}
