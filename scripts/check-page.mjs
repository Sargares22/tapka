// Page check: renders ui/index.html and ui/settings.html in a headless Chromium browser with no Tauri and
// no Windows calls, then inspects the resulting DOM. Run with: node --test scripts/check-page.mjs
import { execFileSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
import { test } from 'node:test';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
// Chrome, not Edge: on this machine headless Edge exits with an empty --dump-dom (checked
// 2026-10-04). Both are Chromium, the engine WebView2 uses.
const browser = ['C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe'].find(existsSync);

// Returns a page's DOM after its scripts have run for `ms` of virtual time. The page is given a
// stand-in for Tauri whose `get_view` answers with window.__view, `get_settings` with
// window.__settings, `running_programs` with window.__programs, `pinned_apps` with window.__pinned
// and `bundle_items` with what window.__bundle makes of its arguments; every call is kept in
// window.__calls and the page's event listeners in window.__on, by event name.
// `script` runs after the page's own script and may leave its findings in the DOM.
function renderedDom(items, script = '', ms = 4000, page = 'index.html', settings = null) {
  assert.ok(browser, 'Google Chrome not found');
  const work = mkdtempSync(join(tmpdir(), 'tapka-check-'));
  try {
    const stub = `<script>window.__calls=[];window.__on={};window.__view={items:${JSON.stringify(items)},cell:54,tablet:false,edge:'right',theme:'dark',accent:'#8ab4ff',add:['Добавить','Открыть редактор пунктов']};window.__settings=${JSON.stringify(settings)};window.__programs=[{name:'Paint Studio',exe:'paint.exe'},{name:'Sketch',exe:'sketch.exe'}];window.__TAURI__={core:{invoke:async(c,a)=>{window.__calls.push([c,a]);return c==='get_view'?window.__view:c==='get_settings'?JSON.parse(JSON.stringify(window.__settings)):c==='running_programs'?window.__programs:c==='pinned_apps'?window.__pinned:c==='bundle_items'?window.__bundle?.(a):null}},event:{listen(n,cb){window.__on[n]=cb}}}</script>`;
    const html = readFileSync(join(root, 'ui', page), 'utf8')
      .replace('<head>', '<head>' + stub)
      .replace('</body>', `<script>${script}</script></body>`);
    const file = join(work, 'page.html');
    writeFileSync(file, html);
    return execFileSync(browser, [
      '--headless=new', '--disable-gpu', '--no-first-run', `--user-data-dir=${join(work, 'profile')}`,
      `--virtual-time-budget=${ms}`, '--dump-dom', pathToFileURL(file).href,
    ], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 90000 });
  } finally {
    rmSync(work, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }
}

const PNG = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==';
// `at` is an item's place in the settings file, which Rust sends with each item it shows
const items = [
  { at: 0, name: 'Снимок', hint: 'Выделить область экрана', icon: null },
  { at: 1, name: 'диктовка', icon: null },
  { at: 2, name: 'Сайт', icon: PNG },
  { at: 3, name: 'Calc', icon: null },
  { at: 4, name: 'Вставить', icon: null, glyph: '<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 4h16"/></svg>' },
];

// Presses, releases and hovers on the items; after each step notes what the name card shows
// (null when hidden) and how many taps the page has reported.
const gestures = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const card = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent; };
  const sub = () => document.querySelector('#card .c-sub').textContent;
  const taps = () => window.__calls.filter(c => c[0] === 'tap').length;
  const fire = (el, type, pointerType) => el.dispatchEvent(
    new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType, clientX: 10, clientY: 10 }));
  // What Rust would report with the cursor over the middle of an item, and after it has left
  const over = el => { const r = el.getBoundingClientRect(); window.__on.hover({ payload: [r.left + r.width / 2, r.top + r.height / 2] }); };
  const away = () => window.__on.hover({ payload: null });
  await sleep(50);
  const b = document.querySelectorAll('.item');
  const out = {};
  fire(b[0], 'pointerdown', 'touch');
  await sleep(350); out.held350 = card();
  await sleep(100); out.held450 = card(); out.hint450 = sub();
  // Where the card and the tail sit against the pressed item and the window
  const mid = el => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; };
  const box = document.getElementById('card').getBoundingClientRect();
  out.place = { item: mid(b[0]), card: mid(document.getElementById('card')), top: box.top, bottom: box.bottom, height: innerHeight };
  out.dipped = b[0].classList.contains('down');
  fire(b[0], 'pointerup', 'touch'); out.released = card(); out.dippedAfter = b[0].classList.contains('down'); out.tapsAfterLongPress = taps();
  fire(b[1], 'pointerdown', 'touch'); await sleep(100); fire(b[1], 'pointerup', 'touch');
  out.tapsAfterShortPress = taps();
  await sleep(500); out.afterShortPress = card();
  away(); over(b[2]); out.lit = b[2].classList.contains('hot');
  await sleep(250); out.hover250 = card();
  await sleep(100); out.hover350 = card(); out.hintHover = sub();
  away(); out.hoverLeft = card(); out.litAfter = b[2].classList.contains('hot');
  // A finger tap leaves the system cursor on the item: no hover card until the cursor goes away
  fire(b[3], 'pointerdown', 'touch'); fire(b[3], 'pointerup', 'touch'); over(b[3]); await sleep(400); out.touchOver = card();
  // A finger that moves on after the label came up scrolls: the label goes and there is no tap
  away();
  fire(b[0], 'pointerdown', 'touch'); await sleep(450); out.longAgain = card();
  window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 40 }));
  out.movedAfterLong = card();
  const before = taps(); fire(b[0], 'pointerup', 'touch'); out.tapsAfterMove = taps() - before;
  // The press is taken away without a release: the label goes too
  fire(b[0], 'pointerdown', 'touch'); await sleep(450);
  document.getElementById('keys').dispatchEvent(new PointerEvent('lostpointercapture', { pointerId: 1 }));
  out.lostCapture = card();
  away();
  window.__view = { ...window.__view, cell: 70.2, tablet: true };
  window.__on.reload(); await sleep(50);
  out.tabletCell = document.getElementById('list').style.getPropertyValue('--cell');
  over(document.querySelectorAll('.item')[2]); await sleep(400); out.tabletHover = card();
  // Rust says a carry has started; the capsule looks carried until the reload that ends it
  window.__on.carry({}); out.lifted = document.body.classList.contains('dragging');
  window.__view = { ...window.__view, edge: 'left' }; window.__on.reload(); await sleep(50);
  out.edge = document.body.dataset.edge; out.landed = !document.body.classList.contains('dragging');
  // Docked to the top the capsule lies flat: keys side by side, the label under its key
  window.__view = { ...window.__view, edge: 'top', tablet: false }; window.__on.reload(); await sleep(50);
  const k = document.querySelectorAll('.item');
  const r0 = k[0].getBoundingClientRect(), r1 = k[1].getBoundingClientRect();
  out.row = Math.abs(r0.top - r1.top) < 0.5 && r1.left - r0.left > 40;
  away(); over(k[1]); await sleep(350);
  const lab = document.getElementById('card').getBoundingClientRect();
  out.under = card() === 'диктовка' && lab.top >= r1.bottom && Math.abs((lab.left + lab.right) / 2 - (r1.left + r1.right) / 2) < 1;
  // Rust says which items' programs are running; the page marks them and unmarks them
  const marks = () => [...document.querySelectorAll('.item')].map(el => el.classList.contains('on'));
  const lit = () => [...document.querySelectorAll('.item')].map(el => el.classList.contains('live'));
  window.__on.marks({ payload: [[false, false], [true, true], [false, false], [true, false], [false, false]] }); out.marked = marks(); out.live = lit();
  window.__on.marks({ payload: [[false, false], [false, false], [false, false], [true, false], [false, false]] }); out.unmarked = marks(); out.unlive = lit();
  // The last key is not an item: a tap on it asks for the editor, with no index
  const all = document.querySelectorAll('.item'), plus = all[all.length - 1];
  out.plusLast = plus.classList.contains('add') && all.length === 6;
  fire(plus, 'pointerdown', 'touch'); fire(plus, 'pointerup', 'touch');
  out.plusTap = window.__calls.filter(c => c[0] === 'tap').pop()[1].index;
  away(); over(plus); await sleep(350); out.plusLabel = card();
  // The settings change the theme and the accent; the page follows at the next reload
  out.themeBefore = document.documentElement.dataset.theme;
  window.__view = { ...window.__view, theme: 'light', accent: '#ff8800' }; window.__on.reload(); await sleep(50);
  out.themeAfter = document.documentElement.dataset.theme;
  out.accentAfter = document.documentElement.style.getPropertyValue('--accent');
  out.pillLight = getComputedStyle(document.getElementById('pill')).backgroundColor;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;

// Two browser starts serve every check below: a start takes about ten seconds
const dom = renderedDom(items);
const buttons = [...dom.matchAll(/<button class="item"[^>]*><span class="key">(.*?)<\/span><\/button>/gs)].map(m => m[1]);
const found = html => JSON.parse(html.match(/data-check="([^"]*)"/)[1].replaceAll('&quot;', '"').replaceAll('&amp;', '&').replaceAll('&lt;', '<').replaceAll('&gt;', '>'));
const seen = found(renderedDom(items, gestures, 6000));

test('the page renders one capsule with one circle per item, in order', () => {
  assert.equal((dom.match(/id="pill"/g) || []).length, 1);
  assert.equal(buttons.length, items.length);
});

test('story 13: item with an icon file shows the icon', () => {
  assert.match(buttons[2], /^<img src="data:image\/png;base64,[^"]+" alt="Сайт">$/);
});

test('story 14: item without an icon shows the first letter of its name', () => {
  assert.deepEqual([buttons[0], buttons[1], buttons[3]], ['С', 'Д', 'C']);
});

test('story 18: name card appears after a 400 ms press or a 300 ms hover', () => {
  assert.equal(seen.held350, null);
  assert.equal(seen.held450, 'Снимок');
  // The hint from the settings file sits under the name
  assert.equal(seen.hint450, 'Выделить область экрана');
  // The label is level with its key unless the window edge is in the way, and never leaves the window
  const p = seen.place;
  assert.ok(p.top >= 8 && p.bottom <= p.height - 8, JSON.stringify(p));
  assert.ok(Math.abs(p.card - Math.max(p.item, (p.bottom - p.top) / 2 + 8)) < 0.5, JSON.stringify(p));
  assert.equal(seen.hintHover, '');
  assert.equal(seen.hover250, null);
  assert.equal(seen.hover350, 'Сайт');
  // The item under the cursor lights up at once and goes dark when the cursor leaves
  assert.equal(seen.lit, true);
  assert.equal(seen.litAfter, false);
  // A finger has no hover
  assert.equal(seen.touchOver, null);
});

test('story 21: tablet mode spreads the items and turns the hover card off', () => {
  assert.equal(seen.tabletCell, '70.2px');
  assert.equal(seen.tabletHover, null);
});

test('story 19: name card hides on release or leave, and a long press is not a tap', () => {
  assert.equal(seen.released, null);
  assert.equal(seen.longAgain, 'Снимок');
  assert.equal(seen.movedAfterLong, null);
  assert.equal(seen.tapsAfterMove, 0);
  assert.equal(seen.lostCapture, null);
  // The key dips while it is held and comes back on release
  assert.equal(seen.dipped, true);
  assert.equal(seen.dippedAfter, false);
  assert.equal(seen.tapsAfterLongPress, 0);
  assert.equal(seen.hoverLeft, null);
  // A short press is a tap and shows no card
  assert.equal(seen.tapsAfterShortPress, 1);
  assert.equal(seen.afterShortPress, null);
});

test('a carry shows as carried until it lands, and the page changes sides at the left edge', () => {
  assert.equal(seen.lifted, true);
  assert.equal(seen.edge, 'left');
  assert.equal(seen.landed, true);
});

test('docked to the top the capsule lies flat and the label opens under its key', () => {
  assert.equal(seen.row, true);
  assert.equal(seen.under, true);
});

test('items whose program is running are marked', () => {
  assert.deepEqual(seen.marked, [false, true, false, true, false, false]);
  assert.deepEqual(seen.unmarked, [false, false, false, true, false, false]);
  // An item whose watched window is showing (dictation recording) is lit, and goes dark after
  assert.deepEqual(seen.live, [false, true, false, false, false, false]);
  assert.deepEqual(seen.unlive, [false, false, false, false, false, false]);
});

test('a built-in icon is drawn inline, in the ink of the theme', () => {
  assert.match(buttons[4], /^<svg viewBox="0 0 24 24" stroke="currentColor">/);
});

test('the last key is the plus: it opens the editor and is not one of the items', () => {
  assert.equal((dom.match(/<button class="item add"/g) || []).length, 1);
  assert.equal(seen.plusLast, true);
  assert.equal(seen.plusTap, null);
  assert.equal(seen.plusLabel, 'Добавить');
});

test('the capsule takes its theme and accent from the settings', () => {
  assert.equal(seen.themeBefore, 'dark');
  assert.equal(seen.themeAfter, 'light');
  assert.equal(seen.accentAfter, '#ff8800');
  assert.equal(seen.pillLight, 'rgb(238, 240, 244)');
});

// ---------- keys for one program
// Which items show for the active program is decided in Rust (`actions::shown_items`, tested
// there); the page is handed the items to show, in Rust's order: the first two for all programs,
// the program's, the rest. Here: the file has four items for all programs and two for paint.exe,
// and paint.exe comes to the front, then goes.
const everywhere = [0, 1, 3, 4].map(at => ({ at, name: 'Всем ' + at, icon: null }));
const paint = [2, 5].map(at => ({ at, name: 'Paint ' + at, icon: null }));
const switching = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const list = document.getElementById('list');
  const names = () => [...document.querySelectorAll('#list .item')].map(b => b.textContent);
  const card = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent; };
  const fire = (el, type) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 10 }));
  // As short as the real window makes a capsule of four keys and more: the list scrolls
  document.getElementById('pill').style.height = '260px';
  await sleep(50);
  out.before = names();
  list.scrollTop = 40; out.scrolledTo = list.scrollTop;
  const firstKeys = [...list.children].slice(0, 2).map(b => b.getBoundingClientRect().top - list.getBoundingClientRect().top + list.scrollTop);
  // The program comes to the front for the first time: its keys after the others, and a note
  window.__view = { ...window.__view, items: ${JSON.stringify([...everywhere.slice(0, 2), ...paint, ...everywhere.slice(2)])} };
  window.__on.set({ payload: 'Клавиши для Paint Studio' }); await sleep(30);
  out.during = names();
  out.appearing = [...list.children].map(b => b.getAnimations({ subtree: true }).length > 0);
  out.longest = Math.max(...[...list.children].flatMap(b => b.getAnimations({ subtree: true })).map(a => a.effect.getTiming().duration));
  out.note = card();
  const box = document.getElementById('card').getBoundingClientRect();
  out.noteInside = box.top >= 0 && box.bottom <= innerHeight && box.right <= list.getBoundingClientRect().left;
  out.noteRegion = window.__calls.filter(c => c[0] === 'card').pop()[1].open;
  out.sameFirstKeys = [...list.children].slice(0, 2).every((b, i) => Math.abs(b.getBoundingClientRect().top - list.getBoundingClientRect().top + list.scrollTop - firstKeys[i]) < 0.5);
  await sleep(1000); out.scrollAfter = list.scrollTop; out.noteAt1s = card();
  await sleep(1200); out.noteAt2s = card();
  // A tap on a key of the program reports its place in the file
  const third = list.children[2];
  fire(third, 'pointerdown'); fire(third, 'pointerup');
  out.tapAt = window.__calls.filter(c => c[0] === 'tap').pop()[1].index;
  // The program goes: its keys go, no note; the program comes back: no note a second time
  window.__view = { ...window.__view, items: ${JSON.stringify(everywhere)} };
  window.__on.set({ payload: null }); await sleep(30);
  out.after = names(); out.noteAfter = card();
  // The program comes to the front while a finger holds the plus: nothing is redrawn under it, the
  // held key keeps its label, the release still opens the editor, and then the keys change
  const plus = () => document.querySelector('.item.add');
  plus().dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 2, pointerType: 'touch', clientX: 10, clientY: 10 }));
  await sleep(450); out.heldLabel = card();
  window.__view = { ...window.__view, items: ${JSON.stringify([...everywhere.slice(0, 2), ...paint, ...everywhere.slice(2)])} };
  window.__on.set({ payload: 'Клавиши для Paint Studio' }); await sleep(30);
  out.whileHeld = names(); out.heldLabelAfter = card();
  window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 2, pointerType: 'touch', clientX: 10, clientY: 10 }));
  await sleep(30);
  out.releasedOn = window.__calls.filter(c => c[0] === 'tap').length;
  const short = document.querySelector('.item.add');
  short.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 3, pointerType: 'touch', clientX: 10, clientY: 10 }));
  window.__view = { ...window.__view, items: ${JSON.stringify(everywhere)} };
  window.__on.set({ payload: null }); await sleep(30);
  out.shortHeld = names();
  window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 3, pointerType: 'touch', clientX: 10, clientY: 10 }));
  await sleep(30);
  out.shortTap = window.__calls.filter(c => c[0] === 'tap').pop()[1].index;
  out.afterRelease = names();
  // The program comes to the front and a finger lands on a key while Rust is still answering:
  // the answer waits for the release, and the release taps the key under the finger, not the
  // program's key drawn in its place since
  window.__view = { ...window.__view, items: ${JSON.stringify([...everywhere.slice(0, 2), ...paint, ...everywhere.slice(2)])} };
  window.__on.set({ payload: null });
  const third2 = list.children[2];
  third2.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 4, pointerType: 'pen', clientX: 10, clientY: 10 }));
  await sleep(30);
  out.racedHeld = names();
  window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 4, pointerType: 'pen', clientX: 10, clientY: 10 }));
  await sleep(30);
  out.racedTap = window.__calls.filter(c => c[0] === 'tap').pop()[1].index;
  out.racedAfter = names();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const sw = found(renderedDom(everywhere, switching, 4000));

test('story 26: keys for the active program come after the first two, softly, with a note once', () => {
  assert.deepEqual(sw.before, ['В', 'В', 'В', 'В']);
  assert.ok(sw.scrolledTo > 0, 'the list could not be scrolled');
  // Two keys for all programs, the two for the program, the other two for all programs
  assert.equal(sw.during.length, 6);
  assert.deepEqual(sw.during, ['В', 'В', 'P', 'P', 'В', 'В']);
  assert.equal(sw.sameFirstKeys, true);
  // Only the new keys come in with an animation, and it is over in under 200 ms
  assert.deepEqual(sw.appearing, [false, false, true, true, false, false]);
  assert.ok(sw.longest > 0 && sw.longest <= 200, String(sw.longest));
  // The list goes back to its start
  assert.equal(sw.scrollAfter, 0);
  // The note shows beside the capsule, through the window's region, for two seconds
  assert.equal(sw.note, 'Клавиши для Paint Studio');
  assert.equal(sw.noteInside, true);
  assert.equal(sw.noteRegion, true);
  assert.equal(sw.noteAt1s, 'Клавиши для Paint Studio');
  assert.equal(sw.noteAt2s, null);
  assert.equal(sw.tapAt, 2);
  assert.equal(sw.after.length, 4);
  assert.equal(sw.noteAfter, null);
});

test('story 26: the keys do not change under a held finger; the release taps the key it was on', () => {
  // A long press on the plus: its label stays, the keys wait for the release
  assert.equal(sw.heldLabel, 'Добавить');
  assert.equal(sw.whileHeld.length, 4);
  assert.equal(sw.heldLabelAfter, 'Добавить');
  // A short press on the plus while the program leaves: the plus is tapped, then the keys change
  assert.deepEqual(sw.shortHeld, ['В', 'В', 'P', 'P', 'В', 'В']);
  assert.equal(sw.shortTap, null);
  assert.equal(sw.afterRelease.length, 4);
  // A press that lands while the new set is on its way: nothing changes under the pen, the
  // release taps the key it was on (the third key for all programs, at 3 in the file), then the
  // new set is drawn
  assert.deepEqual(sw.racedHeld, ['В', 'В', 'В', 'В']);
  assert.equal(sw.racedTap, 3);
  assert.deepEqual(sw.racedAfter, ['В', 'В', 'P', 'P', 'В', 'В']);
});

// ---------- keys held for the pen
// Rust keeps the key down and says which items hold theirs (`held`); here the stand-in for Rust
// answers a tap on a `hold` item the way Rust does. Item 1 holds Shift, item 3 Ctrl.
const holding = [
  { at: 0, name: 'Снимок', icon: null },
  { at: 1, name: 'Держать Shift', icon: null },
  { at: 2, name: 'Вставить', icon: null },
  { at: 3, name: 'Держать Ctrl', icon: null, held: true },
];
const holdScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const down = new Set([3]);
  const real = window.__TAURI__.core.invoke;
  window.__TAURI__.core.invoke = async (c, a) => {
    if (c === 'tap' && (a.index === 1 || a.index === 3)) {
      down.has(a.index) ? down.delete(a.index) : down.add(a.index);
      window.__on.held({ payload: [...down] });
    }
    return real(c, a);
  };
  const lit = () => [...document.querySelectorAll('#list .item')].map(b => b.classList.contains('held'));
  const tap = b => { for (const type of ['pointerdown', 'pointerup']) b.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'pen', clientX: 10, clientY: 10 })); };
  await sleep(50);
  const keys = () => document.querySelectorAll('#list .item');
  out.start = lit();
  tap(keys()[1]); await sleep(30); out.afterTap = lit();
  // Lit exactly as an item whose watched window is showing, once the tap's flash is over
  const look = b => { b.querySelector('.key').getAnimations().forEach(a => a.finish()); const k = getComputedStyle(b.querySelector('.key')); return [k.backgroundColor, k.boxShadow]; };
  keys()[0].classList.add('live'); await sleep(300);
  out.looks = [look(keys()[1]), look(keys()[0])];
  out.sameAsLive = JSON.stringify(look(keys()[1])) === JSON.stringify(look(keys()[0]));
  keys()[0].classList.remove('live');
  out.unlitLook = JSON.stringify(look(keys()[2])) !== JSON.stringify(look(keys()[1]));
  // Other keys work as usual meanwhile
  tap(keys()[2]); await sleep(30);
  out.pasteTap = window.__calls.filter(c => c[0] === 'tap').pop()[1].index; out.afterPaste = lit();
  tap(keys()[1]); await sleep(30); out.afterSecondTap = lit();
  // Rust let go of everything by itself: nothing is lit
  window.__on.held({ payload: [] }); out.afterLetGo = lit();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const held = found(renderedDom(holding, holdScript, 3000));

test('story 27: a hold item is lit after a tap and goes out after the second', () => {
  // A key already held when the capsule is drawn shows lit
  assert.deepEqual(held.start, [false, false, false, true]);
  assert.deepEqual(held.afterTap, [false, true, false, true]);
  assert.equal(held.sameAsLive, true, JSON.stringify(held.looks));
  assert.equal(held.unlitLook, true);
  assert.equal(held.pasteTap, 2);
  assert.deepEqual(held.afterPaste, [false, true, false, true]);
  assert.deepEqual(held.afterSecondTap, [false, false, false, true]);
  assert.deepEqual(held.afterLetGo, [false, false, false, false]);
});

// ---------- feedback on a tap
// A stand-in for WebAudio notes each tone the page plays: when it starts and stops, in seconds.
const feedback = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const tones = [];
  const param = () => ({ setValueAtTime() {}, exponentialRampToValueAtTime() {} });
  window.AudioContext = class {
    constructor() { this.currentTime = 0; this.state = 'running'; this.destination = {}; }
    createOscillator() { const o = { frequency: param(), connect: n => n, start: t => o.at = t, stop: t => tones.push(t - o.at) }; return o; }
    createGain() { return { gain: param(), connect: n => n }; }
  };
  const fire = (el, type, y = 10) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: y }));
  const keys = () => document.querySelectorAll('#list .item');
  const flashing = () => [...keys()].map(b => b.classList.contains('flash'));
  const look = () => [...keys()].map(b => { const k = getComputedStyle(b.querySelector('.key')); return [k.backgroundColor, k.boxShadow, k.transform].join(); }).join('|');
  await sleep(50);
  out.rest = look();
  // A tap: the key flashes at once and is back at rest soon after; the sound is off
  fire(keys()[1], 'pointerdown'); fire(keys()[1], 'pointerup');
  out.during = flashing();
  out.duringLook = getComputedStyle(keys()[1].querySelector('.key')).backgroundColor !== getComputedStyle(keys()[0].querySelector('.key')).backgroundColor;
  await sleep(60); out.at60 = flashing()[1];
  await sleep(240); out.at300 = flashing(); out.restAfter = look() === out.rest; out.tonesOff = tones.length;
  // A finger that moves scrolls, and a long press shows the label: no flash either way
  fire(keys()[2], 'pointerdown');
  window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 40 }));
  fire(keys()[2], 'pointerup', 40); out.afterScroll = flashing();
  fire(keys()[2], 'pointerdown'); await sleep(450); fire(keys()[2], 'pointerup'); out.afterLong = flashing();
  // The sound is switched on: one short tone per tap
  window.__view = { ...window.__view, click_sound: true }; window.__on.reload(); await sleep(50);
  fire(keys()[0], 'pointerdown'); fire(keys()[0], 'pointerup');
  out.tonesOn = tones.slice();
  // Without WebAudio the tap still works and still flashes
  delete window.AudioContext;
  const taps = window.__calls.filter(c => c[0] === 'tap').length;
  await sleep(300);
  fire(keys()[3], 'pointerdown'); fire(keys()[3], 'pointerup');
  out.noAudioTap = window.__calls.filter(c => c[0] === 'tap').length - taps; out.noAudioFlash = flashing()[3];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const fb = found(renderedDom(items, feedback, 3000));

test('story 28: a tap flashes its key for a moment; a scroll or a long press does not', () => {
  assert.deepEqual(fb.during, [false, true, false, false, false]);
  assert.equal(fb.duringLook, true);
  assert.equal(fb.at60, true);
  assert.deepEqual(fb.at300, [false, false, false, false, false]);
  // At rest the capsule looks exactly as before the tap
  assert.equal(fb.restAfter, true);
  assert.deepEqual(fb.afterScroll, [false, false, false, false, false]);
  assert.deepEqual(fb.afterLong, [false, false, false, false, false]);
});

// A key held and then moved is carried to another place among the keys
const carrying = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const keys = () => [...document.querySelectorAll('#list .item')];
  const at = (type, y, pointerType = 'touch') => new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType, clientX: 10, clientY: y });
  const moves = () => window.__calls.filter(c => c[0] === 'move_item').map(c => [c[1].from, c[1].to]);
  const taps = () => window.__calls.filter(c => c[0] === 'tap').length;
  // Where each key is being sent, whatever part of the way its transition has gone
  const shifts = () => keys().map(b => Math.round(parseFloat((b.style.transform.match(/-?[0-9.]+/) || [0])[0])));
  await sleep(50);
  const y = i => { const r = keys()[i].getBoundingClientRect(); return r.top + r.height / 2; };
  const [y0, y1, y2] = [y(0), y(1), y(2)];
  // Held past the long press, then moved two keys down: the key follows, the two it passed step up
  keys()[0].dispatchEvent(at('pointerdown', y0)); await sleep(450);
  window.dispatchEvent(at('pointermove', y0 + 30)); out.lifted = keys()[0].classList.contains('carried'); out.label = !document.getElementById('card').hidden;
  window.dispatchEvent(at('pointermove', y2)); await sleep(200);
  out.during = shifts();
  window.dispatchEvent(at('pointerup', y2)); await sleep(200);
  out.moved = moves(); out.tapsAfterCarry = taps(); out.kept = shifts();
  // Rust saved the order and says so: the keys are drawn anew, nothing is left shifted
  window.__view = { ...window.__view, items: [1, 2, 0, 3, 4].map(i => window.__view.items[i]) };
  window.__on.reload(); await sleep(50);
  out.after = shifts(); out.carriedAfter = keys().some(b => b.classList.contains('carried'));
  // Put back where it was taken from: nothing is saved and nothing stays shifted
  keys()[1].dispatchEvent(at('pointerdown', y1)); await sleep(450);
  window.dispatchEvent(at('pointermove', y1 + 30)); window.dispatchEvent(at('pointermove', y1 + 4)); window.dispatchEvent(at('pointerup', y1 + 4)); await sleep(200);
  out.movedBack = moves().length; out.backShifts = shifts();
  // The plus is not carried, and a move before the long press still scrolls and carries nothing
  const plus = document.querySelector('.item.add'), py = plus.getBoundingClientRect().top + 5;
  plus.dispatchEvent(at('pointerdown', py)); await sleep(450); window.dispatchEvent(at('pointermove', py - 60)); out.plusCarried = !!document.querySelector('.carried'); window.dispatchEvent(at('pointerup', py - 60));
  keys()[0].dispatchEvent(at('pointerdown', y0)); window.dispatchEvent(at('pointermove', y0 + 60)); out.earlyCarried = !!document.querySelector('.carried'); window.dispatchEvent(at('pointerup', y0 + 60));
  out.movedInAll = moves().length;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const cr = found(renderedDom(items, carrying, 5000));

test('a key held and then moved is carried to another place, and the order goes to Rust', () => {
  assert.equal(cr.lifted, true);
  assert.equal(cr.label, false);
  // The carried key is two cells down, the two it passed are one cell up, the rest stand still
  assert.deepEqual(cr.during, [108, -54, -54, 0, 0]);
  assert.deepEqual(cr.moved, [[0, 2]]);
  assert.equal(cr.tapsAfterCarry, 0);
  assert.deepEqual(cr.kept, [108, -54, -54, 0, 0]);
  assert.deepEqual(cr.after, [0, 0, 0, 0, 0]);
  assert.equal(cr.carriedAfter, false);
  assert.equal(cr.movedBack, 1);
  assert.deepEqual(cr.backShifts, [0, 0, 0, 0, 0]);
  assert.equal(cr.plusCarried, false);
  assert.equal(cr.earlyCarried, false);
  assert.equal(cr.movedInAll, 1);
});

// Pictures that would be lost on their keys are made to read in either theme
const reading = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  // A sign of one colour on a clear ground, and a picture of two colours
  const sign = colour => { const c = document.createElement('canvas'); c.width = c.height = 24; const p = c.getContext('2d'); p.fillStyle = colour; p.fillRect(6, 6, 12, 12); return c.toDataURL(); };
  const flag = (a, b) => { const c = document.createElement('canvas'); c.width = c.height = 24; const p = c.getContext('2d'); p.fillStyle = a; p.fillRect(0, 0, 24, 12); p.fillStyle = b; p.fillRect(0, 12, 24, 12); return c.toDataURL(); };
  const pictures = [sign('#ffffff'), sign('#101010'), flag('#d97757', '#ffffff'), flag('#ffffff', '#fff2a8')];
  const look = async theme => {
    window.__view = { ...window.__view, theme, items: pictures.map((icon, at) => ({ at, name: 'n' + at, icon })) };
    window.__on.reload(); await sleep(300);
    return [...document.querySelectorAll('#list .key')].map(k => k.querySelector('.sign') ? 'ink' : k.classList.contains('plate') ? 'plate' : 'as is');
  };
  out.light = await look('light');
  out.dark = await look('dark');
  const s = document.querySelector('#list .sign');
  out.inkOfTheme = s ? getComputedStyle(s).backgroundColor : null;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const rd = found(renderedDom(items, reading, 3000));

test('a picture that would be lost on its key is drawn in the ink of the theme or gets a plate', () => {
  // On light keys the white sign turns to ink and the pale picture of two colours gets a plate
  assert.deepEqual(rd.light, ['ink', 'as is', 'as is', 'plate']);
  // On dark keys it is the black sign that turns; the rest read as they are
  assert.deepEqual(rd.dark, ['as is', 'ink', 'as is', 'as is']);
  assert.equal(rd.inkOfTheme, 'rgb(230, 232, 238)');
});

test('story 28: the click sounds only when switched on, and the page works without WebAudio', () => {
  assert.equal(fb.tonesOff, 0);
  assert.equal(fb.tonesOn.length, 1);
  assert.ok(fb.tonesOn[0] > 0 && fb.tonesOn[0] <= 0.03, String(fb.tonesOn[0]));
  assert.equal(fb.noAudioTap, 1);
  assert.equal(fb.noAudioFlash, true);
});

// ---------- the settings window
const glyph = '<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 4h16"/></svg>';
const settings = {
  items: [
    { raw: { name: 'Снимок', icon: 'snip', hint: 'Выделить область', action: 'hotkey', keys: 'win+shift+s' }, icon: null, glyph, problem: null },
    { raw: { name: 'Диктовка', icon: 'mic', action: 'hotkey', keys: 'ctrl+space', lit: { program: 'recorder.exe', window: 'Recording' } }, icon: null, glyph, problem: null },
    { raw: { name: 'Заметки', action: 'open', target: 'C:\\Tools\\notes.exe' }, icon: PNG, glyph: null, problem: null },
    { raw: { name: 'Потом', action: 'script' }, icon: null, glyph: null, problem: 'unknown action "script"' },
  ],
  // Every kind of item without a hint or a picture, as the list must word it
  kinds: [
    { raw: { name: 'Программа', action: 'open', target: 'C:\\Program Files\\Tool\\tool.exe' } },
    { raw: { name: 'Магазин', action: 'open', target: 'shell:AppsFolder\\Vendor.App_abc!App' } },
    { raw: { name: 'Сайт', action: 'open', target: 'https://www.example.com/feed' } },
    { raw: { name: 'Папка', action: 'open', target: 'C:\\Users\\me\\Pictures' } },
    { raw: { name: 'Файл', action: 'open', target: 'C:\\Users\\me\\plan.txt' } },
    { raw: { name: 'Bluetooth', action: 'open', target: 'ms-settings:bluetooth' } },
    { raw: { name: 'Параметры', action: 'open', target: 'ms-settings:' } },
    { raw: { name: 'Чат', action: 'open', target: 'claude://new' } },
    { raw: { name: 'Вставить', action: 'hotkey', keys: 'ctrl+v' } },
    { raw: { name: 'Голос', action: 'hotkey', keys: 'win+h' } },
    { raw: { name: 'Shift', action: 'hold', keys: 'shift' } },
    { raw: { name: 'Кисть', action: 'hotkey', keys: 'b', only_in: 'paint.exe' }, only: 'Paint Studio' },
  ].map(i => ({ icon: null, glyph: null, problem: null, only: null, ...i })),
  // What a hand may leave in the file: not an object, a name that is not text
  odd: [
    { raw: null, icon: null, glyph: null, problem: 'no name' },
    { raw: 'text', icon: null, glyph: null, problem: 'no name' },
    { raw: { name: 5, action: 'hotkey', keys: 7, hint: { toString: 0 } }, icon: null, glyph: null, problem: 'no name' },
    { raw: { name: 'Крив', action: 'open', target: 'x', hint: { toString: 0 } }, icon: null, glyph: null, problem: null },
    { raw: { name: 'Цел', action: 'open', target: 'x' }, icon: null, glyph: null, problem: null },
  ],
  scale: 1, edge: 'right', theme: 'dark', accent: '#8ab4ff', lang: 'system', russian: true, autostart: false, updates: true, tablet_only: false, click_sound: false,
  version: '1.0.0', error: null, intro: true,
  presets: [{ id: 'snip', item: { name: 'Снимок', icon: 'snip', hint: 'Выделить область экрана', action: 'hotkey', keys: 'win+shift+s' } },
    { id: 'settings', item: { name: 'Параметры', icon: 'gear', hint: 'Открыть параметры Windows', action: 'open', target: 'ms-settings:' } }],
  pages: ['bluetooth', 'network-wifi', 'sound', 'display', 'nightlight', 'batterysaver', 'notifications', 'printers', 'windowsupdate', 'pen']
    .map(page => ({ item: { name: page === 'bluetooth' ? 'Bluetooth' : page, icon: 'gear', action: 'open', target: 'ms-settings:' + page } })),
  bundles: [{ id: 'tablet', name: 'Планшет', items: [{ name: 'Снимок', icon: 'snip', action: 'hotkey', keys: 'win+shift+s' }] },
    { id: 'text', name: 'Текст', items: [{ name: 'Вставить', icon: 'paste', action: 'hotkey', keys: 'ctrl+v' }, { name: 'Голосовой ввод', icon: 'mic', action: 'hotkey', keys: 'win+h' }] }],
  glyphs: { snip: glyph, mic: glyph, gear: glyph, keyboard: glyph, browser: glyph, folder: glyph },
};
const editing = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const last = name => calls(name).pop();
  // The stand-in for Rust keeps the file as it was: each step starts from that list again
  const again = async () => { window.__on.reload(); await sleep(20); };
  const texts = sel => [...document.querySelectorAll(sel)].map(e => e.textContent);
  const kind = n => { $('add').click(); document.querySelectorAll('#body .row')[n].click(); };
  const seg = n => document.querySelectorAll('#body .seg button')[n].click();
  const chip = name => [...document.querySelectorAll('#body .prog')].find(b => b.textContent === name).click();
  const save = () => document.querySelector('#foot .acc').click();
  // Contrast of two colours as CSS computes them, by the WCAG formula
  const lum = c => { const v = c.match(/\\d+/g).slice(0, 3).map(n => n / 255).map(x => x <= .03928 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4); return .2126 * v[0] + .7152 * v[1] + .0722 * v[2]; };
  const contrast = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + .05) / (Math.min(x, y) + .05); };
  const style = (e, p) => getComputedStyle(e)[p];
  await sleep(50);
  const out = {};
  const rows = () => [...document.querySelectorAll('#items .row')];
  out.rows = rows().map(r => r.querySelector('.lbl b').textContent);
  out.lines = rows().map(r => r.querySelector('.lbl small').textContent);
  // No item shows a letter for a picture: each key holds a drawing or an icon
  out.keys = rows().map(r => { const k = r.querySelector('.key'); return [!!k.querySelector('svg, img'), k.textContent.trim()]; });
  out.intro = !$('intro').hidden;
  $('intro-close').click(); out.introClosed = $('intro').hidden;
  out.navRu = texts('.nav span');
  out.theme = document.documentElement.dataset.theme;
  out.accent = document.documentElement.style.getPropertyValue('--accent');
  out.onAccent = document.documentElement.style.getPropertyValue('--on-accent');
  // Cards stand out from the page, and text reads on them, in both themes
  const card = $('items'), page = document.body;
  const measure = () => ({ card: contrast(style(card, 'backgroundColor'), style(page, 'backgroundColor')),
    dim: contrast(style(card.querySelector('.lbl small'), 'color'), style(card, 'backgroundColor')),
    fill: contrast(style($('add'), 'backgroundColor'), style($('add'), 'color')),
    fillOnCard: contrast(style($('add'), 'backgroundColor'), style(card, 'backgroundColor')) });
  out.darkContrast = measure();
  // Look: a change is shown at once and goes to Rust as one setting
  document.querySelector('#theme .th[data-v=light]').click();
  out.themeAfter = document.documentElement.dataset.theme; out.themeCall = last('set_pref');
  out.lightContrast = measure();
  document.querySelectorAll('.swatch')[5].click();
  out.accentCall = last('set_pref'); out.accentLight = measure().fill;
  out.swatches = document.querySelectorAll('.swatch').length; out.firstSwatch = document.querySelector('.swatch').dataset.v;
  $('scale').children[2].click(); out.scaleCall = last('set_pref'); out.capsule = $('capsule').style.transform;
  $('edge').children[0].click(); out.edgeCall = last('set_pref');
  $('autostart').click(); out.autostartCall = last('set_pref');
  $('tablet-only').click(); out.tabletOnlyCall = last('set_pref'); out.tabletOnlyOn = $('tablet-only').classList.contains('on');
  out.soundOff = !$('click-sound').classList.contains('on');
  $('click-sound').click(); out.soundCall = last('set_pref'); out.soundOn = $('click-sound').classList.contains('on');
  out.quit = !!$('quit');
  window.__settings = { ...window.__settings, theme: 'dark' }; await again();
  // The add dialog offers five kinds; a Windows action is added by one tap
  $('add').click();
  out.kinds = texts('#body .row b');
  document.querySelectorAll('#body .row')[1].click();
  out.groups = texts('#body .glabel');
  out.tiles = texts('#body .tile > span:not(.key)');
  document.querySelectorAll('#body .tile')[1].click();
  out.added = last('save_items').items.map(i => i.name); out.addedItem = last('save_items').items[4];
  out.closedAfterAdd = !$('shade').classList.contains('on');
  await again();
  // A page of Windows Settings, from the same tiles
  kind(1); document.querySelectorAll('#body .tile')[4].click();
  out.pageItem = last('save_items').items[4];
  await again();
  // A program pinned to the taskbar: on top of the program list, read anew each time
  window.__pinned = [{ name: 'Paint Studio', target: 'C:\\\\Tools\\\\paint.exe' }, { name: 'Notes', target: 'shell:AppsFolder\\\\Vendor.Notes_abc!App' }];
  kind(0); await sleep(20);
  out.appTitle = $('title').textContent;
  out.pinned = texts('#pinned-list .row b');
  out.appKeys = [...document.querySelectorAll('#body .key')].every(k => k.querySelector('svg, img') && !k.textContent.trim());
  document.querySelectorAll('#pinned-list .row')[1].click();
  out.pinnedItem = last('save_items').items[4];
  await again();
  window.__pinned = [];
  kind(0); await sleep(20);
  out.pinnedNone = document.querySelector('#pinned-list .say').textContent;
  closeDialog();
  // A ready set for one program: one choice for every set; Rust says what it adds, the page puts it at the end
  window.__bundle = a => a.items.some(i => i.keys === 'ctrl+v' && (i.only_in || null) === a.onlyIn) ? []
    : [{ name: 'Вставить', icon: 'paste', action: 'hotkey', keys: 'ctrl+v', ...(a.onlyIn ? { only_in: a.onlyIn } : {}) }];
  kind(4); await sleep(20);
  out.bundleTitle = $('title').textContent;
  out.bundles = texts('#body .bundle b');
  out.bundleKeys = document.querySelectorAll('#body .bundle')[1].querySelectorAll('.key').length;
  out.bundleWhere = document.querySelectorAll('#body .seg').length;
  out.bundleAccent = document.querySelectorAll('#body .bundle .acc').length;
  seg(1); out.bundlePrograms = texts('#body .prog'); chip('Paint Studio');
  // Two quick taps, the second before Rust has answered the first: one set
  const asked = calls('bundle_items').length, saved = calls('save_items').length;
  const go = n => document.querySelectorAll('#body .bundle .btn')[n].click();
  go(1); go(1); go(0); await sleep(20);
  out.bundleTwice = [calls('bundle_items').length - asked, calls('save_items').length - saved];
  out.bundleCall = last('bundle_items');
  out.bundleSaved = last('save_items').items;
  out.bundleToast = $('toast-text').textContent;
  out.bundleClosed = !$('shade').classList.contains('on');
  // Everything of the set already there: nothing is saved and the dialog stays
  window.__settings = { ...window.__settings, items: [...window.__settings.items, { raw: out.bundleSaved[4], icon: null, glyph: null, problem: null, only: 'Paint Studio' }] };
  await again();
  const saves = calls('save_items').length;
  kind(4); await sleep(20); seg(1); chip('Paint Studio');
  go(1); await sleep(20);
  out.bundleAgain = calls('save_items').length - saves;
  out.bundleAgainToast = $('toast-text').textContent;
  closeDialog();
  window.__settings = { ...window.__settings, items: window.__settings.items.slice(0, 4) };
  await again();
  // A site, file or folder: a bare address is a site
  kind(2);
  const inputs = () => [...document.querySelectorAll('#body input[type=text]')];
  inputs()[0].value = 'example.com'; inputs()[1].value = 'Сайт'; save();
  out.site = last('save_items').items[4];
  await again();
  // A shortcut: put together with the buttons, or pressed on a keyboard
  kind(3);
  const mods = () => [...document.querySelectorAll('#body .field .btn')];
  mods()[0].click(); mods()[3].click();
  const key = document.querySelector('#body select'); key.value = 'k'; key.onchange();
  out.keysShown = texts('#body .does .kc');
  save();
  out.keysByButtons = last('save_items').items[4];
  await again();
  kind(3);
  window.dispatchEvent(new KeyboardEvent('keydown', { code: 'F5', ctrlKey: true, shiftKey: true, bubbles: true }));
  save();
  out.keysPressed = last('save_items').items[4].keys;
  await again();
  // A tap on a row opens its card; the card fits the window without scrolling
  rows()[2].click();
  out.cardTitle = $('title').textContent;
  // Its whole height, scrolled part included, within the 640 of the window less the margins of the shade
  out.cardHeight = Math.round($('dialog').getBoundingClientRect().height - $('body').clientHeight + $('body').scrollHeight);
  out.pathHidden = !document.querySelector('#body details').open && inputs()[2].value === 'C:\\\\Tools\\\\notes.exe';
  // The icon buttons are buttons, folded until asked for
  out.glyphsFolded = document.querySelectorAll('#body .gl').length === 0;
  [...document.querySelectorAll('#body .btn')].find(b => b.textContent === 'Сменить…').click();
  const own = [...document.querySelectorAll('#body .btn')].find(b => b.textContent === 'Свой файл…');
  out.ownButton = [style(own, 'height'), style(own, 'paddingLeft')];
  out.glyphCount = document.querySelectorAll('#body .gl > button').length;
  closeDialog();
  // Editing keeps what the window does not show: the lit rule stays in the item
  rows()[1].click();
  inputs()[0].value = 'Запись'; inputs()[1].value = 'Подсказка';
  save();
  out.edited = last('save_items').items[1];
  out.editedOthers = last('save_items').items.map(i => i.name);
  await again();
  // A broken item says so in words; the parser's words are under Details
  rows()[3].click();
  out.brokenCard = [document.querySelector('#body .does .problem').textContent, document.querySelector('#body .why').textContent];
  closeDialog();
  // Removing is in the card, takes the item out and offers to put it back
  rows()[0].click(); document.querySelector('#foot .danger').click();
  out.removed = last('save_items').items.map(i => i.name);
  out.toast = $('toast').classList.contains('on');
  $('toast-undo').click(); out.restored = last('save_items').items.map(i => i.name);
  // Order: the first item carried below the second; the dots do not open the card
  const grip = rows()[0].querySelector('.grip'), h = rows()[0].offsetHeight;
  grip.setPointerCapture = () => {};
  const at = (type, y) => grip.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, pointerType: 'touch', clientX: 20, clientY: y }));
  at('pointerdown', 100); at('pointermove', 100 + h); at('pointerup', 100 + h);
  out.moved = last('save_items').items.map(i => i.name);
  rows()[0].querySelector('.grip').click();
  out.gripOpens = $('shade').classList.contains('on');
  // An item for one program: its line says which, and the card offers the programs with windows
  window.__settings = { ...window.__settings, items: [...window.__settings.items,
    { raw: { name: 'Кисть', action: 'hotkey', keys: 'b', only_in: 'Paint.exe', own: 1 }, icon: null, glyph: null, problem: null, only: 'Paint Studio' },
    { raw: { name: 'Ластик', action: 'hotkey', keys: 'e', only_in: 'old.exe' }, icon: null, glyph: null, problem: null, only: 'old' }] };
  await again();
  out.tag = rows()[4].querySelector('.lbl small .tag').textContent;
  out.tagLine = rows()[4].querySelector('.lbl small').textContent;
  out.noTag = rows()[0].querySelector('.tag') === null;
  rows()[4].click(); await sleep(20);
  out.whereLabel = [...document.querySelectorAll('#body .field > label')].pop().textContent;
  out.whereSeg = texts('#body .seg button.on');
  out.wherePrograms = texts('#body .prog'); out.whereOn = texts('#body .prog.on');
  // Saved untouched, the field stays as the file has it
  save();
  out.untouched = last('save_items').items[4];
  await again();
  // A program no longer running is still offered, so it is not lost
  rows()[5].click(); await sleep(20);
  out.keptPrograms = texts('#body .prog'); out.keptOn = texts('#body .prog.on');
  closeDialog();
  // Everywhere: the field goes; one program: the field comes
  rows()[4].click(); await sleep(20);
  seg(0); save();
  out.everywhere = last('save_items').items[4];
  await again();
  rows()[2].click(); await sleep(20);
  seg(1); const before = calls('save_items').length; save(); out.needProgram = [calls('save_items').length - before, $('toast-text').textContent];
  chip('Sketch'); save();
  out.chosen = last('save_items').items[2];
  await again();
  // Every kind of item in words, with no path, address or file name
  window.__settings = { ...window.__settings, items: window.__settings.kinds }; await again();
  out.kindLines = rows().map(r => r.querySelector('.lbl small').textContent);
  out.kindKeys = rows().map(r => { const k = r.querySelector('.key'); return !!k.querySelector('svg') && !k.textContent.trim(); });
  // English: Rust says the language, the page changes every text
  window.__settings = { ...window.__settings, items: window.__settings.kinds.slice(0, 0).concat(window.__first), russian: false, intro: false, theme: 'light' }; window.__on.reload(); await sleep(50);
  out.navEn = texts('.nav span');
  out.addEn = $('add').textContent; out.linesEn = rows().map(r => r.querySelector('.lbl small').textContent);
  // A file with odd items: the list is still drawn, and an odd item can be opened and removed
  window.__settings = { ...window.__settings, items: window.__settings.odd }; window.__on.reload(); await sleep(50);
  out.oddRows = rows().map(r => r.querySelector('.lbl b').textContent);
  rows()[0].click(); out.oddOpens = $('shade').classList.contains('on');
  document.querySelector('#foot .danger').click(); out.oddRemoved = last('save_items').items.length;
  // Two removals in a hurry, before Rust has answered the first: the second starts from the first
  rows()[0].click(); document.querySelector('#foot .danger').click(); out.oddRemovedTwice = last('save_items').items.length;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const set = found(renderedDom([], `window.__first = ${JSON.stringify(settings.items)};` + editing, 5000, 'settings.html', settings));

test('the settings window lists the items of the file in words, broken ones included', () => {
  assert.deepEqual(set.rows, ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.lines, ['Выделить область', 'Сочетание Ctrl+Space', 'Программа', 'Пункт не работает: коснитесь, чтобы исправить']);
  assert.deepEqual(set.keys, [[true, ''], [true, ''], [true, ''], [true, '']]);
  assert.equal(set.intro, true);
  assert.equal(set.introClosed, true);
});

test('an item without a hint is told by its kind, never by its path, address or file', () => {
  assert.deepEqual(set.kindLines, ['Программа', 'Программа', 'Сайт', 'Папка', 'Файл', 'Параметры: Bluetooth', 'Параметры Windows', 'Ссылка',
    'Сочетание Ctrl+V', 'Действие Windows', 'Держит Shift', 'только в Paint StudioСочетание B']);
  for (const line of set.kindLines) assert.doesNotMatch(line, /\\|:\/\/|\.exe|ms-settings:/i);
  // Without a picture of its own an item shows a drawing of its kind, not a letter
  assert.ok(set.kindKeys.every(Boolean));
  assert.deepEqual(set.linesEn, ['Выделить область', 'Shortcut Ctrl+Space', 'Program', 'This item does not work: tap to fix it']);
});

test('theme, accent, size, edge and autostart apply at once and go to Rust one by one', () => {
  assert.deepEqual([set.theme, set.accent, set.onAccent], ['dark', '#8ab4ff', '#fff']);
  assert.equal(set.themeAfter, 'light');
  assert.deepEqual(set.themeCall, { key: 'theme', value: 'light' });
  assert.deepEqual(set.accentCall, { key: 'accent', value: '#ff9a3c' });
  assert.equal(set.swatches, 8);
  assert.equal(set.firstSwatch, '#8ab4ff');
  assert.deepEqual(set.scaleCall, { key: 'scale', value: 1.25 });
  assert.equal(set.capsule, 'scale(1.25)');
  assert.deepEqual(set.edgeCall, { key: 'edge', value: 'left' });
  assert.deepEqual(set.autostartCall, { key: 'autostart', value: true });
  assert.deepEqual(set.tabletOnlyCall, { key: 'tablet_only', value: true });
  assert.equal(set.tabletOnlyOn, true);
  // Story 28: the tap sound is off until switched on
  assert.equal(set.soundOff, true);
  assert.deepEqual(set.soundCall, { key: 'click_sound', value: true });
  assert.equal(set.soundOn, true);
  // Quit lives in the tray
  assert.equal(set.quit, false);
});

test('cards stand out from the page and every text and button reads, in both themes', () => {
  for (const c of [set.darkContrast, set.lightContrast]) {
    assert.ok(c.card >= 1.3, `card ${c.card}`);
    assert.ok(c.dim >= 4.5, `dim text ${c.dim}`);
    assert.ok(c.fill >= 4.5, `button text ${c.fill}`);
    assert.ok(c.fillOnCard >= 3, `button on card ${c.fillOnCard}`);
  }
  // A light orange accent darkens by itself on a button until its white letters read
  assert.ok(set.accentLight >= 4.5, `orange button ${set.accentLight}`);
});

test('an item is added in five ways', () => {
  assert.deepEqual(set.kinds, ['Программа', 'Действие Windows', 'Сайт или файл', 'Сочетание клавиш', 'Готовый набор']);
  assert.deepEqual(set.groups, ['Частые', 'Окна', 'Страницы Параметров']);
  assert.deepEqual(set.tiles.slice(0, 3), ['Снимок', 'Параметры', 'Bluetooth']);
  assert.equal(set.tiles.length, 12);
  assert.deepEqual(set.added, ['Снимок', 'Диктовка', 'Заметки', 'Потом', 'Параметры']);
  assert.deepEqual(set.addedItem, settings.presets[1].item);
  assert.equal(set.closedAfterAdd, true);
  assert.deepEqual(set.pageItem, { name: 'sound', icon: 'gear', action: 'open', target: 'ms-settings:sound' });
  assert.deepEqual(set.site, { name: 'Сайт', action: 'open', target: 'https://example.com' });
  assert.deepEqual(set.keysShown, ['Ctrl', 'Win', 'K']);
  assert.deepEqual(set.keysByButtons, { name: 'Ctrl + Win + K', icon: 'keyboard', action: 'hotkey', keys: 'ctrl+win+k' });
  assert.equal(set.keysPressed, 'ctrl+shift+f5');
});

test('a pinned program is added by one tap from the top of the program list', () => {
  assert.equal(set.appTitle, 'Программа');
  assert.deepEqual(set.pinned, ['Paint Studio', 'Notes']);
  assert.equal(set.appKeys, true);
  assert.deepEqual(set.pinnedItem, { name: 'Notes', action: 'open', target: 'shell:AppsFolder\\Vendor.Notes_abc!App' });
  assert.equal(set.pinnedNone, 'Закреплённых не найдено');
});

test('a ready set goes to the end of the list, for the program chosen, and not twice', () => {
  assert.equal(set.bundleTitle, 'Готовый набор');
  assert.deepEqual(set.bundles, ['Планшет', 'Текст']);
  assert.equal(set.bundleKeys, 2);
  // One "where" for every set, programs by name, and no set button in the accent
  assert.equal(set.bundleWhere, 1);
  assert.equal(set.bundleAccent, 0);
  assert.deepEqual(set.bundlePrograms, ['Paint Studio', 'Sketch']);
  assert.deepEqual(set.bundleTwice, [1, 1]);
  assert.equal(set.bundleCall.id, 'text');
  assert.equal(set.bundleCall.onlyIn, 'paint.exe');
  assert.deepEqual(set.bundleCall.items.map(i => i.name), ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.bundleSaved.map(i => i.name), ['Снимок', 'Диктовка', 'Заметки', 'Потом', 'Вставить']);
  assert.deepEqual(set.bundleSaved[4], { name: 'Вставить', icon: 'paste', action: 'hotkey', keys: 'ctrl+v', only_in: 'paint.exe' });
  assert.equal(set.bundleToast, 'Добавлено клавиш: 1');
  assert.equal(set.bundleClosed, true);
  assert.equal(set.bundleAgain, 0);
  assert.equal(set.bundleAgainToast, 'Все клавиши набора уже есть в списке');
});

test('a tap on an item opens its card, which fits the window with its path folded away', () => {
  assert.equal(set.cardTitle, 'Изменить');
  assert.ok(set.cardHeight <= 640 - 36, `card ${set.cardHeight} px`);
  assert.equal(set.pathHidden, true);
  assert.equal(set.glyphsFolded, true);
  assert.deepEqual(set.ownButton, ['34px', '16px']);
  assert.equal(set.glyphCount, 6);
  assert.deepEqual(set.brokenCard, ['Пункт не работает', 'unknown action "script"']);
});

test('an item is edited, removed, restored and moved; fields the window does not show are kept', () => {
  assert.deepEqual(set.edited, { name: 'Запись', icon: 'mic', action: 'hotkey', keys: 'ctrl+space', lit: { program: 'recorder.exe', window: 'Recording' }, hint: 'Подсказка' });
  assert.deepEqual(set.editedOthers, ['Снимок', 'Запись', 'Заметки', 'Потом']);
  assert.deepEqual(set.removed, ['Диктовка', 'Заметки', 'Потом']);
  assert.equal(set.toast, true);
  assert.deepEqual(set.restored, ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.moved, ['Диктовка', 'Снимок', 'Заметки', 'Потом']);
  assert.equal(set.gripOpens, false);
});

test('odd items in the file do not break the list', () => {
  assert.deepEqual(set.oddRows, ['—', '—', '—', 'Крив', 'Цел']);
  assert.equal(set.oddOpens, true);
  assert.equal(set.oddRemoved, 4);
  assert.equal(set.oddRemovedTwice, 3);
});

test('story 26: the card shows and sets the program an item is for, keeping the rest', () => {
  assert.equal(set.tag, 'только в Paint Studio');
  assert.equal(set.tagLine, 'только в Paint StudioСочетание B');
  assert.equal(set.noTag, true);
  assert.equal(set.whereLabel, 'Где показывать');
  assert.deepEqual(set.whereSeg, ['В одной программе']);
  assert.deepEqual(set.wherePrograms, ['Paint Studio', 'Sketch']);
  assert.deepEqual(set.whereOn, ['Paint Studio']);
  assert.deepEqual(set.untouched, { name: 'Кисть', action: 'hotkey', keys: 'b', only_in: 'Paint.exe', own: 1 });
  assert.deepEqual(set.keptPrograms, ['Paint Studio', 'Sketch', 'old']);
  assert.deepEqual(set.keptOn, ['old']);
  assert.deepEqual(set.everywhere, { name: 'Кисть', action: 'hotkey', keys: 'b', own: 1 });
  assert.deepEqual(set.needProgram, [0, 'Выберите программу']);
  assert.deepEqual(set.chosen, { name: 'Заметки', action: 'open', target: 'C:\\Tools\\notes.exe', only_in: 'sketch.exe' });
});

test('the settings window speaks Russian and English', () => {
  assert.deepEqual(set.navRu, ['Пункты', 'Вид', 'Общие', 'О программе']);
  assert.deepEqual(set.navEn, ['Items', 'Look', 'General', 'About']);
  assert.equal(set.addEn, 'Add');
});
