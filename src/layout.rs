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

/// The foot of the pill, where the small plus that opens the editor sits.
pub const ADD: f64 = 38.0;

/// Length of the pill for `n` items when nothing limits it: the handle, the items that show, the
/// peek of the next one if there is one, and the foot with the plus.
pub fn pill_height(n: usize, cell: f64) -> f64 {
    let peek = if n > SHOWN { PEEK } else { 0.0 };
    GRIP + n.min(SHOWN) as f64 * cell + peek + ADD + END
}

/// Where the capsule window goes, in physical px: `(x, y, width, height)`.
/// `work` is the work area `(x, y, width, height)` in physical px, `k` is physical px per CSS px
/// (monitor scale × panel scale). `along` is where the pill starts along its edge, as a share of
/// the work area: its top edge for an upright capsule, its left end for a flat one.
/// The pill is as long as `pill_height` says unless that would bring the window closer than MARGIN to the
/// end of the work area; then it is cut short and the list scrolls inside it. An `along` that
/// would put the window outside the work area is pulled back in.
pub fn window_rect(work: (i32, i32, i32, i32), k: f64, along: f64, cell: f64, n: usize, edge: Edge) -> (i32, i32, i32, i32) {
    let (wx, wy, ww, wh) = work;
    let along = if along.is_finite() { along.clamp(0.0, 1.0) } else { 0.0 };
    let full = pill_height(n, cell);
    let least = pill_height(0, cell);
    if edge == Edge::Top {
        let room = (ww as f64 * (1.0 - along) / k - ROOM - MARGIN).max(least);
        let width = ((full.min(room) + 2.0 * ROOM) * k).round() as i32;
        let height = ((WIDTH + CARD_ROOM_TOP) * k).round() as i32;
        let x = wx + (ww as f64 * along - ROOM * k).round() as i32;
        return (x.min(wx + ww - width).max(wx), wy, width, height);
    }
    let room = (wh as f64 * (1.0 - along) / k - ROOM - MARGIN).max(least);
    let width = ((WIDTH + CARD_ROOM) * k).round() as i32;
    let height = ((full.min(room) + 2.0 * ROOM) * k).round() as i32;
    let x = if edge == Edge::Left { wx } else { wx + ww - width };
    let y = wy + (wh as f64 * along - ROOM * k).round() as i32;
    (x, y.min(wy + wh - height).max(wy), width, height)
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
