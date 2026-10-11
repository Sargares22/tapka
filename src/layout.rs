//! Capsule geometry in CSS px at scale 1. The capsule is a 60 px thick strip docked flush to a
//! screen edge: a drag handle at its start, then up to four items, one 54 px cell each (a 46 px
//! key inside), then a small plus that opens the editor. More items than four scroll in their place,
//! and the edge of the next one peeks out, faded, to say so. At the left and right edges the
//! capsule stands upright; at the top edge it lies flat.
//!
//! The window never changes size while the capsule is in use: it always includes the room where
//! the name label appears. What is visible and takes presses is cut out of it with a window
//! region (`strip`), so opening the label does not resize anything.

/// The edge the capsule is docked to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Edge {
    Right,
    Left,
    Top,
}

/// Thickness of the strip of the window the capsule lives in: the pill's 60 plus 18 px of shadow
/// on its inner side.
pub const WIDTH: f64 = 78.0;
/// Thickness of the pill itself.
pub const PILL: f64 = 60.0;
/// Corner radius of the pill.
pub const RADIUS: f64 = 22.0;
/// Transparent room before and after the pill along the edge, for its shadow.
pub const ROOM: f64 = 12.0;
/// The drag handle's stretch at the start of the pill: a full finger target.
pub const GRIP: f64 = 44.0;
/// One item's cell, key and spacing together: the step from key to key.
pub const CELL: f64 = 54.0;
/// After the last cell, so the last key is as far from the pill's end as from its sides.
pub const END: f64 = 3.0;
/// Free space kept between the window and the far end of the work area.
pub const MARGIN: f64 = 16.0;
/// Room for the name label beside an upright capsule, and under a flat one.
pub const CARD_ROOM: f64 = 270.0;
pub const CARD_ROOM_TOP: f64 = 96.0;
/// In tablet mode the step between items is this many times wider.
pub const TABLET_STEP: f64 = 1.3;
/// A capsule dropped this close to the top of the work area docks to the top edge.
pub const SNAP: f64 = 30.0;

/// Length of an item's cell: the usual step with a keyboard, a wider one for fingers without.
pub fn cell(tablet: bool) -> f64 {
    if tablet {
        CELL * TABLET_STEP
    } else {
        CELL
    }
}

/// How many items the capsule shows at once; the rest scroll.
pub const SHOWN: usize = 4;
/// How much of the next item shows past the last shown one when there are more.
pub const PEEK: f64 = 18.0;

/// The foot of the pill, where the small plus sits. A tap on the plus opens two signs in its
/// place, the gear and the fold arrow, each as long as the plus: the pill is one `ADD` longer
/// while they show.
pub const ADD: f64 = 38.0;
/// Length of the pill of a folded capsule: one key, as far from the pill's ends as from its sides.
pub const FOLDED: f64 = 60.0;

/// Length of the pill for `n` items when nothing limits it: the handle, the items that show, the
/// peek of the next one if there is one, and the foot with the plus.
pub fn pill_height(n: usize, cell: f64) -> f64 {
    let peek = if n > SHOWN { PEEK } else { 0.0 };
    GRIP + n.min(SHOWN) as f64 * cell + peek + ADD + END
}

/// Length of the pill along its edge, in CSS px: as long as `pill_height` says, one sign longer
/// while the two signs show in the foot in place of the plus (`signs`), and no longer than fits in
/// the work area. `work` is the work area `(x, y, width, height)` in physical px, `k` is physical
/// px per CSS px. The page draws the pill this long and the window is sized for it, so neither
/// waits for the other when the signs open or close.
pub fn pill_length(work: (i32, i32, i32, i32), k: f64, cell: f64, n: usize, edge: Edge, signs: bool) -> f64 {
    let more = if signs { ADD } else { 0.0 };
    let span = if edge == Edge::Top { work.2 } else { work.3 };
    let room = (span as f64 / k - 2.0 * ROOM - MARGIN).max(pill_height(0, cell) + more);
    (pill_height(n, cell) + more).min(room)
}

/// Where the capsule window goes, in physical px: `(x, y, width, height)`.
/// `work` is the work area `(x, y, width, height)` in physical px, `k` is physical px per CSS px
/// (monitor scale × panel scale). `along` is where the pill starts along its edge, as a share of
/// the work area: its top edge for an upright capsule, its left end for a flat one.
/// The pill is as long as `pill_length` says. Where that is more than fits between `along` and the
/// end of the work area, the window is pulled back along the edge until the whole pill shows; only a
/// pill longer than the work area itself is cut short, and then the list scrolls inside it.
/// `signs` says that the two signs show in the foot in place of the plus.
pub fn window_rect_with(work: (i32, i32, i32, i32), k: f64, along: f64, cell: f64, n: usize, edge: Edge, signs: bool) -> (i32, i32, i32, i32) {
    let (wx, wy, ww, wh) = work;
    let along = if along.is_finite() { along.clamp(0.0, 1.0) } else { 0.0 };
    let long = ((pill_length(work, k, cell, n, edge, signs) + 2.0 * ROOM) * k).round() as i32;
    if edge == Edge::Top {
        let height = ((WIDTH + CARD_ROOM_TOP) * k).round() as i32;
        let x = wx + (ww as f64 * along - ROOM * k).round() as i32;
        return (x.min(wx + ww - long).max(wx), wy, long, height);
    }
    let width = ((WIDTH + CARD_ROOM) * k).round() as i32;
    let x = if edge == Edge::Left { wx } else { wx + ww - width };
    let y = wy + (wh as f64 * along - ROOM * k).round() as i32;
    (x, y.min(wy + wh - long).max(wy), width, long)
}

/// The window of a capsule at rest, with the plus in its foot: what the tests measure.
#[cfg(test)]
pub fn window_rect(work: (i32, i32, i32, i32), k: f64, along: f64, cell: f64, n: usize, edge: Edge) -> (i32, i32, i32, i32) {
    window_rect_with(work, k, along, cell, n, edge, false)
}

/// The window of a folded capsule, as a rectangle on the screen: the short pill with its one key
/// and the room for its shadow, and nothing else. `at` is where that pill starts along the edge, as a share of the work area, like
/// `along` for the open capsule. It never leaves the work area.
pub fn folded_window(work: (i32, i32, i32, i32), k: f64, at: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (wx, wy, ww, wh) = work;
    let at = if at.is_finite() { at.clamp(0.0, 1.0) } else { 0.0 };
    let (thick, long) = ((WIDTH * k).round() as i32, ((FOLDED + 2.0 * ROOM) * k).round() as i32);
    if edge == Edge::Top {
        let start = wx + (ww as f64 * at - ROOM * k).round() as i32;
        return (start.min(wx + ww - long).max(wx), wy, long.min(ww), thick.min(wh));
    }
    let x = if edge == Edge::Left { wx } else { wx + ww - thick.min(ww) };
    let start = wy + (wh as f64 * at - ROOM * k).round() as i32;
    (x, start.min(wy + wh - long).max(wy), thick.min(ww), long.min(wh))
}

/// What shows and takes presses of a folded capsule's window of `size`, `(x, y, width, height)`
/// inside the window: the pill itself. The room around it is cut away, shadow and all, so a
/// press beside the key goes to whatever is under it.
pub fn folded_region(size: (i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    pill_on_screen((0, 0, size.0, size.1), k, edge)
}

/// Where the handle window lies while the capsule is folded: over the folded pill, all of it and
/// no more. Windows carries the folded key by it, the way it carries the capsule by its handle.
pub fn folded_grip(work: (i32, i32, i32, i32), k: f64, at: f64, edge: Edge) -> (i32, i32, i32, i32) {
    pill_on_screen(folded_window(work, k, at, edge), k, edge)
}

/// A press on the handle window moved it at most this far, in CSS px: a touch, not a carry.
pub const TAP_SLACK: f64 = 8.0;

/// Was a press on the handle window a touch? `carried` says that Windows began to carry the
/// window, `moved` is the farthest it got from where it stood, in physical px. A window carried
/// away and brought back was carried all the same.
pub fn is_tap(carried: bool, moved: i32, k: f64) -> bool {
    !carried || moved <= (TAP_SLACK * k).round() as i32
}

/// Where an open capsule whose window, with the two signs showing, is at `window` folds to: the
/// cut-out whose key has its middle where the fold sign is, the last sign of the foot. It may
/// reach past the work area; `along_from` makes the share of it and `folded_window` keeps it inside.
pub fn fold_spot(window: (i32, i32, i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (x, y, w, h) = window;
    let (thick, long) = ((WIDTH * k).round() as i32, ((FOLDED + 2.0 * ROOM) * k).round() as i32);
    // From the end of the window back to the sign's middle, and half the cut-out further
    let back = ((ROOM + END + ADD / 2.0) * k).round() as i32 + long / 2;
    match edge {
        Edge::Right => (x + w - thick, y + h - back, thick, long),
        Edge::Left => (x, y + h - back, thick, long),
        Edge::Top => (x + w - back, y, long, thick),
    }
}

/// The part of a window of `size` the capsule itself occupies, `(x, y, width, height)` inside
/// the window: everything but the label's room.
pub fn strip(size: (i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (w, h) = size;
    let thick = (WIDTH * k).round() as i32;
    match edge {
        Edge::Right => (w - thick.min(w), 0, thick.min(w), h),
        Edge::Left => (0, 0, thick.min(w), h),
        Edge::Top => (0, 0, w, thick.min(h)),
    }
}

/// The pill of a capsule whose window is at `window`, as a rectangle on the screen.
pub fn pill_on_screen(window: (i32, i32, i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (x, y, w, h) = window;
    let (thick, room) = ((PILL * k).round() as i32, (ROOM * k).round() as i32);
    match edge {
        Edge::Right => (x + w - thick, y + room, thick, h - 2 * room),
        Edge::Left => (x, y + room, thick, h - 2 * room),
        Edge::Top => (x + room, y, w - 2 * room, thick),
    }
}

/// The handle of a capsule whose window is at `window`, as a rectangle on the screen: the start
/// of the pill together with the empty room before it, so a finger has a full target.
pub fn grip_on_screen(window: (i32, i32, i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (x, y, w, _) = window;
    let (thick, long) = ((PILL * k).round() as i32, ((ROOM + GRIP) * k).round() as i32);
    match edge {
        Edge::Right => (x + w - thick, y, thick, long),
        Edge::Left => (x, y, thick, long),
        Edge::Top => (x, y, long, thick),
    }
}

/// The edge a capsule dropped with its strip at `at` (screen px) docks to. Close to the top of
/// the work area and away from its corners it lies down along the top edge; otherwise it goes to
/// the nearer side. The corners belong to the sides, so an upright capsule can stand high.
pub fn drop_edge(work: (i32, i32, i32, i32), k: f64, at: (i32, i32, i32, i32)) -> Edge {
    let (wx, wy, ww, _) = work;
    let (x, y, w, _) = at;
    let middle = x + w / 2;
    let near_top = y - wy <= (SNAP * k).round() as i32;
    let off_corners = middle > wx + ww * 15 / 100 && middle < wx + ww * 85 / 100;
    if near_top && off_corners {
        Edge::Top
    } else if middle < wx + ww / 2 {
        Edge::Left
    } else {
        Edge::Right
    }
}

/// The `along` share for a capsule dropped with its strip at `at` and docking to `edge`: the
/// inverse of `window_rect`'s placement along the edge.
pub fn along_from(work: (i32, i32, i32, i32), k: f64, at: (i32, i32, i32, i32), edge: Edge) -> f64 {
    let (wx, wy, ww, wh) = work;
    let pad = ROOM * k;
    let share = if edge == Edge::Top {
        ((at.0 - wx) as f64 + pad) / ww as f64
    } else {
        ((at.1 - wy) as f64 + pad) / wh as f64
    };
    share.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turned_screen_keeps_the_capsule_at_its_edge_inside_the_work_area() {
        // The same tablet in four positions, a taskbar of 96 px at the bottom each time, then a
        // lower resolution and another scale
        let screens = [
            ((0, 0, 2880, 1824), 2.0),
            ((0, 0, 1920, 2784), 2.0),
            ((0, 0, 2880, 1824), 2.0),
            ((0, 0, 1920, 2784), 2.0),
            ((0, 0, 1280, 672), 1.0),
            ((0, 0, 1920, 1032), 1.5),
        ];
        for edge in [Edge::Right, Edge::Left, Edge::Top] {
            for along in [0.0, 0.4529, 0.66, 0.95] {
                for (work, k) in screens {
                    let (wx, wy, ww, wh) = work;
                    let (x, y, w, h) = window_rect(work, k, along, CELL, 7, edge);
                    let what = format!("{edge:?} {along} {work:?}");
                    assert!(x >= wx && y >= wy && x + w <= wx + ww && y + h <= wy + wh, "{what}");
                    let (px, py, pw, _) = pill_on_screen((x, y, w, h), k, edge);
                    match edge {
                        Edge::Right => assert_eq!(px + pw, wx + ww, "{what}"),
                        Edge::Left => assert_eq!(px, wx, "{what}"),
                        Edge::Top => assert_eq!(py, wy, "{what}"),
                    }
                    // The pill starts at the saved share of the new work area unless an end of
                    // the work area is in the way
                    let (start, span) = if edge == Edge::Top { (px - wx, ww) } else { (py - wy, wh) };
                    let wanted = (span as f64 * along).round() as i32;
                    let at_an_end = if edge == Edge::Top { x == wx || x + w == wx + ww } else { y == wy || y + h == wy + wh };
                    assert!(start == wanted || at_an_end, "{what}: {start} vs {wanted}");
                }
            }
        }
    }
    const WORK: (i32, i32, i32, i32) = (0, 0, 2880, 1824);

    fn strip_on_screen(along: f64, edge: Edge) -> (i32, i32, i32, i32) {
        let (x, y, w, h) = window_rect(WORK, 2.0, along, CELL, 4, edge);
        let (sx, sy, sw, sh) = strip((w, h), 2.0, edge);
        (x + sx, y + sy, sw, sh)
    }

    #[test]
    fn along_from_inverts_window_rect() {
        for edge in [Edge::Right, Edge::Left, Edge::Top] {
            for along in [0.0, 0.3, 0.66, 0.9] {
                let at = strip_on_screen(along, edge);
                let back = along_from(WORK, 2.0, at, edge);
                assert_eq!(at, strip_on_screen(back, edge), "{edge:?} along {along} -> {back}");
            }
        }
    }

    #[test]
    fn the_strip_hugs_its_edge_and_the_window_stays_in_the_work_area() {
        assert_eq!(strip_on_screen(0.5, Edge::Right).0 + 156, 2880);
        assert_eq!(strip_on_screen(0.5, Edge::Left).0, 0);
        assert_eq!(strip_on_screen(0.5, Edge::Top).1, 0);
        for edge in [Edge::Right, Edge::Left, Edge::Top] {
            for along in [-1.0, 0.0, 0.5, 1.0, 9.0, f64::NAN] {
                let (x, y, w, h) = window_rect(WORK, 2.0, along, CELL, 12, edge);
                assert!(x >= 0 && y >= 0 && x + w <= 2880 && y + h <= 1824, "{edge:?} {along}: {x},{y} {w}x{h}");
            }
        }
    }

    #[test]
    fn the_open_foot_lengthens_the_capsule_and_the_folded_key_stays_in_the_work_area() {
        let inside = |(x, y, w, h): (i32, i32, i32, i32), (wx, wy, ww, wh): (i32, i32, i32, i32)| x >= wx && y >= wy && x + w <= wx + ww && y + h <= wy + wh;
        // A tablet at 200 %, lying and standing, with a taskbar; the three sizes of the panel
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744)];
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            for edge in [Edge::Right, Edge::Left, Edge::Top] {
                // (start, end) of a rectangle along the edge
                let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { (r.0, r.0 + r.2) } else { (r.1, r.1 + r.3) };
                for (work, cell) in [(works[0], CELL), (works[1], CELL), (works[0], CELL * TABLET_STEP)] {
                    for along in [0.0, 0.3, 0.62, 0.97, 1.0] {
                        let what = format!("{edge:?} scale {scale} along {along} {work:?} cell {cell}");
                        let work_end = span(work).1;
                        // The two signs: the capsule is one sign longer, whole, and where it was
                        // unless the end of the work area is in the way; there it is pulled back
                        // along the edge, not cut short
                        let rest = window_rect(work, k, along, cell, 6, edge);
                        let open = window_rect_with(work, k, along, cell, 6, edge, true);
                        assert!(inside(open, work), "{what}: {open:?}");
                        let longer = (span(open).1 - span(open).0) - (span(rest).1 - span(rest).0);
                        assert!((longer - (ADD * k).round() as i32).abs() <= 1, "{what}: longer by {longer}");
                        assert!(span(open).0 == span(rest).0 || span(open).1 == work_end, "{what}: {rest:?} -> {open:?}");
                        // Folded: the key's middle is where the fold sign was, at the same edge
                        let at = along_from(work, k, fold_spot(open, k, edge), edge);
                        let cut = folded_window(work, k, at, edge);
                        assert!(inside(cut, work), "{what}: {cut:?}");
                        match edge {
                            Edge::Right => assert_eq!(cut.0 + cut.2, work.0 + work.2, "{what}"),
                            Edge::Left => assert_eq!(cut.0, work.0, "{what}"),
                            Edge::Top => assert_eq!(cut.1, work.1, "{what}"),
                        }
                        let sign = span(open).1 - ((ROOM + END + ADD / 2.0) * k).round() as i32;
                        let middle = (span(cut).0 + span(cut).1) / 2;
                        assert!((middle - sign).abs() <= 2 || span(cut).1 == work_end, "{what}: key at {middle}, sign at {sign}");
                        // A carry saves the place the same way, and it reads back to the same place
                        assert_eq!(folded_window(work, k, along_from(work, k, cut, edge), edge), cut, "{what}");
                        // The screen is turned: the same share of the other work area, inside it
                        for turned in works {
                            assert!(inside(folded_window(turned, k, at, edge), turned), "{what} turned to {turned:?}");
                        }
                    }
                }
                for odd in [-1.0, 9.0, f64::NAN] {
                    assert!(inside(folded_window(works[0], k, odd, edge), works[0]), "{edge:?} {odd}");
                }
            }
        }
    }

    #[test]
    fn the_window_is_as_long_as_the_pill_the_page_draws_and_the_signs_move_nothing_before_them() {
        // A tablet at 200 %, lying and standing, and a screen too short for the whole capsule
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 640, 560)];
        for (work, k) in [(works[0], 2.0), (works[1], 2.5), (works[0], 1.6), (works[2], 2.0)] {
            for edge in [Edge::Right, Edge::Left, Edge::Top] {
                let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { (r.0, r.2) } else { (r.1, r.3) };
                for (cell, n) in [(CELL, 3), (CELL, 8), (CELL * TABLET_STEP, 8)] {
                    let rest = pill_length(work, k, cell, n, edge, false);
                    let open = pill_length(work, k, cell, n, edge, true);
                    // One sign longer, unless the work area has no room for it
                    let fits = pill_height(n, cell) + ADD <= span(work).1 as f64 / k - 2.0 * ROOM - MARGIN;
                    assert!((open - rest - ADD).abs() < 1e-9 || !fits, "{edge:?} {work:?} {k}: {rest} -> {open}");
                    for along in [0.0, 0.3, 0.62, 0.97] {
                        let what = format!("{edge:?} {work:?} k {k} along {along} cell {cell} n {n}");
                        let at_rest = window_rect_with(work, k, along, cell, n, edge, false);
                        let with_signs = window_rect_with(work, k, along, cell, n, edge, true);
                        // The window is the pill the page draws and the room before and after it
                        assert_eq!(span(at_rest).1, ((rest + 2.0 * ROOM) * k).round() as i32, "{what}");
                        assert_eq!(span(with_signs).1, ((open + 2.0 * ROOM) * k).round() as i32, "{what}");
                        // The pill starts where the window does: with the signs it starts where it
                        // did, or the window is pulled back whole from the end of the work area
                        let end = span(work).0 + span(work).1;
                        assert!(span(with_signs).0 == span(at_rest).0 || span(with_signs).0 + span(with_signs).1 == end, "{what}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_handle_window_lies_over_the_folded_key_and_a_touch_is_told_from_a_carry() {
        // A tablet at 200 % with a taskbar on top, the three sizes of the panel
        let work = (0, 40, 2880, 1784);
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            let (side, room) = ((PILL * k).round() as i32, (ROOM * k).round() as i32);
            for edge in [Edge::Right, Edge::Left, Edge::Top] {
                // The start of the edge, the middle, and past the end of the work area
                for at in [0.0, 0.5, 1.0] {
                    let what = format!("{edge:?} scale {scale} at {at}");
                    let (wx, wy, ww, wh) = folded_window(work, k, at, edge);
                    let (x, y, w, h) = folded_grip(work, k, at, edge);
                    // The whole pill, which is as long as it is thick, flush with the edge
                    assert_eq!((w, h), (side, side), "{what}");
                    assert!(x >= wx && y >= wy && x + w <= wx + ww && y + h <= wy + wh, "{what}: outside the folded window");
                    assert!(x >= work.0 && y >= work.1 && x + w <= work.0 + work.2 && y + h <= work.1 + work.3, "{what}: outside the work area");
                    match edge {
                        Edge::Right => assert_eq!((x + w, y), (work.0 + work.2, wy + room), "{what}"),
                        Edge::Left => assert_eq!((x, y), (work.0, wy + room), "{what}"),
                        Edge::Top => assert_eq!((x, y), (wx + room, work.1), "{what}"),
                    }
                    // What is cut out of the folded window is that pill and no more
                    let (rx, ry, rw, rh) = folded_region((ww, wh), k, edge);
                    assert_eq!((wx + rx, wy + ry, rw, rh), (x, y, w, h), "{what}");
                    // Carried to another edge and let go there, it is over the key again
                    let along = along_from(work, k, (wx, wy, ww, wh), edge);
                    assert_eq!(folded_grip(work, k, along, edge), (x, y, w, h), "{what}");
                }
            }
        }
        // A press Windows never began to carry is a touch; so is one it carried no further than
        // the slack. Carried away and back, the farthest point counts, so that is a carry
        assert!(is_tap(false, 0, 2.0));
        assert!(is_tap(false, 500, 2.0));
        assert!(is_tap(true, 0, 2.0));
        assert!(is_tap(true, 16, 2.0));
        assert!(!is_tap(true, 17, 2.0));
        assert!(is_tap(true, 8, 1.0) && !is_tap(true, 9, 1.0));
    }

    #[test]
    fn a_drop_docks_to_the_top_only_near_it_and_away_from_the_corners() {
        let upright = |x, y| (x, y, 156, 600);
        // Sides: the nearer one
        assert_eq!(drop_edge(WORK, 2.0, upright(300, 700)), Edge::Left);
        assert_eq!(drop_edge(WORK, 2.0, upright(1500, 700)), Edge::Right);
        // Top: within SNAP of the top, in the middle
        assert_eq!(drop_edge(WORK, 2.0, upright(1300, 40)), Edge::Top);
        assert_eq!(drop_edge(WORK, 2.0, upright(1500, 61)), Edge::Right);
        // An upright capsule can still stand in a top corner
        assert_eq!(drop_edge(WORK, 2.0, upright(2724, 0)), Edge::Right);
        assert_eq!(drop_edge(WORK, 2.0, upright(0, 0)), Edge::Left);
        // A flat capsule nudged along the top stays there
        assert_eq!(drop_edge(WORK, 2.0, (900, 0, 700, 156)), Edge::Top);
    }
}
