//! Capsule geometry in CSS px at scale 1. The capsule is a 60 px thick strip docked flush to a
//! screen edge: a drag handle at its start, then up to four items, or as many cells as it was
//! stretched to, one 54 px cell each (a 46 px key inside), then a small plus that opens the editor.
//! More items than that scroll in their place, and the edge of the next one peeks out, faded, to
//! say so. At the left and right edges the capsule stands upright; at the top edge it lies flat.
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

/// How many items the capsule shows at once until it is stretched; the rest scroll.
pub const SHOWN: usize = 4;
/// The fewest cells a capsule may be stretched to.
pub const LEAST: usize = 2;
/// How much of the next item shows past the last shown one when there are more.
pub const PEEK: f64 = 18.0;

/// The foot of the pill, where the small plus sits. A tap on the plus opens two signs in its
/// place, the gear and the fold arrow, each as long as the plus, and after them the end grip: the
/// pill is one `ADD` and one `STRETCH` longer while they show.
pub const ADD: f64 = 38.0;
/// The end grip after the two signs, which stretches the capsule: a strip as long as the handle at
/// the pill's head, with the same bar, inside the pill. Nothing past the pill's end is part of it.
pub const STRETCH: f64 = GRIP;
/// Length of the pill of a folded capsule: one key, as far from the pill's ends as from its sides.
pub const FOLDED: f64 = 60.0;

/// The most the keys' part of the pill may take: the cells that show and the peek of the next item
/// if there is one. Without a `length` (cells, from the settings) as many cells as there are items,
/// up to `SHOWN`; with one, that many cells. Never more than the keys need (`pill_height`, `room`):
/// a capsule stretched past its keys shows no empty cells, and grows back to its length when the
/// program in front brings more keys.
pub fn keys_length(n: usize, cell: f64, length: Option<usize>) -> f64 {
    let shown = length.map_or(n.min(SHOWN), |l| l.max(LEAST));
    let peek = if n > shown { PEEK } else { 0.0 };
    shown as f64 * cell + peek
}

/// Length of the pill for `n` items and no meters when nothing limits it: the handle, the keys'
/// part (`keys_length`, no longer than the keys), and the foot with the plus.
pub fn pill_height(n: usize, cell: f64, length: Option<usize>) -> f64 {
    GRIP + keys_length(n, cell, length).min(n as f64 * cell) + ADD + END
}

/// The divider between the keys and the meters: a finger's target with a thin line in its middle.
pub const LINE: f64 = 14.0;
/// One meter's cell: a 32 px dial and its percent under it.
pub const METER: f64 = 52.0;
/// The most room the keys and the meters may share, the divider not counted. Without meters, the
/// keys' part (`keys_length`). With them there is no peek of the next key on top of the cells:
/// without a `length` up to `SHOWN` cells and every meter; with one, that many cells for the keys and
/// the meters together. Where the line stands shares it (`parts`).
pub fn room_for(n: usize, m: usize, cell: f64, length: Option<usize>) -> f64 {
    if m == 0 {
        return keys_length(n, cell, length);
    }
    match length {
        None => n.min(SHOWN) as f64 * cell + m as f64 * METER,
        Some(l) => l.max(LEAST) as f64 * cell,
    }
}

/// Slack for sums of cells that should meet exactly.
const EPS: f64 = 1e-6;

/// How `room` is shared between the keys' part and the meters' part, `(keys, meters)` in CSS px.
/// The room is the same wherever the line stands: the line only moves the border inside it, a whole
/// meter at a time. The meters' part is a whole number of meters, one or more, so none is cut at the
/// line and nothing is empty under the last; the keys get the rest, one key's cell or more and no more
/// than all of them, and scroll in it with the edge of the next key fading out (`room` cuts the room
/// so a place like that exists). Of those places the line stands at the one nearest to `divider`, the
/// keys' part as a share of `room`; without it the meters show as many as they can, all of them when
/// the room has a key's cell besides. The page shares the room the same way (`parts` in
/// `ui/index.html`).
#[cfg(test)]
pub fn parts(room: f64, n: usize, m: usize, cell: f64, divider: Option<f64>) -> (f64, f64) {
    let keys = n as f64 * cell;
    let room = room.max(0.0);
    if m == 0 {
        return (keys.min(room), 0.0);
    }
    if n == 0 {
        return (0.0, (m as f64 * METER).min(room));
    }
    if cell + METER > room {
        // Too little room for one of each: they share it in their proportion
        let k = room * cell / (cell + METER);
        return (k, room - k);
    }
    // The places of the line: from the fewest meters that leave the keys no more than all of them to
    // the most that leave them a key's cell
    let most = (((room - cell) / METER + EPS).floor() as usize).min(m);
    let fewest = (((room - keys) / METER - EPS).ceil().max(1.0) as usize).min(most);
    let shown = match divider {
        Some(share) => ((1.0 - share) * room / METER).round().max(0.0) as usize,
        None => most,
    }
    .clamp(fewest, most);
    (room - shown as f64 * METER, shown as f64 * METER)
}

/// The room the keys and the meters share in a pill: `room_for`, and no more than they hold and than
/// the work area leaves them beside the handle, the divider and the foot with the plus (see
/// `pill_length`). With keys and meters, cut back to the longest room in which the line can stand on
/// a whole meter with a key's cell or more before it and no room empty after the last key (`parts`):
/// only a single key with a room between two such places is cut. The signs add their length to the
/// pill and take nothing from this room, unless the pill with them would run past the work area:
/// then the room is cut by what does not fit and no more, and with too little left for that place of
/// the line the keys keep a key's cell and what is left after it.
pub fn room(work: (i32, i32, i32, i32), k: f64, cell: f64, n: usize, m: usize, edge: Edge, signs: bool, length: Option<usize>) -> f64 {
    let (fixed, screen) = frame(work, k, m, edge, false);
    let room = room_for(n, m, cell, length).min(n as f64 * cell + m as f64 * METER).min(screen - fixed).max(0.0);
    let room = if m == 0 || n == 0 {
        room
    } else {
        // The most meters that leave a key's cell, and the keys no more than all of them before them
        (1..=m).rev().find(|&shown| room + EPS >= shown as f64 * METER + cell).map_or(room, |shown| room.min(shown as f64 * METER + n as f64 * cell))
    };
    if !signs {
        return room;
    }
    let (fixed, screen) = frame(work, k, m, edge, true);
    room.min(screen - fixed).max(0.0)
}

/// What a pill has besides the keys and the meters, and the most the work area lets it be.
fn frame(work: (i32, i32, i32, i32), k: f64, m: usize, edge: Edge, signs: bool) -> (f64, f64) {
    let more = if signs { ADD + STRETCH } else { 0.0 };
    let span = if edge == Edge::Top { work.2 } else { work.3 };
    let fixed = GRIP + ADD + END + more + if m > 0 { LINE } else { 0.0 };
    (fixed, (span as f64 / k - 2.0 * ROOM - MARGIN).max(GRIP + ADD + END + more))
}

/// Length of the pill along its edge, in CSS px: the handle, the room its keys and meters share
/// (`room`), the divider, and the foot, one sign and the end grip longer while the two signs show in
/// place of the plus (`signs`); no longer than fits in the work area. Where the divider stands does
/// not change it. `work` is the work area `(x, y, width, height)` in physical px, `k` is physical px
/// per CSS px. The page draws the pill this long and the window is sized for it, so neither waits
/// for the other when the signs open or close. `m` is the number of meters under the keys, `length`
/// the cells the capsule was stretched to (`keys_length`).
pub fn pill_length(work: (i32, i32, i32, i32), k: f64, cell: f64, n: usize, m: usize, edge: Edge, signs: bool, length: Option<usize>) -> f64 {
    let (fixed, screen) = frame(work, k, m, edge, signs);
    (fixed + room(work, k, cell, n, m, edge, signs, length)).min(screen)
}

/// Where the capsule window goes, in physical px: `(x, y, width, height)`.
/// `work` is the work area `(x, y, width, height)` in physical px, `k` is physical px per CSS px
/// (monitor scale × panel scale). `along` is where the pill starts along its edge, as a share of
/// the work area: its top edge for an upright capsule, its left end for a flat one.
/// The pill is as long as `pill_length` says. Where that is more than fits between `along` and the
/// end of the work area, the window is pulled back along the edge until the whole pill shows; only a
/// pill longer than the work area itself is cut short, and then the list scrolls inside it.
/// `signs` says that the two signs show in the foot in place of the plus; `m` is the number of meters,
/// `length` the cells the capsule was stretched to.
pub fn window_rect_with(work: (i32, i32, i32, i32), k: f64, along: f64, cell: f64, n: usize, m: usize, edge: Edge, signs: bool, length: Option<usize>) -> (i32, i32, i32, i32) {
    let (wx, wy, ww, wh) = work;
    let along = if along.is_finite() { along.clamp(0.0, 1.0) } else { 0.0 };
    let long = ((pill_length(work, k, cell, n, m, edge, signs, length) + 2.0 * ROOM) * k).round() as i32;
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
    window_rect_with(work, k, along, cell, n, 0, edge, false, None)
}

/// How far the end grip may stretch the capsule, and from where. The pill's start stays where it
/// stands with the signs up, so the handle does not move under a stretch; from there the pill may
/// grow by whole cells until, signs and all, it would pass the end of the work area, be cut short
/// by `pill_length`, or show more than its keys and meters (`room`): `most` is the fewest cells
/// that pill takes, and never fewer than `LEAST`. Returns `(along, most)`: `along` is where the pill
/// starts as a share of the work area, the `along` given unless the window stands pulled back
/// against the end of the work area.
pub fn reach(work: (i32, i32, i32, i32), k: f64, along: f64, cell: f64, n: usize, m: usize, edge: Edge, length: Option<usize>) -> (f64, usize) {
    let window = window_rect_with(work, k, along, cell, n, m, edge, true, length);
    let (start, long, end, span) = if edge == Edge::Top {
        (window.0, window.2, work.0 + work.2, work.2)
    } else {
        (window.1, window.3, work.1 + work.3, work.3)
    };
    let pill = |cells| pill_length(work, k, cell, n, m, edge, true, Some(cells));
    let fits = |cells| ((pill(cells) + 2.0 * ROOM) * k).round() as i32 <= end - start;
    // A cell more than the work area is long can only be cut short
    let bound = (span as f64 / k / cell) as usize + LEAST;
    let mut most = LEAST;
    while most < bound && fits(most + 1) {
        most += 1;
    }
    while most > LEAST && pill(most - 1) >= pill(most) {
        most -= 1;
    }
    let along = if start + long == end { along_from(work, k, window, edge) } else { along };
    (along, most)
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
/// cut-out whose key has its middle where the fold sign is, the last sign of the foot before the
/// end grip. It may reach past the work area; `along_from` makes the share of it and
/// `folded_window` keeps it inside.
pub fn fold_spot(window: (i32, i32, i32, i32), k: f64, edge: Edge) -> (i32, i32, i32, i32) {
    let (x, y, w, h) = window;
    let (thick, long) = ((WIDTH * k).round() as i32, ((FOLDED + 2.0 * ROOM) * k).round() as i32);
    // From the end of the window back past the end grip to the sign's middle, and half the cut-out further
    let back = ((ROOM + END + STRETCH + ADD / 2.0) * k).round() as i32 + long / 2;
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

/// Where a window of `size` stands beside the pill at `pill`, all in physical px, as `(x, y,
/// width, height)`: on the side away from the edge, `gap` from the pill, level with its start, and
/// whole inside the work area; a window larger than the work area is cut to it.
pub fn beside(pill: (i32, i32, i32, i32), size: (i32, i32), work: (i32, i32, i32, i32), gap: i32, edge: Edge) -> (i32, i32, i32, i32) {
    let ((px, py, pw, ph), (wx, wy, ww, wh)) = (pill, work);
    let (w, h) = (size.0.min(ww), size.1.min(wh));
    let (x, y) = match edge {
        Edge::Right => (px - gap - w, py),
        Edge::Left => (px + pw + gap, py),
        Edge::Top => (px, py + ph + gap),
    };
    (x.min(wx + ww - w).max(wx), y.min(wy + wh - h).max(wy), w, h)
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
                        // The two signs: the capsule is one sign and the end grip longer, whole, and where it was
                        // unless the end of the work area is in the way; there it is pulled back
                        // along the edge, not cut short
                        let rest = window_rect(work, k, along, cell, 6, edge);
                        let open = window_rect_with(work, k, along, cell, 6, 0, edge, true, None);
                        assert!(inside(open, work), "{what}: {open:?}");
                        let longer = (span(open).1 - span(open).0) - (span(rest).1 - span(rest).0);
                        assert!((longer - ((ADD + STRETCH) * k).round() as i32).abs() <= 1, "{what}: longer by {longer}");
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
                        let sign = span(open).1 - ((ROOM + END + STRETCH + ADD / 2.0) * k).round() as i32;
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
                    let rest = pill_length(work, k, cell, n, 0, edge, false, None);
                    let open = pill_length(work, k, cell, n, 0, edge, true, None);
                    // One sign and the end grip longer, unless the work area has no room for them
                    let fits = pill_height(n, cell, None) + ADD + STRETCH <= span(work).1 as f64 / k - 2.0 * ROOM - MARGIN;
                    assert!((open - rest - ADD - STRETCH).abs() < 1e-9 || !fits, "{edge:?} {work:?} {k}: {rest} -> {open}");
                    for along in [0.0, 0.3, 0.62, 0.97] {
                        let what = format!("{edge:?} {work:?} k {k} along {along} cell {cell} n {n}");
                        let at_rest = window_rect_with(work, k, along, cell, n, 0, edge, false, None);
                        let with_signs = window_rect_with(work, k, along, cell, n, 0, edge, true, None);
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
    fn meters_lengthen_the_pill_by_their_block_and_no_meters_change_nothing() {
        let work = (0, 0, 2880, 1824);
        for edge in [Edge::Right, Edge::Left, Edge::Top] {
            for n in [0, 3, 8] {
                let plain = pill_length(work, 2.0, CELL, n, 0, edge, false, None);
                assert_eq!(plain, pill_height(n, CELL, None));
                // With meters the keys' room has no peek of the next key on top of the cells
                let keys = plain - if n > SHOWN { PEEK } else { 0.0 };
                assert_eq!(pill_length(work, 2.0, CELL, n, 1, edge, false, None), keys + LINE + METER);
                assert_eq!(pill_length(work, 2.0, CELL, n, 2, edge, true, None), keys + LINE + 2.0 * METER + ADD + STRETCH);
                // Not stretched, the capsule shows every meter under its keys, as long as the screen lets it
                assert_eq!(pill_length(work, 2.0, CELL, n, 3, edge, false, None), keys + LINE + 3.0 * METER);
                assert_eq!(pill_length(work, 2.0, CELL, n, 9, edge, false, None), keys + LINE + 9.0 * METER);
            }
        }
        // No meters: the window stands where it stood before them, worked out by hand at 200 %
        // and 0.3 along. Its length is the pill (handle 44, the keys that show, the peek, the foot
        // 38 + 3) and the room for the shadow at each end, 12; upright it is 78 + 270 wide, flat
        // 78 + 96 high
        for (n, long) in [(0, (44.0 + 41.0 + 24.0) * 2.0), (3, (44.0 + 3.0 * 54.0 + 41.0 + 24.0) * 2.0), (8, (44.0 + 4.0 * 54.0 + 18.0 + 41.0 + 24.0) * 2.0)] {
            let long = long as i32;
            assert_eq!(window_rect_with(work, 2.0, 0.3, CELL, n, 0, Edge::Right, false, None), (2880 - 696, 523, 696, long), "{n}");
            assert_eq!(window_rect_with(work, 2.0, 0.3, CELL, n, 0, Edge::Left, false, None), (0, 523, 696, long), "{n}");
            assert_eq!(window_rect_with(work, 2.0, 0.3, CELL, n, 0, Edge::Top, false, None), (840, 0, long, 348), "{n}");
        }
        // A screen too short for them all: the pill fits the work area, the lists scroll inside it
        let low = (0, 0, 640, 500);
        assert_eq!(pill_length(low, 2.0, CELL, 8, 5, Edge::Right, false, None), 250.0 - 2.0 * ROOM - MARGIN);
    }

    #[test]
    fn a_stretched_capsule_has_its_cells_whatever_the_keys_and_none_is_as_before() {
        // No length: up to four keys, the peek of the fifth, as it always was
        for n in [0, 1, 3, 4, 5, 12] {
            assert_eq!(keys_length(n, CELL, None), n.min(SHOWN) as f64 * CELL + if n > SHOWN { PEEK } else { 0.0 }, "{n}");
        }
        // A length: that many cells with few keys too, the peek only with more keys than cells
        assert_eq!(keys_length(3, CELL, Some(6)), 6.0 * CELL);
        assert_eq!(keys_length(6, CELL, Some(6)), 6.0 * CELL);
        assert_eq!(keys_length(8, CELL, Some(6)), 6.0 * CELL + PEEK);
        assert!((keys_length(8, CELL * TABLET_STEP, Some(3)) - (3.0 * CELL * TABLET_STEP + PEEK)).abs() < 1e-9);
        // Never fewer than two cells, whatever the file says
        assert_eq!(keys_length(5, CELL, Some(0)), keys_length(5, CELL, Some(LEAST)));
        assert_eq!(keys_length(5, CELL, Some(1)), 2.0 * CELL + PEEK);
    }

    #[test]
    fn the_window_of_a_stretched_capsule_is_its_pill_inside_the_work_area() {
        // A tablet at 200 %, lying and standing, a screen too short, the three sizes of the panel
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 1600, 560)];
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            for work in works {
                let (wx, wy, ww, wh) = work;
                for edge in [Edge::Right, Edge::Left, Edge::Top] {
                    let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { r.2 } else { r.3 };
                    for cell in [CELL, CELL * TABLET_STEP] {
                        for (n, m) in [(3, 0), (9, 0), (9, 3)] {
                            for length in [Some(2), Some(5), Some(9), Some(40)] {
                                for along in [0.0, 0.3, 0.97] {
                                    for signs in [false, true] {
                                        let what = format!("{edge:?} {work:?} k {k} cell {cell} n {n} m {m} {length:?} along {along} signs {signs}");
                                        let window = window_rect_with(work, k, along, cell, n, m, edge, signs, length);
                                        let (x, y, w, h) = window;
                                        assert!(x >= wx && y >= wy && x + w <= wx + ww && y + h <= wy + wh, "{what}: {window:?}");
                                        // The window is the pill the page draws and the room before and after it
                                        let pill = pill_length(work, k, cell, n, m, edge, signs, length);
                                        assert_eq!(span(window), ((pill + 2.0 * ROOM) * k).round() as i32, "{what}");
                                        let (px, py, pw, _) = pill_on_screen(window, k, edge);
                                        match edge {
                                            Edge::Right => assert_eq!(px + pw, wx + ww, "{what}"),
                                            Edge::Left => assert_eq!(px, wx, "{what}"),
                                            Edge::Top => assert_eq!(py, wy, "{what}"),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // As long as asked when it fits and there are keys for it: a handle, five cells, the foot,
        // worked out by hand; with three keys only three cells, and no empty ones
        assert_eq!(pill_length(WORK, 2.0, CELL, 7, 0, Edge::Right, false, Some(5)), GRIP + 5.0 * CELL + PEEK + ADD + END);
        assert_eq!(pill_length(WORK, 2.0, CELL, 5, 0, Edge::Right, true, Some(5)), GRIP + 5.0 * CELL + 2.0 * ADD + STRETCH + END);
        assert_eq!(pill_length(WORK, 2.0, CELL, 3, 0, Edge::Right, false, Some(5)), GRIP + 3.0 * CELL + ADD + END);
    }

    #[test]
    fn the_end_grip_stretches_from_two_cells_to_what_fits_and_the_handle_stays() {
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 640, 560)];
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            for work in works {
                for edge in [Edge::Right, Edge::Left, Edge::Top] {
                    let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { (r.0, r.0 + r.2) } else { (r.1, r.1 + r.3) };
                    let work_end = span(work).1;
                    for cell in [CELL, CELL * TABLET_STEP] {
                        for (n, m) in [(3, 0), (9, 2)] {
                            for length in [None, Some(2), Some(7)] {
                                for along in [0.0, 0.3, 0.62, 0.97] {
                                    let what = format!("{edge:?} {work:?} k {k} cell {cell} n {n} m {m} {length:?} along {along}");
                                    let (at, most) = reach(work, k, along, cell, n, m, edge, length);
                                    assert!(most >= LEAST, "{what}");
                                    // Where the pill stands with the signs up before the stretch
                                    let before = span(window_rect_with(work, k, along, cell, n, m, edge, true, length));
                                    for cells in LEAST..=most {
                                        let now = span(window_rect_with(work, k, at, cell, n, m, edge, true, Some(cells)));
                                        // The handle does not move, and the whole pill shows
                                        assert_eq!(now.0, before.0, "{what}: {cells} cells");
                                        assert!(now.1 <= work_end, "{what}: {cells} cells");
                                        // Nor when the signs go after the release
                                        let rest = span(window_rect_with(work, k, at, cell, n, m, edge, false, Some(cells)));
                                        assert_eq!(rest.0, before.0, "{what}: {cells} cells at rest");
                                    }
                                    // One cell more would run past the work area, or show nothing more:
                                    // the keys and meters all show, or the pill is cut short
                                    let longest = pill_length(work, k, cell, n, m, edge, true, Some(most));
                                    let longer = pill_length(work, k, cell, n, m, edge, true, Some(most + 1));
                                    assert!(longer <= longest || before.0 + ((longer + 2.0 * ROOM) * k).round() as i32 > work_end, "{what}: {most}");
                                    // ...and the fewest cells give that pill
                                    assert!(most == LEAST || pill_length(work, k, cell, n, m, edge, true, Some(most - 1)) < longest, "{what}: {most}");
                                }
                            }
                        }
                    }
                }
            }
        }
        // Not pulled back: the place is the one given; at the end of the screen it is where it stands
        assert_eq!(reach(WORK, 2.0, 0.3, CELL, 3, 0, Edge::Right, None).0, 0.3);
        let (at, most) = reach(WORK, 2.0, 0.97, CELL, 3, 0, Edge::Right, None);
        assert!(at < 0.97 && most == 3, "{at} {most}");
        // High on a tall screen there is room for many cells, as many as there are keys; a short
        // screen still has two
        assert!(reach(WORK, 2.0, 0.0, CELL, 20, 0, Edge::Right, None).1 >= 12);
        assert_eq!(reach(WORK, 2.0, 0.0, CELL, 5, 0, Edge::Right, None).1, 5);
        assert_eq!(reach((0, 0, 640, 400), 2.0, 0.0, CELL, 3, 0, Edge::Right, None).1, LEAST);
    }

    #[test]
    fn a_capsule_longer_than_its_keys_and_meters_shows_no_empty_cells() {
        // Stretched to eight cells with three keys, with and without meters: the pill is the
        // handle, the keys, the divider, every meter and the foot, nothing more
        for m in [0, 1, 3] {
            let pill = pill_length(WORK, 2.0, CELL, 3, m, Edge::Right, false, Some(8));
            let line = if m > 0 { LINE } else { 0.0 };
            assert_eq!(pill, GRIP + 3.0 * CELL + line + m as f64 * METER + ADD + END, "{m}");
            let window = window_rect_with(WORK, 2.0, 0.3, CELL, 3, m, Edge::Right, false, Some(8));
            assert_eq!(window.3, ((pill + 2.0 * ROOM) * 2.0).round() as i32, "{m}");
            // All of it is the keys and the meters: the line has one place
            let room = super::room(WORK, 2.0, CELL, 3, m, Edge::Right, false, Some(8));
            for divider in [None, Some(0.0), Some(0.5), Some(1.0)] {
                assert_eq!(parts(room, 3, m, CELL, divider), (3.0 * CELL, m as f64 * METER), "{m} {divider:?}");
            }
        }
    }

    #[test]
    fn the_divider_moves_a_whole_meter_at_a_time_and_the_pill_keeps_its_length() {
        // A tablet at 200 %, lying and standing, and a screen too short for the whole capsule
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 640, 700), (0, 0, 640, 560)];
        for work in works {
            for edge in [Edge::Right, Edge::Top] {
                for cell in [CELL, CELL * TABLET_STEP] {
                    for (n, m) in [(1, 1), (3, 2), (10, 1), (10, 3), (10, 10), (2, 7)] {
                        for length in [None, Some(2), Some(5), Some(8), Some(40)] {
                            for signs in [false, true] {
                                let what = format!("{work:?} {edge:?} cell {cell} n {n} m {m} {length:?} signs {signs}");
                                let room = super::room(work, 2.0, cell, n, m, edge, signs, length);
                                let pill = pill_length(work, 2.0, cell, n, m, edge, signs, length);
                                let fixed = GRIP + LINE + ADD + END + if signs { ADD + STRETCH } else { 0.0 };
                                for step in 0..=20 {
                                    let divider = Some(step as f64 / 20.0);
                                    let (keys, meters) = parts(room, n, m, cell, divider);
                                    // Wherever the line stands, the parts fill the same pill
                                    assert!((fixed + keys + meters - pill).abs() < 1e-6, "{what} {divider:?}: {keys} {meters}");
                                    if room < cell + METER {
                                        continue;
                                    }
                                    // Whole meters, one at least and no more than there are
                                    let shown = meters / METER;
                                    assert!((shown - shown.round()).abs() < 1e-6, "{what} {divider:?}: {meters}");
                                    assert!(shown.round() >= 1.0 && shown.round() <= m as f64, "{what} {divider:?}: {meters}");
                                    // A key's cell at least, and nothing empty after the last key, unless the
                                    // signs cut the room at the end of the work area
                                    assert!(keys + 1e-6 >= cell, "{what} {divider:?}: {keys}");
                                    let cut = signs && room < super::room(work, 2.0, cell, n, m, edge, false, length) - 1e-9;
                                    assert!(cut || keys <= n as f64 * cell + 1e-6, "{what} {divider:?}: {keys}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_signs_lengthen_the_pill_by_their_own_length_or_up_to_the_end_of_the_work_area() {
        // Ten keys, one meter, eight cells on a work area 950 px high at 200 %: with the signs the room
        // was once cut again to fewer whole keys, and the pill got shorter than without them
        let low = (0, 0, 1600, 950);
        assert!(pill_length(low, 2.0, CELL, 10, 1, Edge::Right, true, Some(8)) >= pill_length(low, 2.0, CELL, 10, 1, Edge::Right, false, Some(8)));
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 1600, 560), low, (0, 0, 1600, 1100)];
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            for work in works {
                for edge in [Edge::Right, Edge::Left, Edge::Top] {
                    let screen = frame(work, k, 0, edge, true).1;
                    for cell in [CELL, CELL * TABLET_STEP] {
                        for (n, m) in [(3, 0), (9, 0), (9, 3), (10, 1), (10, 2), (2, 7)] {
                            for length in [None, Some(2), Some(5), Some(8), Some(40)] {
                                let what = format!("{edge:?} {work:?} k {k} cell {cell} n {n} m {m} {length:?}");
                                let closed = pill_length(work, k, cell, n, m, edge, false, length);
                                let open = pill_length(work, k, cell, n, m, edge, true, length);
                                assert!(open + 1e-9 >= closed, "{what}: {closed} -> {open}");
                                if closed + ADD + STRETCH <= screen {
                                    assert!((open - closed - ADD - STRETCH).abs() < 1e-9, "{what}: {closed} -> {open}");
                                } else {
                                    assert!((open - screen).abs() < 1e-9, "{what}: {closed} -> {open}, {screen}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_line_stands_on_the_meter_nearest_to_where_it_was_let_go() {
        // The owner's capsule: eight keys and three meters in eight cells. All three meters show
        // whole, the keys scroll in the rest and the edge of the next one fades out
        let room = super::room(WORK, 2.0, CELL, 8, 3, Edge::Right, false, Some(8));
        assert_eq!(room, 8.0 * CELL);
        assert_eq!(parts(room, 8, 3, CELL, None), (room - 3.0 * METER, 3.0 * METER));
        assert_eq!(pill_length(WORK, 2.0, CELL, 8, 3, Edge::Right, false, Some(8)), GRIP + room + LINE + ADD + END);
        // Pulled down, one meter fewer a step, down to one; pulled up, back to all three
        let at = |shown: f64| Some((room - shown * METER) / room);
        assert_eq!(parts(room, 8, 3, CELL, at(2.0)), (room - 2.0 * METER, 2.0 * METER));
        assert_eq!(parts(room, 8, 3, CELL, Some(1.0)), (room - METER, METER));
        assert_eq!(parts(room, 8, 3, CELL, Some(0.0)), (room - 3.0 * METER, 3.0 * METER));
        // A share between two meters goes to the nearer one; a share saved before the steps too
        assert_eq!(parts(room, 8, 3, CELL, at(1.4)).1, METER);
        assert_eq!(parts(room, 8, 3, CELL, at(1.6)).1, 2.0 * METER);
        assert_eq!(parts(room, 8, 3, CELL, Some(0.6124)).1, 3.0 * METER);
        // Ten meters in seven cells: as many meters as leave a key's cell, the line from one to them
        let short = super::room(WORK, 2.0, CELL, 10, 10, Edge::Right, false, Some(7));
        assert_eq!(short, 7.0 * CELL);
        assert_eq!(parts(short, 10, 10, CELL, None), (short - 6.0 * METER, 6.0 * METER));
        assert_eq!(parts(short, 10, 10, CELL, Some(1.0)).1, METER);
        // Three keys and three meters with room for all: one place, as no key's room may stay empty
        let all = super::room(WORK, 2.0, CELL, 3, 3, Edge::Right, false, Some(8));
        assert_eq!(all, 3.0 * CELL + 3.0 * METER);
        assert_eq!(parts(all, 3, 3, CELL, Some(1.0)), (3.0 * CELL, 3.0 * METER));
        // Not stretched: up to four keys and every meter
        let plain = super::room(WORK, 2.0, CELL, 6, 8, Edge::Right, false, None);
        assert_eq!(parts(plain, 6, 8, CELL, None), (4.0 * CELL, 8.0 * METER));
        // A screen too short for everything: the room is what it leaves, shared the same way
        let low = (0, 0, 640, 700);
        let tight = super::room(low, 2.0, CELL, 10, 3, Edge::Right, false, Some(8));
        assert_eq!(tight, 350.0 - 2.0 * ROOM - MARGIN - (GRIP + LINE + ADD + END));
        assert_eq!(pill_length(low, 2.0, CELL, 10, 3, Edge::Right, false, Some(8)), 350.0 - 2.0 * ROOM - MARGIN);
        let (keys, meters) = parts(tight, 10, 3, CELL, None);
        assert_eq!((keys, meters), (tight - 3.0 * METER, 3.0 * METER));
        // ...cut back only for a single key, to a room the line can share without an empty end
        assert_eq!(super::room(WORK, 2.0, CELL, 1, 3, Edge::Right, false, Some(3)), CELL + 2.0 * METER);
        assert_eq!(super::room(WORK, 2.0, CELL, 10, 1, Edge::Right, false, Some(8)), 8.0 * CELL);
    }

    #[test]
    fn the_end_grip_reaches_a_capsule_that_shows_every_key_and_every_meter_whole() {
        // The owner's capsule: eight keys, three meters, at the right edge of a tablet at 200 %,
        // 0.1436 down. The last step of the grip shows all of them, nothing cut, and it fits
        let (along, most) = reach(WORK, 2.0, 0.1436, CELL, 8, 3, Edge::Right, Some(11));
        let room = super::room(WORK, 2.0, CELL, 8, 3, Edge::Right, true, Some(most));
        assert_eq!(room, 8.0 * CELL + 3.0 * METER);
        assert_eq!(parts(room, 8, 3, CELL, None), (8.0 * CELL, 3.0 * METER));
        assert_eq!(most, 11);
        let window = window_rect_with(WORK, 2.0, along, CELL, 8, 3, Edge::Right, true, Some(most));
        assert!(window.1 >= 0 && window.1 + window.3 <= WORK.3, "{window:?}");
        // Two keys and seven meters the same
        let (_, most) = reach(WORK, 2.0, 0.1, CELL, 2, 7, Edge::Right, None);
        let room = super::room(WORK, 2.0, CELL, 2, 7, Edge::Right, true, Some(most));
        assert_eq!(parts(room, 2, 7, CELL, None), (2.0 * CELL, 7.0 * METER));
        // Wherever the work area has room for the whole pill from where it starts, the last step
        // shows every key and meter
        let works = [(0, 0, 2880, 1824), (0, 40, 1920, 2744), (0, 0, 1600, 950), (0, 0, 1600, 560)];
        for scale in [0.8, 1.0, 1.25] {
            let k = 2.0 * scale;
            for work in works {
                for edge in [Edge::Right, Edge::Left, Edge::Top] {
                    let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { (r.0, r.0 + r.2) } else { (r.1, r.1 + r.3) };
                    for cell in [CELL, CELL * TABLET_STEP] {
                        for (n, m) in [(8, 3), (3, 2), (10, 1), (2, 7), (6, 0), (1, 12)] {
                            for along in [0.0, 0.1436, 0.3, 0.62] {
                                for length in [None, Some(2), Some(11)] {
                                    let what = format!("{edge:?} {work:?} k {k} cell {cell} n {n} m {m} along {along} {length:?}");
                                    let (_, most) = reach(work, k, along, cell, n, m, edge, length);
                                    let all = n as f64 * cell + m as f64 * METER;
                                    let fixed = GRIP + ADD + END + ADD + STRETCH + if m > 0 { LINE } else { 0.0 };
                                    let start = span(window_rect_with(work, k, along, cell, n, m, edge, true, length)).0;
                                    let fits = start + ((fixed + all + 2.0 * ROOM) * k).round() as i32 <= span(work).1 && fixed + all <= frame(work, k, m, edge, true).1;
                                    let room = super::room(work, k, cell, n, m, edge, true, Some(most));
                                    assert!(!fits || room + 1e-6 >= all, "{what}: {most} cells, room {room} of {all}");
                                    if fits && m > 0 {
                                        let (keys, meters) = parts(room, n, m, cell, None);
                                        assert!((keys - n as f64 * cell).abs() < 1e-6 && (meters - m as f64 * METER).abs() < 1e-6, "{what}: {keys} {meters}");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn any_count_of_keys_and_meters_reaches_all_of_them_or_fills_the_work_area() {
        // Every count from one key to ten and from no meter to ten, at three scales, upright and flat
        let works = [(0, 0, 2880, 1824), (0, 0, 1920, 1080)];
        for k in [1.0, 1.5, 2.0] {
            for work in works {
                for edge in [Edge::Right, Edge::Top] {
                    let span = |r: (i32, i32, i32, i32)| if edge == Edge::Top { (r.0, r.0 + r.2) } else { (r.1, r.1 + r.3) };
                    for n in 1..=10 {
                        for m in 0..=10 {
                            for along in [0.0, 0.3] {
                                let what = format!("{edge:?} {work:?} k {k} n {n} m {m} along {along}");
                                let (at, most) = reach(work, k, along, CELL, n, m, edge, None);
                                let all = n as f64 * CELL + m as f64 * METER;
                                let line = if m > 0 { LINE } else { 0.0 };
                                let open = |cells| pill_length(work, k, CELL, n, m, edge, true, Some(cells));
                                let start = span(window_rect_with(work, k, at, CELL, n, m, edge, true, Some(most))).0;
                                let end = |cells| start + ((open(cells) + 2.0 * ROOM) * k).round() as i32;
                                assert!(end(most) <= span(work).1, "{what}: {most} cells run past the work area");
                                let room = super::room(work, k, CELL, n, m, edge, true, Some(most));
                                let fits = start + ((GRIP + ADD + END + ADD + STRETCH + line + all + 2.0 * ROOM) * k).round() as i32 <= span(work).1
                                    && GRIP + ADD + END + ADD + STRETCH + line + all <= frame(work, k, m, edge, true).1;
                                if fits {
                                    // (a) the last step shows every key and meter whole
                                    assert!(room + 1e-6 >= all, "{what}: {most} cells, room {room} of {all}");
                                    if m > 0 {
                                        assert_eq!(parts(room, n, m, CELL, None), (n as f64 * CELL, m as f64 * METER), "{what}");
                                    }
                                } else {
                                    // (b) one cell more would run past the work area or draw the same
                                    // pill: the pill fills what there is, and the parts scroll
                                    assert!(room < all, "{what}");
                                    assert!(end(most + 1) > span(work).1 || open(most + 1) <= open(most), "{what}: {most}");
                                }
                                for cells in [LEAST, (LEAST + most) / 2, most] {
                                    for signs in [false, true] {
                                        let room = super::room(work, k, CELL, n, m, edge, signs, Some(cells));
                                        let pill = pill_length(work, k, CELL, n, m, edge, signs, Some(cells));
                                        let fixed = GRIP + ADD + END + line + if signs { ADD + STRETCH } else { 0.0 };
                                        let cut = signs && room < super::room(work, k, CELL, n, m, edge, false, Some(cells)) - 1e-9;
                                        for step in 0..=10 {
                                            let divider = Some(step as f64 / 10.0);
                                            let (keys, meters) = parts(room, n, m, CELL, divider);
                                            // (c) the same pill wherever the line stands
                                            assert!((fixed + keys + meters - pill).abs() < 1e-6, "{what} {cells} {signs} {divider:?}");
                                            // (d) whole meters, none cut at the line and nothing empty under the
                                            // last; a key's cell at least and nothing empty after the last key
                                            if m > 0 && room >= CELL + METER {
                                                let shown = meters / METER;
                                                assert!((shown - shown.round()).abs() < 1e-6 && shown.round() >= 1.0 && shown.round() <= m as f64, "{what} {cells} {signs} {divider:?}: {meters}");
                                                assert!(keys + 1e-6 >= CELL && (cut || keys <= n as f64 * CELL + 1e-6), "{what} {cells} {signs} {divider:?}: {keys}");
                                            }
                                        }
                                        // Every number of meters the line offers is reachable, one a step, and it
                                        // offers all of them whenever the room has a key's cell besides
                                        if m > 0 && !signs && room >= CELL + METER {
                                            let shown = |share: f64| (parts(room, n, m, CELL, Some(share)).1 / METER).round() as usize;
                                            let (fewest, most) = (shown(1.0), shown(0.0));
                                            for j in fewest..=most {
                                                assert_eq!(shown((room - j as f64 * METER) / room), j, "{what} {cells}");
                                            }
                                            assert!(most == m || room < CELL + m as f64 * METER - 1e-6, "{what} {cells}: {most} of {m}");
                                        }
                                    }
                                }
                            }
                        }
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
    fn a_window_beside_the_pill_stands_away_from_the_edge_and_whole_on_the_screen() {
        let size = (800, 880);
        // Upright at the right and left edges: beside the pill, level with its top
        assert_eq!(beside((2760, 547, 120, 600), size, WORK, 16, Edge::Right), (2760 - 16 - 800, 547, 800, 880));
        assert_eq!(beside((0, 547, 120, 600), size, WORK, 16, Edge::Left), (136, 547, 800, 880));
        // Low on the screen: pulled up until it ends with the work area
        assert_eq!(beside((2760, 1400, 120, 400), size, WORK, 16, Edge::Right), (1944, 1824 - 880, 800, 880));
        // Flat at the top: under the pill, from its left end, pulled back from the right end
        assert_eq!(beside((840, 0, 600, 120), size, WORK, 16, Edge::Top), (840, 136, 800, 880));
        assert_eq!(beside((2400, 0, 400, 120), size, WORK, 16, Edge::Top), (2880 - 800, 136, 800, 880));
        // Taller than a short work area, at a high scale: as tall as the work area, from its top
        let short = (0, 40, 1280, 600);
        assert_eq!(beside((1160, 200, 120, 300), (840, 920), short, 24, Edge::Right), (1160 - 24 - 840, 40, 840, 600));
        // Wider than the work area too: as wide as it
        assert_eq!(beside((0, 200, 120, 300), (1400, 400), short, 24, Edge::Left), (0, 200, 1280, 400));
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
