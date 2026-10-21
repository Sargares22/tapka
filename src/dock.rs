//! Docking: panels of other programs the user chose stand in a column under the capsule and go
//! along with it. Pure: where each one goes, and when one that keeps going back is let be. The
//! windows themselves are looked at and moved in `win` and `main`.

use crate::config::Docked;
use crate::layout::Edge;
use crate::win::{TopWindow, WindowId};
use std::time::{Duration, Instant};

/// Between the capsule and the first panel, and between panels, in logical px.
pub const GAP: f64 = 8.0;
/// A window this far from the capsule's edge of the work area, in logical px, stands at it.
pub const SLACK: f64 = 32.0;
/// A placed window moved by something other than the panel twice within this time fights back.
pub const FIGHT: Duration = Duration::from_secs(10);
/// A move is handed to the window's own thread; until it shows, the window is waited for this long.
pub const SETTLE: Duration = Duration::from_secs(2);
/// After this many moves Windows refused, or that never took, the window is let be.
pub const TRIES: u32 = 3;
/// Two places this close, in physical px, are the same place: a window's own rounding of its
/// scale may move it by a pixel.
const NEAR: i32 = 2;

/// `(x, y, width, height)` in physical px.
pub type Rect = (i32, i32, i32, i32);

fn near(a: (i32, i32), b: (i32, i32)) -> bool {
    (a.0 - b.0).abs() <= NEAR && (a.1 - b.1).abs() <= NEAR
}

/// Where each docked window goes: the corner of the window, in physical px. `pill` is the pill as
/// it is drawn on the screen, not the capsule window, which has room for the shadow at each end.
/// `parts` says, for each window, where in it the part a person sees is drawn, `(x, y, width,
/// height)` from its corner. Those parts stand flush to the capsule's edge, `gap` apart, in the
/// order of `parts`: in a column right under the pill, and those that do not fit there in a column
/// right above it, growing upwards; at the top edge, in a row to the right of the pill, then to its
/// left. Only the part has to fit in the work area: the transparent rest of the window may run past
/// it. A window whose part fits on neither side gets `None` and is left where it is; the next one
/// tries the same places.
pub fn column(pill: Rect, edge: Edge, work: Rect, gap: i32, parts: &[Rect]) -> Vec<Option<(i32, i32)>> {
    let (wx, wy, ww, wh) = work;
    let top = edge == Edge::Top;
    // Along the edge: where the next part may start after the pill, and where the next one before
    // it may end
    let (mut after, mut before) = if top { (pill.0 + pill.2 + gap, pill.0 - gap) } else { (pill.1 + pill.3 + gap, pill.1 - gap) };
    let at = |along: i32, w: i32| match edge {
        Edge::Right => (wx + ww - w, along),
        Edge::Left => (wx, along),
        Edge::Top => (along, wy),
    };
    let fits = |(x, y): (i32, i32), w: i32, h: i32| x >= wx && y >= wy && x + w <= wx + ww && y + h <= wy + wh;
    parts
        .iter()
        .map(|&(dx, dy, w, h)| {
            let long = if top { w } else { h };
            let (under, over) = (at(after, w), at(before - long, w));
            let place = if fits(under, w, h) {
                after += long + gap;
                under
            } else if fits(over, w, h) {
                before -= long + gap;
                over
            } else {
                return None;
            };
            Some((place.0 - dx, place.1 - dy))
        })
        .collect()
}

/// Whether two rectangles cover some of the same pixels.
pub fn overlap(a: Rect, b: Rect) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// Where in a window at `window` (screen px) a person sees something: the union of its
/// elements' rectangles `drawn` (screen px), each cut to the window, as `(x, y, width, height)`
/// from the window's corner. A panel drawn in a web page is a large transparent window: the
/// page, its document and the groups that span it are as large as the window, so an element over
/// half the window's area is taken for such a frame and counts for nothing, as do empty ones and
/// those outside the window. The pill's own group counts, so its padding around the icons and
/// texts is part of what shows. With nothing left, `None`.
pub fn drawn_part(window: Rect, drawn: &[Rect]) -> Option<Rect> {
    let (wx, wy, ww, wh) = window;
    let frame = ww as i64 * wh as i64 / 2;
    let mut union: Option<(i32, i32, i32, i32)> = None;
    for &(x, y, w, h) in drawn {
        let (l, t, r, b) = (x.max(wx), y.max(wy), (x + w).min(wx + ww), (y + h).min(wy + wh));
        if w <= 0 || h <= 0 || r <= l || b <= t || (r - l) as i64 * (b - t) as i64 > frame {
            continue;
        }
        union = Some(match union {
            Some((ul, ut, ur, ub)) => (ul.min(l), ut.min(t), ur.max(r), ub.max(b)),
            None => (l, t, r, b),
        });
    }
    union.map(|(l, t, r, b)| (l - wx, t - wy, r - l, b - t))
}

/// The drawn part of a docked window is looked for again this often, and when its size changes.
pub const REFRESH: Duration = Duration::from_secs(5);

/// The part of a window a person sees, from its corner. Only a window that can be seen through,
/// layered, drawn without a redirection bitmap or cut by a window region, is shrunk to its drawn
/// elements; an opaque one shows all of itself, whatever its elements say, and so does a
/// see-through one that told nothing.
pub fn part_of(see_through: bool, window: Rect, drawn: &[Rect]) -> Rect {
    let whole = (0, 0, window.2, window.3);
    if see_through { drawn_part(window, drawn).unwrap_or(whole) } else { whole }
}

/// What is known of where a docked window draws itself (`part_of`): for which window, of which
/// process, at which size.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub id: WindowId,
    pub size: (i32, i32),
    pub part: Rect,
    pub when: Instant,
}

/// Whether a window whose visible rectangle is `rect` stands at the capsule's edge of the work
/// area: its side within `slack` of that edge, and across from some of the work area.
pub fn at_edge(rect: Rect, work: Rect, edge: Edge, slack: i32) -> bool {
    let ((x, y, w, h), (wx, wy, ww, wh)) = (rect, work);
    let across = if edge == Edge::Top { x < wx + ww && x + w > wx } else { y < wy + wh && y + h > wy };
    let side = match edge {
        Edge::Right => (x + w - (wx + ww)).abs(),
        Edge::Left => (x - wx).abs(),
        Edge::Top => (y - wy).abs(),
    };
    across && side <= slack
}

/// Where a window shows on the screen: the part it draws (`Shape`), once that is known for this
/// window at this size, else all of what shows of it.
pub fn shown_rect(window: &TopWindow, shapes: &[Shape]) -> Rect {
    let (x, y, w, h) = window.rect;
    match shapes.iter().find(|s| s.id == window.id && s.size == (w, h)) {
        Some(s) => (x + s.part.0, y + s.part.1, s.part.2, s.part.3),
        None => window.rect,
    }
}

/// What the search for a docked entry's window found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Found<'a> {
    One(&'a TopWindow),
    /// No window of it stands above the others now.
    None,
    /// Several of its windows, none told from the others: which one was meant cannot be guessed.
    Several,
}

/// The window of a docked entry among those that stay above the others. `kept` is the window the
/// entry went by so far, while it still exists (`win::still`): that one and no other, even while it
/// is out of the list, minimized or no longer above the others, since a program's other window of
/// the same path and class (a popup, a larger window) is not the panel that was ticked. Without one,
/// a window of its program by path and class or, for an old entry, by file name, that stands at the
/// capsule's edge: the only such one; with several, none. A window elsewhere on the screen is never
/// pulled in: a program's only window above the others may be something else, an overlay at the
/// bottom of the screen say, while its panel is hidden.
pub fn pick<'a>(entry: &Docked, kept: Option<WindowId>, windows: impl IntoIterator<Item = &'a TopWindow>, at_edge: impl Fn(&TopWindow) -> bool) -> Found<'a> {
    let theirs: Vec<&TopWindow> = windows.into_iter().filter(|t| entry.matches(&t.exe, &t.path, &t.class)).collect();
    if let Some(id) = kept {
        return theirs.into_iter().find(|t| t.id == id).map_or(Found::None, Found::One);
    }
    let at: Vec<&TopWindow> = theirs.into_iter().filter(|t| at_edge(t)).collect();
    match at.as_slice() {
        [] => Found::None,
        [one] => Found::One(one),
        _ => Found::Several,
    }
}

/// Why the panel lets a docked window be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Why {
    /// It went back to its own place by itself, twice.
    Returns,
    /// Windows would not let the panel move it.
    Refused,
}

/// What to do with a docked window this time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Step {
    /// It is where it should be, its move is on its way, or it is let be.
    Stay,
    /// Move it to its place.
    Place,
    /// From now on it is let be.
    Release(Why),
}

/// The last move of a docked window.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Move {
    /// Where the panel put it, where it stood before, and when.
    put: (i32, i32),
    from: (i32, i32),
    when: Instant,
    /// It was seen where it was put.
    reached: bool,
}

/// What the panel remembers of one docked window it moves.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Moved {
    last: Option<Move>,
    /// When it was found moved by something other than the panel.
    returns: Vec<Instant>,
    /// Moves Windows refused, or that never showed, one after another.
    failures: u32,
    /// The panel no longer moves it, and why.
    pub let_be: Option<Why>,
}

impl Moved {
    /// The window stands at `at` and should stand at `place`. Off its place, it is moved there. A
    /// move is handed to the window's thread and shows a moment later: while the window still
    /// stands where it was, within `SETTLE`, it is waited for; a move that never showed counts
    /// as refused. Found anywhere but where it was put, once the move took, something other than
    /// the panel moved it: its program, going back toward its own saved place, which it need not
    /// reach exactly, or a hand. The second such move within `FIGHT` lets it be, so the panel and
    /// the program do not toss it back and forth. A window left behind by the capsule's move stands
    /// where it was put, and is only put at its new place.
    pub fn step(&mut self, now: Instant, at: (i32, i32), place: (i32, i32)) -> Step {
        if self.let_be.is_some() {
            return Step::Stay;
        }
        if let Some(last) = self.last.as_mut().filter(|m| near(at, m.put)) {
            last.reached = true;
            self.failures = 0;
        }
        if near(at, place) {
            return Step::Stay;
        }
        if let Some(last) = self.last {
            if !last.reached && near(at, last.from) {
                if now.duration_since(last.when) < SETTLE {
                    return Step::Stay;
                }
                if let Some(why) = self.refused() {
                    return Step::Release(why);
                }
            } else if !near(at, last.put) {
                self.returns.retain(|&t| now.duration_since(t) <= FIGHT);
                self.returns.push(now);
                if self.returns.len() >= 2 {
                    self.let_be = Some(Why::Returns);
                    return Step::Release(Why::Returns);
                }
            }
        }
        self.last = Some(Move { put: place, from: at, when: now, reached: false });
        Step::Place
    }

    /// Windows refused the move just asked for. After `TRIES` refusals in a row the window is let
    /// be, and that is returned.
    pub fn failed(&mut self) -> Option<Why> {
        self.last = None;
        self.refused()
    }

    fn refused(&mut self) -> Option<Why> {
        self.failures += 1;
        (self.failures >= TRIES).then(|| *self.let_be.insert(Why::Refused))
    }
}

/// One docked entry as the panel follows it: the window it moves now and what it remembers of it.
#[derive(Debug, Clone)]
pub struct Watch {
    /// `Docked::key` of the entry.
    pub key: String,
    pub id: WindowId,
    pub moved: Moved,
    /// The log has said that the window fits nowhere by the capsule and stands over a docked one.
    pub told: bool,
}

/// The watch of an entry, for its window now. Another window, a restarted program's say, starts
/// afresh, even one whose window was let be: the new window stands where its program put it.
pub fn watch_for<'a>(watches: &'a mut Vec<Watch>, key: &str, id: WindowId) -> &'a mut Watch {
    let at = match watches.iter().position(|w| w.key == key) {
        Some(at) => at,
        None => {
            watches.push(Watch { key: key.to_string(), id, moved: Moved::default(), told: false });
            watches.len() - 1
        }
    };
    if watches[at].id != id {
        watches[at] = Watch { key: key.to_string(), id, moved: Moved::default(), told: false };
    }
    &mut watches[at]
}

/// The watches after the list of docked entries changed to `docked`: those of entries no longer on
/// it are gone, and the rest forget what they learned of their windows' moves, so a window let be
/// is tried again; each keeps the window it went by.
pub fn start_again(watches: &mut Vec<Watch>, docked: &[Docked]) {
    watches.retain(|w| docked.iter().any(|d| d.key() == w.key));
    for w in watches.iter_mut() {
        w.moved = Moved::default();
        w.told = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = (0, 0, 2880, 1824);

    #[test]
    fn panels_stand_flush_under_the_capsule_in_order_and_one_that_does_not_fit_is_left_alone() {
        // A pill at the right edge, 200 % scale: the gap is 16 px
        let pill = (2760, 400, 120, 700);
        let at = column(pill, Edge::Right, WORK, 16, &[(0, 0, 300, 200), (0, 0, 200, 120)]);
        assert_eq!(at, [Some((2580, 1116)), Some((2680, 1332))]);
        // Left edge: flush to the left
        let left = (0, 400, 120, 700);
        assert_eq!(column(left, Edge::Left, WORK, 16, &[(0, 0, 300, 200), (0, 0, 200, 120)]), [Some((0, 1116)), Some((0, 1332))]);
        // Too tall for the room under the capsule and above it: left alone, and the next one takes
        // its place
        let at = column(pill, Edge::Right, WORK, 16, &[(0, 0, 300, 200), (0, 0, 300, 600), (0, 0, 200, 300)]);
        assert_eq!(at, [Some((2580, 1116)), None, Some((2680, 1332))]);
        // Exactly to the bottom of the work area fits; a pixel more fits above no more than below
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[(0, 0, 100, 708)]), [Some((2780, 1116))]);
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[(0, 0, 100, 709)]), [None]);
        // Wider than the work area never fits
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[(0, 0, 3000, 50)]), [None]);
        // A work area that does not start at the corner: a taskbar on top
        let work = (0, 96, 2880, 1728);
        assert_eq!(column((2760, 96, 120, 600), Edge::Right, work, 16, &[(0, 0, 300, 100)]), [Some((2580, 712))]);
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[]), []);
    }

    #[test]
    fn a_mostly_transparent_window_is_placed_by_the_part_it_draws() {
        let pill = (2760, 400, 120, 700);
        // A window of 576 x 1040 that draws only 120 x 300 near its right edge, 400 px down: that
        // part goes flush to the edge under the capsule, and the window's transparent rest runs
        // past the bottom of the work area
        let at = column(pill, Edge::Right, WORK, 16, &[(440, 400, 120, 300), (0, 0, 200, 100)]);
        assert_eq!(at, [Some((2880 - 120 - 440, 1116 - 400)), Some((2680, 1432))]);
        // The part decides what fits, not the window: 708 px of part fit, 709 do not
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[(10, 50, 100, 708)]), [Some((2770, 1066))]);
        assert_eq!(column(pill, Edge::Right, WORK, 16, &[(10, 50, 100, 709)]), [None]);
        // Left edge: the part's left side on the edge, the window may start off the screen
        assert_eq!(column((0, 400, 120, 700), Edge::Left, WORK, 16, &[(30, 40, 100, 100)]), [Some((-30, 1076))]);
        // Top edge: the part's top on the edge, to the right of the capsule
        assert_eq!(column((1000, 0, 700, 120), Edge::Top, WORK, 16, &[(50, 20, 300, 80)]), [Some((1666, -20))]);
    }

    #[test]
    fn the_drawn_part_is_the_union_of_what_shows_inside_the_window() {
        let window = (2304, 818, 576, 1040);
        // Two texts and an image; an empty element and one beside the window count for nothing
        let drawn = [(2700, 900, 100, 40), (2750, 960, 120, 30), (2720, 1000, 60, 60), (2600, 900, 0, 30), (100, 100, 50, 50)];
        assert_eq!(drawn_part(window, &drawn), Some((396, 82, 170, 160)));
        // Half outside: cut to the window
        assert_eq!(drawn_part(window, &[(2850, 800, 100, 50)]), Some((546, 0, 30, 32)));
        // As a web page panel tells it: the page and its document span the window and count for
        // nothing; the pill's group with its padding and a button under it make the part
        let page = [window, window, (2304, 818, 576, 1040), (2768, 1117, 112, 442), (2789, 1147, 71, 72), (2801, 1227, 47, 33), (2772, 1573, 92, 92)];
        assert_eq!(drawn_part(window, &page), Some((464, 299, 112, 548)));
        // A window that is all one element is drawn all over only if it is small enough to be a
        // part; a frame of more than half of it is not
        assert_eq!(drawn_part((0, 0, 100, 100), &[(0, 0, 100, 51)]), None);
        assert_eq!(drawn_part((0, 0, 100, 100), &[(0, 0, 100, 50)]), Some((0, 0, 100, 50)));
        assert_eq!(drawn_part(window, &[]), None);
        assert_eq!(drawn_part(window, &[(0, 0, 10, 10), (2400, 900, -5, 10), (0, 0, 0, 0)]), None);
    }

    #[test]
    fn at_the_top_edge_panels_stand_in_a_row_to_the_right_of_the_capsule_then_to_its_left() {
        let pill = (1000, 0, 700, 120);
        // The third has no room on the right and goes to the left of the pill; the fourth still
        // finds room on the right; the fifth fits on neither side
        let at = column(pill, Edge::Top, WORK, 16, &[(0, 0, 300, 200), (0, 0, 400, 100), (0, 0, 900, 80), (0, 0, 200, 60), (0, 0, 500, 60)]);
        assert_eq!(at, [Some((1716, 0)), Some((2032, 0)), Some((84, 0)), Some((2448, 0)), None]);
        // On the left the part's right side is the gap away from the pill, and the window moves by
        // its offset
        assert_eq!(column(pill, Edge::Top, WORK, 16, &[(0, 0, 1200, 80), (50, 20, 300, 80)]), [None, Some((1666, -20))]);
        assert_eq!(column((2400, 0, 400, 120), Edge::Top, WORK, 16, &[(50, 20, 300, 80)]), [Some((2034, -20))]);
        let work = (0, 48, 2880, 1776);
        assert_eq!(column((1000, 48, 700, 120), Edge::Top, work, 16, &[(0, 0, 300, 200)]), [Some((1716, 48))]);
    }

    #[test]
    fn a_panel_that_does_not_fit_under_the_capsule_stands_above_it_and_none_overlap() {
        let pill = (2760, 900, 120, 500);
        let parts = [(0, 0, 120, 300), (0, 0, 80, 304), (0, 0, 100, 400), (0, 0, 100, 200), (0, 0, 50, 90)];
        let at = column(pill, Edge::Right, WORK, 16, &parts);
        // Under the pill, then above it growing upwards, in the order of the list; one that fits
        // neither is left alone, and a smaller one after it still finds room under the pill
        assert_eq!(at, [Some((2760, 1416)), Some((2800, 580)), Some((2780, 164)), None, Some((2830, 1732))]);
        let placed: Vec<Rect> = at.iter().zip(parts).filter_map(|(at, p)| at.map(|(x, y)| (x + p.0, y + p.1, p.2, p.3))).collect();
        for (i, a) in placed.iter().enumerate() {
            assert!(!overlap(*a, pill), "{a:?} on the pill");
            assert!(placed[i + 1..].iter().all(|b| !overlap(*a, *b)), "{a:?} overlaps");
        }
        // The gap is measured from the pill: the first part above ends 16 px over its top
        assert_eq!(580 + 304 + 16, pill.1);
        // As on the screen measured 20.10.2026: a capsule high up, a tall panel docked under it;
        // the second fits neither under it nor above, and stands where it is, over the first
        let pill = (2760, 92, 120, 874);
        let at = column(pill, Edge::Right, WORK, 16, &[(464, 299, 112, 548), (580, 308, 80, 304)]);
        assert_eq!(at, [Some((2304, 683)), None]);
        assert!(overlap((2800, 1438, 80, 304), (2768, 982, 112, 548)));
        assert!(!overlap((2800, 1438, 80, 304), (2800, 1742, 80, 50)));
        assert!(!overlap((0, 0, 10, 10), (10, 0, 10, 10)));
    }

    #[test]
    fn a_window_stands_at_the_edge_when_its_side_is_near_it() {
        // Flush, a little off, too far, and on another screen's height
        assert!(at_edge((2580, 300, 300, 200), WORK, Edge::Right, 64));
        assert!(at_edge((2540, 300, 300, 200), WORK, Edge::Right, 64));
        assert!(!at_edge((2400, 300, 300, 200), WORK, Edge::Right, 64));
        assert!(!at_edge((2580, 1900, 300, 200), WORK, Edge::Right, 64));
        assert!(!at_edge((2580, 300, 300, 200), WORK, Edge::Left, 64));
        assert!(at_edge((0, 300, 300, 200), WORK, Edge::Left, 64));
        assert!(at_edge((900, 10, 300, 80), WORK, Edge::Top, 64));
        assert!(!at_edge((900, 300, 300, 80), WORK, Edge::Top, 64));
        assert!(!at_edge((3000, 0, 300, 80), WORK, Edge::Top, 64));
    }

    #[test]
    fn a_window_moved_by_something_else_twice_within_ten_seconds_is_let_be() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let (own, place) = ((2580, 300), (2580, 1116));
        let mut m = Moved::default();
        // At its place: nothing to do
        assert_eq!(m.step(ms(0), place, place), Step::Stay);
        // Away from it: moved, seen there, back where it stood: moved again; once more: let be
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.step(ms(500), place, place), Step::Stay);
        assert_eq!(m.step(ms(1000), own, place), Step::Place);
        assert_eq!(m.step(ms(1500), place, place), Step::Stay);
        assert_eq!(m.step(ms(2000), (own.0 + 1, own.1 - 2), place), Step::Release(Why::Returns));
        assert_eq!(m.let_be, Some(Why::Returns));
        // Let be, it is never moved again
        assert_eq!(m.step(ms(3000), own, place), Step::Stay);
        assert_eq!(m.step(ms(60_000), (5, 5), place), Step::Stay);

        // Two moves more than ten seconds apart: put back each time
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.step(ms(500), place, place), Step::Stay);
        assert_eq!(m.step(ms(6000), own, place), Step::Place);
        assert_eq!(m.step(ms(6500), place, place), Step::Stay);
        assert_eq!(m.step(ms(17_000), (100, 100), place), Step::Place);
        assert_eq!(m.let_be, None);

        // As seen on the owner's machine 21.10.2026: the program heads back toward its own saved
        // place every second and never lands exactly where it stood. Moved away from where it was
        // put, wherever to, even before the panel saw it there: the second time it is let be
        let mut m = Moved::default();
        for (t, at, step) in [
            (0, (2304, 851), Step::Place),
            (500, place, Step::Stay),
            (1000, (2304, 796), Step::Place),
            (1500, (2304, 784), Step::Release(Why::Returns)),
            (2000, (2304, 900), Step::Stay),
        ] {
            assert_eq!(m.step(ms(t), at, place), step, "at {t} ms");
        }

        // Left behind by the capsule's move, back and forth: only put at the new place
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.step(ms(500), place, place), Step::Stay);
        let lower = (2580, 1300);
        assert_eq!(m.step(ms(1000), place, lower), Step::Place);
        assert_eq!(m.step(ms(1500), lower, place), Step::Place);
        assert_eq!(m.step(ms(2000), place, lower), Step::Place);
        assert_eq!(m.step(ms(2500), lower, lower), Step::Stay);
        assert_eq!(m.let_be, None);
    }

    #[test]
    fn a_move_on_its_way_is_waited_for_and_one_windows_refuses_is_not_a_return() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let (own, place) = ((2580, 300), (2580, 1116));
        // The move shows only after a while: still where it stood is no return, it is waited for
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.step(ms(500), own, place), Step::Stay);
        assert_eq!(m.step(ms(1500), own, place), Step::Stay);
        assert_eq!(m.step(ms(1800), place, place), Step::Stay);
        // One real return, and the next move again slow: still no second return
        assert_eq!(m.step(ms(2500), own, place), Step::Place);
        assert_eq!(m.step(ms(3000), own, place), Step::Stay);
        assert_eq!(m.step(ms(4000), own, place), Step::Stay);
        assert_eq!(m.let_be, None);

        // A move that never shows is refused: tried again, and after three it is let be, refused
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.step(ms(1000), own, place), Step::Stay);
        assert_eq!(m.step(ms(2000), own, place), Step::Place);
        assert_eq!(m.step(ms(4000), own, place), Step::Place);
        assert_eq!(m.step(ms(6000), own, place), Step::Release(Why::Refused));
        assert_eq!(m.step(ms(9000), own, place), Step::Stay);

        // Windows said no to the move: tried again at once, and after three it is let be, refused
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.failed(), None);
        assert_eq!(m.step(ms(500), own, place), Step::Place);
        assert_eq!(m.failed(), None);
        assert_eq!(m.step(ms(1000), own, place), Step::Place);
        assert_eq!(m.failed(), Some(Why::Refused));
        assert_eq!(m.let_be, Some(Why::Refused));
        assert_eq!(m.step(ms(1500), own, place), Step::Stay);
        // A move that worked in between starts the count again
        let mut m = Moved::default();
        assert_eq!(m.step(ms(0), own, place), Step::Place);
        assert_eq!(m.failed(), None);
        assert_eq!(m.step(ms(500), own, place), Step::Place);
        assert_eq!(m.failed(), None);
        assert_eq!(m.step(ms(1000), own, place), Step::Place);
        assert_eq!(m.step(ms(1500), place, place), Step::Stay);
        assert_eq!(m.step(ms(9000), own, place), Step::Place);
        assert_eq!(m.failed(), None);
        assert_eq!(m.let_be, None);
    }

    fn window(hwnd: isize, path: &str, class: &str, rect: Rect) -> TopWindow {
        let exe = path.rsplit('\\').next().unwrap().to_lowercase();
        TopWindow { id: WindowId { hwnd, pid: 7, born: 1 }, exe, family: None, path: path.into(), class: class.into(), rect, border: (0, 0), see_through: true }
    }

    #[test]
    fn a_new_window_of_the_program_is_followed_afresh_even_after_its_old_one_was_let_be() {
        let mut watches = Vec::new();
        let first = WindowId { hwnd: 10, pid: 7, born: 1 };
        watch_for(&mut watches, "panel.exe", first).moved.let_be = Some(Why::Returns);
        assert_eq!(watch_for(&mut watches, "panel.exe", first).moved.let_be, Some(Why::Returns));
        // The program restarted: another window, or the same handle in another process
        assert_eq!(watch_for(&mut watches, "panel.exe", WindowId { hwnd: 11, pid: 7, born: 1 }).moved.let_be, None);
        watch_for(&mut watches, "panel.exe", first).moved.let_be = Some(Why::Refused);
        assert_eq!(watch_for(&mut watches, "panel.exe", WindowId { hwnd: 10, pid: 8, born: 2 }).moved.let_be, None);
        // Each entry has its own
        watch_for(&mut watches, "other.exe", first);
        assert_eq!(watches.len(), 2);
        // The list changes: a window let be is tried again and keeps being its entry's window; the
        // entry taken off the list is forgotten
        let reborn = WindowId { hwnd: 10, pid: 8, born: 2 };
        watch_for(&mut watches, "panel.exe", reborn).moved.let_be = Some(Why::Returns);
        start_again(&mut watches, &[Docked { exe: "panel.exe".into(), path: None, class: None }]);
        assert_eq!(watches.len(), 1);
        assert_eq!((watches[0].id, watches[0].moved.let_be), (reborn, None));
    }

    #[test]
    fn the_ticked_window_is_found_by_path_and_class_and_an_opaque_one_is_never_shrunk() {
        let ticked = Docked { exe: "panel.exe".into(), path: Some("C:\\Tools\\panel.exe".into()), class: Some("Panel".into()) };
        let windows = [
            window(1, "C:\\Tools\\panel.exe", "Popup", (0, 0, 2000, 1500)),
            window(2, "C:\\Tools\\panel.exe", "Panel", (100, 100, 900, 900)),
            window(3, "C:\\Tools\\panel.exe", "Panel", (2580, 300, 300, 200)),
            window(4, "D:\\Copy\\panel.exe", "Panel", (2480, 300, 400, 900)),
        ];
        let edge = |t: &TopWindow| at_edge(t.rect, WORK, Edge::Right, 64);
        let found = |f: Found| match f {
            Found::One(t) => Some(t.id.hwnd),
            Found::None => None,
            Found::Several => Some(-1),
        };
        // Of its own path and class, the only one at the capsule's edge, though smaller; another
        // program of the same file name and the program's popup are never it
        assert_eq!(found(pick(&ticked, None, &windows, edge)), Some(3));
        // The only one of its path and class, but away from the edge: never pulled in
        assert_eq!(found(pick(&ticked, None, &windows[..2], edge)), None);
        assert_eq!(found(pick(&ticked, None, &windows[..1], edge)), None);
        // An entry of an older version goes by the file name: one at the edge of the four
        let old = Docked { exe: "panel.exe".into(), path: None, class: None };
        assert_eq!(found(pick(&old, None, &windows[..3], edge)), Some(3));
        assert_eq!(found(pick(&old, None, &windows, edge)), Some(-1));
        assert_eq!(found(pick(&old, None, &windows, |_| false)), None);
        // As seen 21.10.2026: the panel's bubble is hidden, and the program's only window above the
        // others is an overlay at the bottom of the screen; it is not the panel
        let overlay = window(7, "C:\\Tools\\panel.exe", "Panel", (1184, 1740, 512, 100));
        assert_eq!(pick(&ticked, None, [&overlay], edge), Found::None);
        assert_eq!(pick(&old, None, [&overlay], edge), Found::None);

        // The part a person sees: shrunk to what it draws only when the window can be seen through
        let rect = (2304, 818, 576, 1040);
        let drawn = [(2768, 1117, 112, 442)];
        assert_eq!(part_of(true, rect, &drawn), (464, 299, 112, 442));
        assert_eq!(part_of(false, rect, &drawn), (0, 0, 576, 1040));
        assert_eq!(part_of(true, rect, &[]), (0, 0, 576, 1040));
    }

    #[test]
    fn the_window_a_panel_went_by_is_kept_and_of_several_equal_ones_none_is_guessed() {
        let ticked = Docked { exe: "panel.exe".into(), path: Some("C:\\Tools\\panel.exe".into()), class: Some("Panel".into()) };
        let edge = |t: &TopWindow| at_edge(t.rect, WORK, Edge::Right, 64);
        let small = window(3, "C:\\Tools\\panel.exe", "Panel", (2580, 300, 300, 200));
        let large = window(5, "C:\\Tools\\panel.exe", "Panel", (1880, 0, 1000, 1500));
        // The program opens a larger window of the same class at the edge: the panel keeps the
        // one it went by
        assert_eq!(pick(&ticked, Some(small.id), [&small, &large], edge), Found::One(&small));
        // Kept but out of the list for now (minimized, or no longer above the others): no other
        // window is taken in its place
        assert_eq!(pick(&ticked, Some(small.id), [&large], edge), Found::None);
        // The same handle in another run of the program is not the kept window
        let reborn = TopWindow { id: WindowId { born: 2, ..small.id }, ..window(3, "C:\\Tools\\panel.exe", "Panel", small.rect) };
        assert_eq!(pick(&ticked, Some(small.id), [&reborn], edge), Found::None);
        // Gone: looked for again, and two at the edge of the same path and class are not guessed
        assert_eq!(pick(&ticked, None, [&small, &large], edge), Found::Several);
        assert_eq!(pick(&ticked, None, [&large], edge), Found::One(&large));
        let away = window(6, "C:\\Tools\\panel.exe", "Panel", (100, 100, 300, 200));
        assert_eq!(pick(&ticked, None, [&away, &large], edge), Found::One(&large));
        assert_eq!(pick(&ticked, None, [&away], edge), Found::None);
    }

    #[test]
    fn a_window_that_can_be_seen_through_stands_at_the_edge_by_the_part_it_draws() {
        // A transparent window whose pill is flush with the right edge, its frame far from it
        let panel = window(1, "C:\\Tools\\panel.exe", "Panel", (2304, 818, 576, 1040));
        let shape = Shape { id: panel.id, size: (576, 1040), part: (464, 299, 112, 548), when: Instant::now() };
        assert_eq!(shown_rect(&panel, std::slice::from_ref(&shape)), (2768, 1117, 112, 548));
        assert!(at_edge(shown_rect(&panel, std::slice::from_ref(&shape)), WORK, Edge::Right, 64));
        // Its part drawn in the middle of the frame, away from the edge: not at it, though the
        // frame is
        let inner = Shape { part: (0, 299, 112, 548), ..shape.clone() };
        assert!(at_edge(panel.rect, WORK, Edge::Right, 64));
        assert!(!at_edge(shown_rect(&panel, &[inner]), WORK, Edge::Right, 64));
        // Not known yet, or known for another size or another window: the whole of what shows
        let resized = Shape { size: (500, 1040), ..shape.clone() };
        assert_eq!(shown_rect(&panel, &[resized]), panel.rect);
        let other = Shape { id: WindowId { hwnd: 9, ..panel.id }, ..shape };
        assert_eq!(shown_rect(&panel, &[other]), panel.rect);
        assert_eq!(shown_rect(&panel, &[]), panel.rect);
    }
}
