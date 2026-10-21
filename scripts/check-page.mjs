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
// and `bundle_items` with what window.__bundle makes of its arguments, `open_key_set` and
// `key_set_items` with what window.__keySet and window.__keySetItems make, `dock_candidates` with
// window.__candidates, `first_programs` with window.__found, and `save_docked` and `save_meters` fail while window.__saveFails is set, `save_items` while window.__itemsFail is, and waits for window.__itemsHold while it is set; every call is kept in
// window.__calls and the page's event listeners in window.__on, by event name.
// `script` runs after the page's own script and may leave its findings in the DOM; `before` runs
// before it, and `hash` is put after the page's address, as `#welcome` for the welcome window.
function renderedDom(items, script = '', ms = 4000, page = 'index.html', settings = null, { hash = '', before = '' } = {}) {
  assert.ok(browser, 'Google Chrome not found');
  const work = mkdtempSync(join(tmpdir(), 'tapka-check-'));
  try {
    const stub = `<script>window.__calls=[];window.__on={};window.__view={items:${JSON.stringify(items)},cell:54,tablet:false,edge:'right',theme:'dark',accent:'#8ab4ff',add:['Добавить','Настройки и сворачивание капсулы'],gear:{name:'Настройки',hint:'Открыть окно настроек',glyph:'<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 4h16"/></svg>'},fold:{name:'Свернуть',hint:'Свернуть капсулу в одну клавишу',glyph:'<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 12h16"/></svg>'},unfold:'<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M20 12H4"/></svg>'};window.__settings=${JSON.stringify(settings)};window.__programs=[{name:'Paint Studio',exe:'paint.exe'},{name:'Sketch',exe:'sketch.exe'}];window.__TAURI__={core:{invoke:async(c,a)=>{window.__calls.push([c,a]);if((c==='save_docked'||c==='save_meters')&&window.__saveFails)throw 'the file is locked';if(c==='save_items'&&window.__itemsFail)throw 'the file is locked';if(c==='save_items'&&window.__itemsHold)await window.__itemsHold;return c==='get_view'?window.__view:c==='get_settings'?JSON.parse(JSON.stringify(window.__settings)):c==='running_programs'?window.__programs:c==='pinned_apps'?window.__pinned:c==='bundle_items'?window.__bundle?.(a):c==='open_key_set'?window.__keySet():c==='key_set_items'?window.__keySetItems(a):c==='save_key_set'?3:c==='dock_candidates'?window.__candidates:c==='first_programs'?window.__found:null}},event:{listen(n,cb){window.__on[n]=cb}}}</script>`;
    const html = readFileSync(join(root, 'ui', page), 'utf8')
      .replace('<head>', '<head>' + stub + `<script>${before}</script>`)
      .replace('</body>', `<script>${script}</script></body>`);
    const file = join(work, 'page.html');
    writeFileSync(file, html);
    return execFileSync(browser, [
      '--headless=new', '--disable-gpu', '--no-first-run', `--user-data-dir=${join(work, 'profile')}`,
      `--virtual-time-budget=${ms}`, '--dump-dom', pathToFileURL(file).href + hash,
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
  // The last key is not an item: it has its label, and a tap on it runs nothing and opens the
  // two signs of the foot
  const all = document.querySelectorAll('.item'), plus = all[all.length - 1];
  out.plusLast = plus.classList.contains('add') && all.length === 6;
  away(); over(plus); await sleep(350); out.plusLabel = card(); away();
  const tapsBefore = taps();
  fire(plus, 'pointerdown', 'touch'); fire(plus, 'pointerup', 'touch');
  out.plusTap = [taps() - tapsBefore, window.__calls.filter(c => c[0] === 'foot').pop()[1].open];
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

test('the last key is the plus: it opens the signs and is not one of the items', () => {
  assert.equal((dom.match(/<button class="item add"/g) || []).length, 1);
  assert.equal(seen.plusLast, true);
  assert.deepEqual(seen.plusTap, [0, true]);
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
  // Short enough for the list of four keys to scroll, and long enough for the third key to show
  // whole while the two signs and the end grip are up, which make the pill longer by 82 px
  document.getElementById('pill').style.height = '296px';
  window.__view = { ...window.__view, pill: [296, 378] };
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
  // held key keeps its label, and after the release the keys change
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
  out.shortTap = window.__calls.filter(c => c[0] === 'foot').length;
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
  assert.equal(sw.shortTap, 1);
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

// ---------- a key that did not work
// Rust says which item's action failed and gives the two lines to show (`failed`); here the
// stand-in for Rust answers a tap with them, in Russian and in English, as Rust would.
const failingScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const answers = [];
  const real = window.__TAURI__.core.invoke;
  window.__TAURI__.core.invoke = async (c, a) => {
    if (c === 'tap' && answers.length) window.__on.failed({ payload: { at: a.index, ...answers.shift() } });
    return real(c, a);
  };
  const label = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent + '|' + c.querySelector('.c-sub').textContent; };
  const mid = el => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; };
  const beside = b => Math.abs(mid(document.getElementById('card')) - mid(b)) < 0.5;
  const fire = (el, type) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 10 }));
  await sleep(50);
  const keys = document.querySelectorAll('#list .item');
  // A tap whose keys Windows refused: the words come up beside the tapped key
  answers.push({ title: 'Не сработало', why: 'Windows не приняла нажатие' });
  fire(keys[1], 'pointerdown'); fire(keys[1], 'pointerup'); await sleep(30);
  out.ru = label(); out.ruBeside = beside(keys[1]);
  out.region = window.__calls.filter(c => c[0] === 'card').pop()[1].open;
  // Two seconds on, another key fails (a program that took its time not to open): its words
  // take the place of the first, and stay their own two and a half seconds
  await sleep(2000); out.at2s = label();
  window.__on.failed({ payload: { at: 3, title: "Didn't work", why: 'This window runs as administrator' } });
  out.en = label(); out.enBeside = beside(keys[3]);
  await sleep(600); out.pastFirst = label();
  await sleep(1800); out.at2400 = label();
  await sleep(200); out.at2600 = label();
  out.regionClosed = window.__calls.filter(c => c[0] === 'card').pop()[1].open;
  // A failure that arrives while a key is held waits for the release
  fire(keys[2], 'pointerdown');
  window.__on.failed({ payload: { at: 2, title: 'Не сработало', why: 'Не открылось' } });
  out.whileHeld = label();
  fire(keys[2], 'pointerup'); out.afterRelease = label(); out.afterReleaseBeside = beside(keys[2]);
  // The next press closes the words, and a long press shows the key's own label as before
  fire(keys[0], 'pointerdown'); out.nextPress = label();
  await sleep(450); out.name = label();
  fire(keys[0], 'pointerup'); out.nameGone = label();
  // An item that is not on the capsule now has no key to speak beside
  window.__on.failed({ payload: { at: 9, title: 'Не сработало', why: 'Не открылось' } }); out.noKey = label();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const fl = found(renderedDom(items, failingScript, 8000));

test('story 34: a key that did not work says so beside itself for two and a half seconds', () => {
  assert.equal(fl.ru, 'Не сработало|Windows не приняла нажатие');
  assert.equal(fl.ruBeside, true);
  // Through the window's region, like any label
  assert.equal(fl.region, true);
  assert.equal(fl.at2s, fl.ru);
  assert.equal(fl.en, "Didn't work|This window runs as administrator");
  assert.equal(fl.enBeside, true);
  // The first failure's time ran out meanwhile: it does not close the second's words
  assert.equal(fl.pastFirst, fl.en);
  assert.equal(fl.at2400, fl.en);
  assert.equal(fl.at2600, null);
  assert.equal(fl.regionClosed, false);
  assert.equal(fl.whileHeld, null);
  assert.equal(fl.afterRelease, 'Не сработало|Не открылось');
  assert.equal(fl.afterReleaseBeside, true);
  assert.equal(fl.nextPress, null);
  assert.equal(fl.name, 'Снимок|Выделить область экрана');
  assert.equal(fl.nameGone, null);
  assert.equal(fl.noKey, null);
});

// The words wait for the release and for the drawing put off by the press, and go with their key
// when the list scrolls
const lateScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const list = document.getElementById('list');
  const label = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent + '|' + c.querySelector('.c-sub').textContent; };
  const mid = el => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; };
  const key = at => [...list.children].find(b => b.textContent === String(at));
  const beside = at => Math.abs(mid(document.getElementById('card')) - mid(key(at))) < 0.5;
  const rect = () => window.__calls.filter(c => c[0] === 'card').pop()[1].rect;
  // As short as the real window makes a capsule of more than four keys: the list scrolls
  document.getElementById('pill').style.height = '260px';
  await sleep(50);
  // A key is held; meanwhile its action fails and the keys change places. The release, and the
  // loss of the pointer that follows every release, leave the words up, beside the key where
  // it stands now
  key(1).dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 10 }));
  window.__on.failed({ payload: { at: 1, title: 'Не сработало', why: 'Не открылось' } });
  window.__view = { ...window.__view, items: [2, 1, 0, 3, 4].map(i => window.__view.items[i]) };
  window.__on.reload();
  window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 10 }));
  document.getElementById('keys').dispatchEvent(new PointerEvent('lostpointercapture', { pointerId: 1 }));
  await sleep(100);
  out.order = [...list.children].map(b => b.textContent).join('');
  out.late = label(); out.lateBeside = out.late && beside(1);
  // The list scrolls a little: the words go with the key, and Rust is told their new rectangle.
  // This browser draws no frames, so the scroll is announced by hand
  const before = rect();
  list.scrollTop = 30; list.dispatchEvent(new Event('scroll')); out.scrolledTo = list.scrollTop;
  out.scrolled = label(); out.scrolledBeside = out.scrolled && beside(1); out.rectMoved = before[1] - rect()[1];
  // The key leaves the part of the list that shows: the words go
  list.scrollTop = 500; list.dispatchEvent(new Event('scroll'));
  out.gone = label(); out.regionClosed = window.__calls.filter(c => c[0] === 'card').pop()[1].open;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const lt = found(renderedDom([0, 1, 2, 3, 4].map(at => ({ at, name: String(at), icon: null })), lateScript, 3000));

test('story 34: the words outlive the release and a drawing that was put off', () => {
  assert.equal(lt.order, '21034');
  assert.equal(lt.late, 'Не сработало|Не открылось');
  assert.equal(lt.lateBeside, true);
});

test('story 34: the words go with their key when the list scrolls, and close when it is out of sight', () => {
  assert.equal(lt.scrolledTo, 30);
  assert.equal(lt.scrolled, 'Не сработало|Не открылось');
  assert.equal(lt.scrolledBeside, true);
  assert.equal(lt.rectMoved, 30);
  assert.equal(lt.gone, null);
  assert.equal(lt.regionClosed, false);
});

// ---------- the foot and the folded capsule
// Rust sizes the window, folds the capsule and saves its place, and Windows carries the folded
// key; here the stand-in for Rust answers a fold the way Rust does, with a folded view.
const foldingScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const foot = () => [...document.querySelectorAll('#keys > .item.add')];
  const long = () => foot().reduce((sum, b) => sum + b.getBoundingClientRect().height, 0);
  const pill = document.getElementById('pill');
  const label = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent; };
  const at = (type, x, y) => new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: x, clientY: y });
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  const tap = el => { const [x, y] = mid(el); el.dispatchEvent(at('pointerdown', x, y)); window.dispatchEvent(at('pointerup', x, y)); };
  await sleep(50);
  // At rest the foot is the plus; a tap puts the gear over the arrow, one sign longer in all
  out.rest = [foot().length, long()];
  tap(foot()[0]);
  out.open = [foot().length, long(), foot()[0].getBoundingClientRect().bottom <= foot()[1].getBoundingClientRect().top];
  out.footCalls = calls('foot').map(c => c.open); out.tapsOnPlus = calls('tap').length;
  // The gear asks for the settings window, the way the plus used to
  tap(foot()[0]);
  out.gear = calls('tap').map(c => c.index); out.afterGear = foot().length;
  // Four seconds after the last touch of a sign the plus is back
  await sleep(3900); out.at3900 = foot().length;
  await sleep(200); out.at4100 = foot().length; out.closed = calls('foot').pop().open;
  // A touch of any key puts the plus back at once
  tap(foot()[0]); out.again = foot().length;
  tap(document.querySelector('#list .item')); await sleep(30);
  out.afterKey = foot().length; out.keyTap = calls('tap').pop().index;
  // The arrow asks Rust to fold; Rust answers with a folded view
  tap(foot()[0]); tap(foot()[1]);
  out.fold = calls('fold').length;
  window.__view = { ...window.__view, folded: true }; window.__on.reload(); await sleep(50);
  const key = () => document.querySelector('.item.unfold');
  out.folded = [document.querySelectorAll('.item').length, !!key(), getComputedStyle(document.getElementById('grip')).display, pill.getBoundingClientRect().top, pill.getBoundingClientRect().height];
  // No label of any kind: not for a long press, a hover, a failure or a program's keys
  let [x, y] = mid(key());
  key().dispatchEvent(at('pointerdown', x, y)); await sleep(450); out.longLabel = label();
  // Held and not moved, it still brings the capsule back, like a tap
  window.dispatchEvent(at('pointerup', x, y)); out.unfoldAfterLong = calls('unfold').length;
  tap(key()); out.unfoldAfterTap = calls('unfold').length;
  window.__on.hover({ payload: [x, y] }); await sleep(400); out.hoverLabel = label(); window.__on.hover({ payload: null });
  window.__on.failed({ payload: { at: 0, title: 'Не сработало', why: 'Не открылось' } }); out.failLabel = label();
  window.__on.set({ payload: 'Клавиши для Paint Studio' }); await sleep(50); out.noteLabel = label();
  out.cards = calls('card').filter(c => c.open).length;
  // Windows carries the folded key, not the page: a press that reached the page and moved,
  // held first or not, carries nothing and does not bring the capsule back
  key().dispatchEvent(at('pointerdown', x, y)); await sleep(450);
  window.dispatchEvent(at('pointermove', x, y + 100)); window.dispatchEvent(at('pointerup', x, y + 100));
  key().dispatchEvent(at('pointerdown', x, y)); window.dispatchEvent(at('pointermove', x, y + 40)); window.dispatchEvent(at('pointerup', x, y + 40));
  out.moved = [calls('unfold').length, calls('tap').length, calls('move_item').length, pill.getBoundingClientRect().top];
  // At the top edge the folded key stands along the row
  window.__view = { ...window.__view, edge: 'top' }; window.__on.reload(); await sleep(50);
  const r = pill.getBoundingClientRect();
  out.top = [r.left, r.width, r.height, r.top];
  // Unfolded by Rust, the capsule is whole again, with the plus
  window.__view = { ...window.__view, edge: 'right', folded: false }; window.__on.reload(); await sleep(50);
  out.back = [document.querySelectorAll('#list .item').length, foot().length, document.body.classList.contains('folded')];
  // A capsule that could not fold says so beside its foot
  window.__on.failed({ payload: { at: null, title: 'Не сработало', why: 'Windows не приняла нажатие' } });
  out.foldFailed = label(); out.foldFailedBeside = Math.abs(mid(document.getElementById('card'))[1] - mid(foot()[0])[1]) < 0.5;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const fd = found(renderedDom(items, foldingScript, 12000));

test('story 35: the plus opens the gear and the fold arrow, and they go after four seconds or a tap on a key', () => {
  // 38 px at rest and two signs of 38 px when open: `ADD` in layout.rs, which sizes the window
  assert.deepEqual(fd.rest, [1, 38]);
  assert.deepEqual(fd.open, [2, 76, true]);
  assert.deepEqual(fd.footCalls, [true]);
  assert.equal(fd.tapsOnPlus, 0);
  assert.deepEqual(fd.gear, [null]);
  assert.equal(fd.afterGear, 2);
  assert.equal(fd.at3900, 2);
  assert.equal(fd.at4100, 1);
  assert.equal(fd.closed, false);
  assert.equal(fd.again, 2);
  assert.equal(fd.afterKey, 1);
  assert.equal(fd.keyTap, 0);
  assert.equal(fd.fold, 1);
});

test('story 36: the folded capsule is one key with no label; a touch brings the capsule back', () => {
  // One key, no handle and no plus, the short pill 12 px into the window
  assert.deepEqual(fd.folded, [1, true, 'none', 12, 60]);
  assert.equal(fd.longLabel, null);
  assert.equal(fd.unfoldAfterLong, 1);
  assert.equal(fd.unfoldAfterTap, 2);
  assert.equal(fd.hoverLabel, null);
  assert.equal(fd.failLabel, null);
  assert.equal(fd.noteLabel, null);
  assert.equal(fd.cards, 0);
  assert.deepEqual(fd.moved, [2, 2, 0, 12]);
  assert.deepEqual(fd.top, [12, 60, 60, 0]);
  assert.deepEqual(fd.back, [5, 1, false]);
  assert.equal(fd.foldFailed, 'Не сработало');
  assert.equal(fd.foldFailedBeside, true);
});

// ---------- the signs move nothing, and a key under the fading end is not tapped
// Eight items, four of them showing, and the pill as long as Rust says (`pill_length` in
// layout.rs): 319 px with the plus, 401 px with the two signs and the end grip. The window here
// never changes.
const steadyScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const list = document.getElementById('list');
  const rows = () => [...list.children];
  const foot = () => [...document.querySelectorAll('#keys > .item.add')];
  const label = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent; };
  const at = (type, x, y) => new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: x, clientY: y });
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  const tap = el => { const [x, y] = mid(el); el.dispatchEvent(at('pointerdown', x, y)); window.dispatchEvent(at('pointerup', x, y)); };
  const places = () => [...rows(), foot()[0]].map(b => { const r = b.getBoundingClientRect(); return [r.left, r.top]; });
  // This browser draws no frames: a scroll the page asks for is done at once and announced by
  // hand, and how it was asked for is kept
  const asked = [], scrollBy = list.scrollBy.bind(list);
  list.scrollBy = how => { asked.push(how.behavior); scrollBy({ ...how, behavior: 'auto' }); list.dispatchEvent(new Event('scroll')); };
  window.__view = { ...window.__view, pill: [319, 401] }; window.__on.reload(); await sleep(50);
  const pill = document.getElementById('pill');
  out.rest = [pill.getBoundingClientRect().height, foot().length];
  const before = places();
  tap(foot()[0]);
  out.open = [pill.getBoundingClientRect().height, foot().length];
  out.openMoved = places().filter((p, i) => p[0] !== before[i][0] || p[1] !== before[i][1]).length;
  await sleep(4200);
  out.closed = [pill.getBoundingClientRect().height, foot().length];
  out.closedMoved = places().filter((p, i) => p[0] !== before[i][0] || p[1] !== before[i][1]).length;
  // The fourth key shows whole and is tapped; the fifth peeks out from under the fading end
  tap(rows()[3]); out.wholeTap = calls('tap').map(c => c.index);
  const veiled = rows()[4];
  let [x, y] = mid(veiled);
  veiled.dispatchEvent(at('pointerdown', x, y)); out.dips = veiled.classList.contains('down');
  window.dispatchEvent(at('pointerup', x, y));
  out.veiledTap = [calls('tap').length, veiled.classList.contains('flash'), list.scrollTop, asked.join()];
  tap(rows()[4]); out.tapAfterReveal = calls('tap').pop().index;
  // Held, a veiled key shows no label and is not carried; a finger moving on it scrolls the list
  [x, y] = mid(rows()[5]);
  rows()[5].dispatchEvent(at('pointerdown', x, y)); await sleep(450); out.heldLabel = label();
  window.dispatchEvent(at('pointermove', x, y - 30)); list.dispatchEvent(new Event('scroll'));
  out.dragged = [list.scrollTop, document.querySelectorAll('.carried').length];
  window.dispatchEvent(at('pointerup', x, y - 30)); await sleep(50);
  out.afterDrag = [calls('tap').length, calls('move_item').length];
  // The other end: at the end of the list the fourth key is under the fading start
  list.scrollTop = 9999; list.dispatchEvent(new Event('scroll')); out.end = list.scrollTop;
  tap(rows()[3]);
  out.start = [calls('tap').length, list.scrollTop];
  tap(rows()[3]); out.startTap = calls('tap').pop().index;
  // Lying along the top edge the list goes sideways
  window.__view = { ...window.__view, edge: 'top' }; window.__on.reload(); await sleep(50);
  out.flat = [pill.getBoundingClientRect().width, pill.getBoundingClientRect().height];
  const taps = calls('tap').length;
  tap(rows()[4]);
  out.flatVeiled = [calls('tap').length - taps, list.scrollLeft];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const sd = found(renderedDom(Array.from({ length: 8 }, (_, at) => ({ at, name: 'Клавиша ' + at, icon: null })), steadyScript, 12000));

test('the signs open and close without moving a key of the list or the first sign', () => {
  assert.deepEqual(sd.rest, [319, 1]);
  assert.deepEqual(sd.open, [401, 2]);
  assert.equal(sd.openMoved, 0);
  assert.deepEqual(sd.closed, [319, 1]);
  assert.equal(sd.closedMoved, 0);
});

// The signs stay while the mouse is over the capsule, and go SIGNS_MS after it leaves; a finger
// leaves the cursor where it touched, and they go as ever
const hoverSignsScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const feet = () => document.querySelectorAll('#keys > .item.add').length;
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  const tap = (el, pointerType) => { const [x, y] = mid(el); const at = t => new PointerEvent(t, { bubbles: true, pointerId: 1, pointerType, clientX: x, clientY: y }); el.dispatchEvent(at('pointerdown')); window.dispatchEvent(at('pointerup')); return [x, y]; };
  const plus = () => document.querySelector('#keys > .item.add');
  window.__view = { ...window.__view, pill: [319, 401] }; window.__on.reload(); await sleep(50);
  const key = mid(document.querySelector('#list .item'));
  // The mouse taps the plus and stays over the keys: the signs stay past their time
  tap(plus(), 'mouse'); window.__on.hover({ payload: key });
  await sleep(4500); out.over = feet();
  // Over the transparent rest of the window, it has left the capsule: they go after their time
  window.__on.hover({ payload: [10, key[1]] });
  await sleep(3800); out.besideEarly = feet();
  await sleep(500); out.besideLate = feet();
  // A finger taps the plus; Windows puts the cursor where it touched
  window.__on.hover({ payload: null });
  const touched = tap(plus(), 'touch'); window.__on.hover({ payload: touched });
  await sleep(4300); out.touch = feet();
  // Over the capsule for long, then away from the window: their time starts when it leaves
  window.__on.hover({ payload: null });
  tap(plus(), 'mouse'); window.__on.hover({ payload: key });
  await sleep(5000); out.long = feet();
  window.__on.hover({ payload: null });
  await sleep(3800); out.leftEarly = feet();
  await sleep(500); out.leftLate = feet();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const hs = found(renderedDom(Array.from({ length: 8 }, (_, at) => ({ at, name: 'Клавиша ' + at, icon: null })), hoverSignsScript, 26000));

test('the signs stay while the mouse is over the capsule and go after it leaves; a finger changes nothing', () => {
  assert.equal(hs.over, 2);
  assert.deepEqual([hs.besideEarly, hs.besideLate], [2, 1]);
  assert.equal(hs.touch, 1);
  assert.equal(hs.long, 2);
  assert.deepEqual([hs.leftEarly, hs.leftLate], [2, 1]);
});

test('a key the fading end hides by more than a third is not tapped: the list scrolls to it', () => {
  assert.deepEqual(sd.wholeTap, [3]);
  assert.equal(sd.dips, false);
  // No tap and no flash, and the list goes on by one key, smoothly
  assert.deepEqual(sd.veiledTap, [1, false, 54, 'smooth']);
  assert.equal(sd.tapAfterReveal, 4);
  assert.equal(sd.heldLabel, null);
  assert.deepEqual(sd.dragged, [84, 0]);
  assert.deepEqual(sd.afterDrag, [2, 0]);
  // 8 keys of 54 px in a list of 234 px end at 198; the fourth then starts 18 px into the list
  assert.equal(sd.end, 198);
  assert.deepEqual(sd.start, [2, 144]);
  assert.equal(sd.startTap, 3);
  assert.deepEqual(sd.flat, [319, 60]);
  assert.deepEqual(sd.flatVeiled, [0, 54]);
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

// The meters under the keys: shown only when there are some; the divider is pulled; a key whose
// program has a meter shows it on its rim
const metered = [
  { at: 0, name: 'Render', icon: null, ring: 0 },
  { at: 1, name: 'Снимок', icon: null },
  { at: 2, name: 'Calc', icon: null },
];
const meterScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const q = s => document.querySelector(s);
  const card = () => { const c = q('#card'); return c.hidden ? null : [c.querySelector('.c-title').textContent, c.querySelector('.c-sub').textContent]; };
  const fire = (el, type) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 2, pointerType: 'touch', clientX: 10, clientY: 10 }));
  // What Rust sends for a meter: a percent, and from a file an outer ring and lines of hint
  const rd = (value, outer = null, hint = []) => value == null ? null : { value, outer, hint };
  const out = {};
  await sleep(50);
  out.plain = [!!q('#divider'), !!q('#meters'), document.querySelectorAll('.ring').length, q('#list').style.flex];
  // The room Rust gives the keys and the meters (240 px) and the pill it makes of them: the handle,
  // the line and the foot are 99 px more, the signs 82 more again
  window.__view = { ...window.__view, items: ${JSON.stringify(metered)}, pill: [339, 421], room: [240, 240], generation: 1, no_data: 'нет данных',
    meters: [{ name: 'Render', icon: null, target: 'D:/Tools/render.exe', value: 42 }, { name: 'backup', icon: null, target: null, value: null }] };
  window.__on.reload(); await sleep(50);
  const ms = [...document.querySelectorAll('.meter')], keys = [...document.querySelectorAll('#list .item')];
  out.block = { divider: !!q('#keys > #divider'), meters: ms.length, pct: ms.map(m => m.querySelector('.pct').textContent),
    none: ms.map(m => m.classList.contains('none')), letters: ms.map(m => m.querySelector('.dial span').textContent),
    below: q('#meters').getBoundingClientRect().top >= q('#list').getBoundingClientRect().bottom,
    foot: q('#meters').nextElementSibling.classList.contains('add') };
  out.rings = keys.map(k => !!k.querySelector('svg.ring'));
  // Chrome writes the dashes back with a comma
  const dash = el => el.querySelector('.arc').style.strokeDasharray.replaceAll(',', '');
  out.ringBefore = dash(keys[0].querySelector('.ring'));
  // A file of one number looks as it did: no outer ring, the dial and its cell of the same size
  const size = el => { const b = el.getBoundingClientRect(); return [Math.round(b.width), Math.round(b.height)]; };
  const outerShown = m => getComputedStyle(m.querySelector('svg.outer')).display !== 'none';
  out.oneNumber = { outer: ms.map(outerShown), dial: size(ms[0].querySelector('.dial')), cell: size(ms[0])[1] };
  // A meter is a rounded square: its value runs along its outline from the middle of its top, and
  // the second outline is a square too; nothing is drawn as a circle
  const arcOf = svg => svg.querySelector('.arc');
  out.square = { own: arcOf(ms[0].querySelector('.dial > svg:not(.outer)')).getAttribute('d').slice(0, 10), length: arcOf(ms[0].querySelector('.dial')).getAttribute('pathLength'),
    outer: arcOf(ms[0].querySelector('svg.outer')).getAttribute('d').slice(0, 11), outerLength: arcOf(ms[0].querySelector('svg.outer')).getAttribute('pathLength'),
    corner: getComputedStyle(ms[0].querySelector('.dial')).borderRadius, circles: document.querySelectorAll('circle').length };
  window.__on.meters({ payload: { generation: 1, values: [rd(85), rd(30)] } });
  out.after = { dash: dash(keys[0].querySelector('.ring')), warn: keys[0].querySelector('.ring').classList.contains('warn'),
    pct: ms.map(m => m.querySelector('.pct').textContent), none: ms.map(m => m.classList.contains('none')),
    dial: [ms[0].querySelector('.dial').classList.contains('warn'), ms[1].querySelector('.dial').classList.contains('warn')] };
  window.__on.meters({ payload: { generation: 1, values: [rd(80), null] } });
  out.at80 = [keys[0].querySelector('.ring').classList.contains('warn'), ms[1].querySelector('.pct').textContent];
  // Values read for another list than the one drawn go to no meter
  window.__on.meters({ payload: { generation: 0, values: [rd(5), rd(6)] } });
  window.__on.meters({ payload: { generation: 2, values: [rd(7), rd(8)] } });
  out.stale = [ms[0].querySelector('.pct').textContent, ms[1].querySelector('.pct').textContent, dash(keys[0].querySelector('.ring'))];
  // A file's second number: a thin ring outside the dial, without a percent of its own, in the
  // warning colour above 80; the dial and the cell keep their size
  window.__on.meters({ payload: { generation: 1, values: [rd(42, 85, ['CPU 42 %', 'GPU 85 %']), rd(30)] } });
  const outerOf = m => m.querySelector('svg.outer');
  out.ringed = { outer: ms.map(outerShown), dash: dash(outerOf(ms[0])), warn: outerOf(ms[0]).classList.contains('warn'),
    dialDash: dash(ms[0].querySelector('.dial')), dialWarn: ms[0].querySelector('.dial').classList.contains('warn'),
    pct: ms.map(m => m.querySelector('.pct').textContent), dial: size(ms[0].querySelector('.dial')), ring: size(outerOf(ms[0])), cell: size(ms[0])[1],
    stroke: getComputedStyle(outerOf(ms[0]).querySelector('.arc')).strokeWidth,
    // The ring with its stroke stays inside its cell
    inside: outerOf(ms[0]).querySelector('.arc').getBoundingClientRect().top - 0.75 >= ms[0].getBoundingClientRect().top - 0.25 };
  window.__on.meters({ payload: { generation: 1, values: [rd(42, 60, ['CPU 42 %', 'GPU 85 %']), rd(30)] } });
  out.ringedCalm = outerOf(ms[0]).classList.contains('warn');
  // Held, the meter's label shows the hint under the name and percent, a line each; a new hint
  // comes into the open label; a meter without one shows none
  const hint = () => [...q('#card .c-hint').children].map(d => d.textContent);
  fire(ms[0], 'pointerdown'); await sleep(450);
  out.hinted = { card: card(), hint: hint(), width: q('#card').getBoundingClientRect().width };
  window.__on.meters({ payload: { generation: 1, values: [rd(43, 60, ['CPU 43 %']), rd(30)] } });
  out.hintChanged = [card(), hint()];
  fire(ms[0], 'pointerup');
  fire(ms[1], 'pointerdown'); await sleep(450);
  out.unhinted = [card(), hint(), getComputedStyle(q('#card .c-hint')).display];
  fire(ms[1], 'pointerup');
  // Back to the values the rest of this run goes by: one number each
  window.__on.meters({ payload: { generation: 1, values: [rd(80), null] } });
  out.ringGone = ms.map(outerShown);
  // The parts as they stand: the three keys whole, the meters in what is left, the pill as Rust said
  const heights = () => [Math.round(q('#list').getBoundingClientRect().height), Math.round(q('#meters').getBoundingClientRect().height), Math.round(q('#pill').getBoundingClientRect().height)];
  out.parts = heights();
  // The divider follows the finger up: the keys scroll in less, the meters grow to all of them, and
  // past that the pill is shorter, with nothing empty below the meters. Down, it stops under the
  // last key, and the meters give up room. Let go, its place goes to Rust
  const list = q('#list'), d = q('#divider'), r = d.getBoundingClientRect();
  const pull = (y, type) => d.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 3, pointerType: 'touch', clientX: r.left + 20, clientY: y }));
  pull(r.top + 7, 'pointerdown'); pull(r.top - 13, 'pointermove');
  out.up20 = heights();
  pull(r.top - 53, 'pointermove');
  out.up60 = heights();
  pull(r.top + 47, 'pointermove');
  out.down40 = heights();
  pull(r.top - 53, 'pointermove');
  out.sentWhilePulled = window.__calls.filter(c => c[0] === 'set_divider').length;
  // The settings are read again while the line is pulled: the drawing waits for the release, and
  // comes once Rust has the new place
  const views = () => window.__calls.filter(c => c[0] === 'get_view').length;
  const flex = list.style.flex, viewsBefore = views();
  window.__on.reload(); await sleep(50);
  out.keptWhilePulled = [list.style.flex === flex, views() - viewsBefore];
  window.__view = { ...window.__view, divider: 102 / 240, pill: [305, 387] };
  pull(r.top - 53, 'pointerup'); await sleep(50);
  out.drawnAfter = [views() - viewsBefore, q('#list').style.flex === flex, heights()];
  out.sent = window.__calls.filter(c => c[0] === 'set_divider').map(c => c[1].at);
  out.taps = window.__calls.filter(c => c[0] === 'tap').length;
  // Pulled as far as it goes, each part keeps room for one whole key or meter
  pull(r.top + 7, 'pointerdown'); pull(r.top + 900, 'pointermove'); pull(r.top + 900, 'pointerup');
  out.meterRoom = Math.round(q('#meters').getBoundingClientRect().height);
  await sleep(50);
  const d2 = q('#divider').getBoundingClientRect();
  const pull2 = (y, type) => q('#divider').dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 3, pointerType: 'touch', clientX: d2.left + 20, clientY: y }));
  pull2(d2.top + 7, 'pointerdown'); pull2(d2.top - 900, 'pointermove');
  out.keyRoom = Math.round(q('#list').getBoundingClientRect().height);
  pull2(d2.top - 900, 'pointerup'); await sleep(50);
  // A tap on a meter of a program opens it; a file's meter has nothing to open; held, a meter
  // shows its name and percent
  const opened = () => window.__calls.filter(c => c[0] === 'open_meter').map(c => c[1].index);
  // The release drew the meters anew
  const now = [...document.querySelectorAll(".meter")];
  fire(now[1], 'pointerdown'); fire(now[1], 'pointerup'); out.fileTap = opened();
  fire(now[0], 'pointerdown'); fire(now[0], 'pointerup'); out.programTap = opened();
  out.programTapWith = window.__calls.filter(c => c[0] === 'open_meter').map(c => [c[1].name, c[1].target]);
  // The settings are read again while a meter is pressed: the drawing waits for the release, so
  // the release opens the program of the meter it was on, and the drawing comes after it
  const pressed = [...document.querySelectorAll('.meter')][0], viewsAtPress = views();
  fire(pressed, 'pointerdown');
  window.__on.reload(); await sleep(50);
  out.waitWhilePressed = [views() - viewsAtPress, pressed.isConnected];
  fire(pressed, 'pointerup'); await sleep(50);
  out.drawnAfterRelease = [views() - viewsAtPress, opened().length];
  const first = document.querySelector('.meter');
  fire(first, 'pointerdown'); await sleep(450); out.held = card(); fire(first, 'pointerup'); out.released = card();
  out.tapsAfter = window.__calls.filter(c => c[0] === 'tap').length;
  // The place saved in the settings file is where the divider stands after a start
  window.__view = { ...window.__view, divider: 0.25 }; window.__on.reload(); await sleep(50);
  out.restored = [q('#list').style.flex, q('#meters').style.flex];
  // A place in the file that would leave a part less than one key or meter, or more than its keys,
  // is not drawn so
  window.__view = { ...window.__view, divider: 0.02 }; window.__on.reload(); await sleep(50);
  out.restoredLow = heights();
  window.__view = { ...window.__view, divider: 0.99 }; window.__on.reload(); await sleep(50);
  out.restoredHigh = heights();
  // ...nor a list that grows under a pulled line
  window.__view = { ...window.__view, meters: [...window.__view.meters, { name: 'third', icon: null, target: null, value: 3 }] }; window.__on.reload(); await sleep(50);
  out.grownList = heights();
  window.__view = { ...window.__view, divider: 0.25, meters: window.__view.meters.slice(0, 2) }; window.__on.reload(); await sleep(50);
  // A file not updated for longer than it allows: grey with its last percent, and its label says how
  // long ago and when it resets; a file with nothing in it is grey with no percent and says so
  window.__view = { ...window.__view, divider: null }; window.__on.reload(); await sleep(50);
  const gs = () => [...document.querySelectorAll('.meter')];
  window.__on.meters({ payload: { generation: 1, values: [{ value: 64, outer: 20, hint: ['CPU'], stale: true, notes: ['сброс через 2 ч 33 мин', 'обновлено 12 мин назад'] }, null] } });
  const look = m => [m.classList.contains('stale'), m.classList.contains('none'), m.querySelector('.pct').textContent, getComputedStyle(m.querySelector('.dial .arc')).stroke];
  out.staleLook = gs().map(look);
  out.freshStroke = (() => { const m = gs()[0]; m.classList.remove('stale'); const s = getComputedStyle(m.querySelector('.dial .arc')).stroke; m.classList.add('stale'); return s; })();
  const lines = () => [...q('#card .c-hint').children].map(d => d.textContent);
  fire(gs()[0], 'pointerdown'); await sleep(450);
  out.staleLabel = [card(), lines()];
  // The clock moves on with the file as it was: Rust sends the new words and the open label shows them
  window.__on.meters({ payload: { generation: 1, values: [{ value: 64, outer: 20, hint: ['CPU'], stale: true, notes: ['сброс через 2 ч 32 мин', 'обновлено 13 мин назад'] }, null] } });
  out.staleLater = lines();
  fire(gs()[0], 'pointerup');
  fire(gs()[1], 'pointerdown'); await sleep(450);
  out.noData = [card(), lines()];
  fire(gs()[1], 'pointerup');
  // Updated again: as before
  window.__on.meters({ payload: { generation: 1, values: [{ value: 65, outer: null, hint: [], stale: false, notes: [] }, null] } });
  out.freshAgain = look(gs()[0]);
  // A view copied before a newer event comes after it: the event's values stay; a newer view and
  // a newer event count, an older event does not
  const pct = () => gs()[0].querySelector('.pct').textContent;
  window.__on.meters({ payload: { generation: 1, readings: 5, values: [{ value: 70, outer: null, hint: [], stale: false, notes: [] }, null] } });
  window.__view = { ...window.__view, readings: 3, meters: [{ ...window.__view.meters[0], value: 42, stale: true }, window.__view.meters[1]] };
  window.__on.reload(); await sleep(50);
  out.lateView = [pct(), gs()[0].classList.contains('stale')];
  window.__on.meters({ payload: { generation: 1, readings: 4, values: [{ value: 10, outer: null, hint: [], stale: false, notes: [] }, null] } });
  out.oldEvent = pct();
  window.__view = { ...window.__view, readings: 6, meters: [{ ...window.__view.meters[0], value: 81, stale: false }, window.__view.meters[1]] };
  window.__on.reload(); await sleep(50);
  out.newView = pct();
  window.__on.meters({ payload: { generation: 1, readings: 7, values: [{ value: 82, outer: null, hint: [], stale: false, notes: [] }, null] } });
  out.newEvent = pct();
  window.__on.reload(); await sleep(50);
  // The press of the divider asks Rust for the room of the longest pill at once, a release without
  // a move gives it back; a moved release saves, which sizes the window
  const dv = q('#divider'), dr = dv.getBoundingClientRect();
  const press = (type, y) => dv.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 4, pointerType: 'touch', clientX: dr.left + 20, clientY: y }));
  const before = window.__calls.filter(c => c[0] === 'divider').length;
  const asked = () => window.__calls.filter(c => c[0] === 'divider').slice(before).map(c => c[1].pulled);
  const saves = window.__calls.filter(c => c[0] === 'set_divider').length;
  press('pointerdown', dr.top + 7); out.pressed = asked();
  press('pointerup', dr.top + 7); out.still = asked();
  press('pointerdown', dr.top + 7); press('pointermove', dr.top - 13); press('pointerup', dr.top - 13); await sleep(50);
  out.movedRelease = [asked(), window.__calls.filter(c => c[0] === 'set_divider').length - saves];
  // At the top edge the meters lie in a row after the keys
  window.__view = { ...window.__view, edge: 'top' }; window.__on.reload(); await sleep(50);
  const m2 = [...document.querySelectorAll('.meter')].map(m => m.getBoundingClientRect());
  out.flat = Math.abs(m2[0].top - m2[1].top) < 0.5 && m2[1].left - m2[0].left > 40 && q('#meters').getBoundingClientRect().left >= q('#list').getBoundingClientRect().right;
  // Folded, the capsule draws no meters; a value that comes late breaks nothing
  window.__view = { ...window.__view, edge: 'right', folded: true }; window.__on.reload(); await sleep(50);
  try { window.__on.meters({ payload: [5, 6] }); out.lateWhenFolded = [document.querySelectorAll('.meter').length, 'ok']; } catch (e) { out.lateWhenFolded = String(e); }
  // No meters: the capsule is as it was
  window.__view = { ...window.__view, folded: false, edge: 'right', meters: [], items: ${JSON.stringify(metered.map(({ ring, ...it }) => it))} };
  window.__on.reload(); await sleep(50);
  out.gone = [!!q('#divider'), !!q('#meters'), document.querySelectorAll('.ring').length, q('#list').style.flex];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const mt = found(renderedDom(items, meterScript, 6000));

test('story 38: the meters show under the keys behind a line, only when there are meters', () => {
  assert.deepEqual(mt.plain, [false, false, 0, '']);
  assert.deepEqual(mt.block, { divider: true, meters: 2, pct: ['42%', '—'], none: [false, true], letters: ['R', 'B'], below: true, foot: true });
  assert.deepEqual(mt.after.pct, ['85%', '30%']);
  assert.deepEqual(mt.after.none, [false, false]);
  assert.deepEqual(mt.after.dial, [true, false]);
  assert.equal(mt.at80[1], '—');
  // Values of another generation than the list drawn change nothing
  assert.deepEqual(mt.stale, ['80%', '—', '80 100']);
  assert.equal(mt.flat, true);
  assert.deepEqual(mt.lateWhenFolded, [0, 'ok']);
  assert.deepEqual(mt.gone, [false, false, 0, '']);
});

test('story 38: the line is pulled to share the room, its place goes to Rust and comes back', () => {
  // Three keys whole (162) and the meters in the rest of the 240 px, as layout.rs shares it
  assert.deepEqual(mt.parts, [162, 78, 339]);
  // Up: the meters grow to both (104), then the pill is shorter, nothing empty under them
  assert.deepEqual(mt.up20, [142, 98, 339]);
  assert.deepEqual(mt.up60, [102, 104, 305]);
  // Down: the line stops under the third key, no empty cell below it
  assert.deepEqual(mt.down40, [162, 78, 339]);
  assert.equal(mt.sentWhilePulled, 0);
  assert.deepEqual(mt.keptWhilePulled, [true, 0]);
  assert.deepEqual(mt.drawnAfter, [1, true, [102, 104, 305]]);
  assert.equal(mt.sent.length, 1);
  assert.ok(Math.abs(mt.sent[0] - 102 / 240) < 1e-9, String(mt.sent[0]));
  // Pulled as far as it goes, or restored from the file, or with more meters than before, each
  // part keeps one whole key (54) or meter (52), and the keys' part no more than its keys
  assert.ok(mt.meterRoom >= 52, String(mt.meterRoom));
  assert.ok(mt.keyRoom >= 54, String(mt.keyRoom));
  assert.equal(mt.taps, 0);
  assert.deepEqual(mt.restored, ['0 0 60px', '0 0 104px']);
  assert.deepEqual(mt.restoredLow.slice(0, 2), [54, 104]);
  assert.deepEqual(mt.restoredHigh.slice(0, 2), [162, 78]);
  assert.deepEqual(mt.grownList.slice(0, 2), [162, 78]);
});

test('a stale meter is grey with its last percent and says how old it is; one with no data says so', () => {
  const grey = 'rgb(139, 143, 154)';
  assert.deepEqual(mt.staleLook[0], [true, false, '64%', grey]);
  assert.notEqual(mt.freshStroke, grey);
  assert.deepEqual(mt.staleLook[1].slice(0, 3), [false, true, '—']);
  assert.deepEqual(mt.staleLabel, [['Render', '64%'], ['сброс через 2 ч 33 мин', 'обновлено 12 мин назад', 'CPU']]);
  assert.deepEqual(mt.staleLater, ['сброс через 2 ч 32 мин', 'обновлено 13 мин назад', 'CPU']);
  assert.deepEqual(mt.noData, [['backup', 'нет данных'], []]);
  assert.deepEqual(mt.freshAgain.slice(0, 3), [false, false, '65%']);
  assert.equal(mt.freshAgain[3], mt.freshStroke);
});

test('a drawing copied before the newest values does not put old ones back', () => {
  assert.deepEqual(mt.lateView, ['70%', false]);
  assert.equal(mt.oldEvent, '70%');
  assert.equal(mt.newView, '81%');
  assert.equal(mt.newEvent, '82%');
});

test('a press of the divider reserves the window for the longest pill, and only the release gives it back', () => {
  assert.deepEqual(mt.pressed, [true]);
  assert.deepEqual(mt.still, [true, false]);
  assert.deepEqual(mt.movedRelease, [[true, false, true], 1]);
});

test('story 38: a tap on a meter opens its program, and a held meter shows its name and percent', () => {
  assert.deepEqual(mt.fileTap, []);
  assert.deepEqual(mt.programTap, [0]);
  // The meter goes with its name and program, so Rust opens nothing when the list changed
  assert.deepEqual(mt.programTapWith, [['Render', 'D:/Tools/render.exe']]);
  // A drawing waits while a meter is pressed and comes after the release opened its program
  assert.deepEqual(mt.waitWhilePressed, [0, true]);
  assert.deepEqual(mt.drawnAfterRelease, [1, 2]);
  assert.deepEqual(mt.held, ['Render', '80%']);
  assert.equal(mt.released, null);
  assert.equal(mt.tapsAfter, 0);
});

test('story 39: a file of two numbers rings its meter, its lines of hint show in the label', () => {
  // One number: as it was
  assert.deepEqual(mt.oneNumber, { outer: [false, false], dial: [34, 34], cell: 52 });
  assert.deepEqual(mt.ringed, { outer: [true, false], dash: '85 100', warn: true, dialDash: '42 100', dialWarn: false,
    pct: ['42%', '30%'], dial: [34, 34], ring: [39, 39], cell: 52, stroke: '1.5px', inside: true });
  assert.equal(mt.ringedCalm, false);
  assert.deepEqual(mt.hinted.card, ['Render', '42%']);
  assert.deepEqual(mt.hinted.hint, ['CPU 42 %', 'GPU 85 %']);
  assert.ok(mt.hinted.width <= 260, String(mt.hinted.width));
  assert.deepEqual(mt.hintChanged, [['Render', '43%'], ['CPU 43 %']]);
  assert.deepEqual(mt.unhinted, [['backup', '30%'], [], 'none']);
  assert.deepEqual(mt.ringGone, [false, false]);
});

test('story 40: the key of a program with a meter shows it on its rim, other keys stay as they were', () => {
  assert.deepEqual(mt.rings, [true, false, false]);
  assert.equal(mt.ringBefore, '42 100');
  assert.equal(mt.after.dash, '85 100');
  assert.equal(mt.after.warn, true);
  // Above 80 % the warning colour, at 80 not yet
  assert.equal(mt.at80[0], false);
});

test('story 44: meters are rounded squares filled from the middle of the top, with a square second outline', () => {
  assert.deepEqual(mt.square, { own: 'M17 1.5H24', length: '100', outer: 'M19.5 .75H2', outerLength: '100', corner: '10px', circles: 0 });
});

// Keys without a meter: Rust says which programs show progress on the taskbar
const rimmed = [
  { at: 0, name: 'Files', icon: null, rim: 30 },
  { at: 1, name: 'Render', icon: null, ring: 0 },
  { at: 2, name: 'Calc', icon: null },
];
const rimScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  window.__view = { ...window.__view, meters: [{ name: 'Render', icon: null, target: 'C:/Tools/render.exe', value: 42 }], generation: 1, age: 7 };
  window.__on.reload(); await sleep(50);
  const rings = () => [...document.querySelectorAll('#list .item')].map(k => { const r = k.querySelector('.ring'); return r ? r.querySelector('.arc').style.strokeDasharray.replaceAll(',', '') : null; });
  out.drawn = rings();
  // Calc's program starts a copy, Files' copy goes on
  window.__on.rims({ payload: { age: 7, rims: [[0, 64], [2, 85]] } });
  out.started = rings();
  // Places of another list of items than the one drawn go to no key
  window.__on.rims({ payload: { age: 6, rims: [[1, 5]] } });
  window.__on.rims({ payload: { age: 8, rims: [] } });
  out.stale = rings();
  out.warn = document.querySelectorAll('#list .item')[2].querySelector('.ring').classList.contains('warn');
  // Both copies are over: their rims go; the key with a meter keeps its own
  window.__on.rims({ payload: { age: 7, rims: [] } });
  out.ended = rings();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const rm = found(renderedDom(rimmed, rimScript));

test('story 44: a key without a meter fills its rim while its program shows taskbar progress, and clears after', () => {
  assert.deepEqual(rm.drawn, ['30 100', '42 100', null]);
  assert.deepEqual(rm.started, ['64 100', '42 100', '85 100']);
  assert.deepEqual(rm.stale, rm.started);
  assert.equal(rm.warn, true);
  assert.deepEqual(rm.ended, [null, '42 100', null]);
});

// The card of programs adds three keys past the four in sight; the capsule asks for a tap beside
// them until the first tap on a key, or for six seconds
const addScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const four = ${JSON.stringify(items.slice(0, 4))};
  const seven = [...four, ...['Chrome', 'Chat', 'Terminal'].map((name, i) => ({ at: 4 + i, name, icon: null }))];
  const scroll = () => { const l = document.getElementById('list'); return document.body.dataset.edge === 'top' ? l.scrollLeft : l.scrollTop; };
  const lastShows = () => { const l = document.getElementById('list').getBoundingClientRect(), k = [...document.querySelectorAll('#list .item')].pop().getBoundingClientRect();
    return document.body.dataset.edge === 'top' ? k.right <= l.right + 0.5 : k.bottom <= l.bottom + 0.5; };
  const soft = () => document.getAnimations().length;
  const tip = () => { const c = document.getElementById('card'); return c.hidden ? null : c.querySelector('.c-title').textContent; };
  const mid = b => { const r = b.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  const fire = (b, type) => { const [x, y] = mid(b); b.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: x, clientY: y })); };
  const added = { at: [4, 5, 6], tip: 'Коснитесь клавиши' };
  // Headless Chrome does not run a smooth scroll to its end: the scroll asked for is made at once
  const list = document.getElementById('list'), scrollBy = list.scrollBy.bind(list);
  list.scrollBy = how => { scrollBy({ ...how, behavior: 'auto' }); list.dispatchEvent(new Event('scroll')); };
  for (const edge of ['right', 'top']) {
    window.__view = { ...window.__view, items: four, edge, pill: [400, 438] }; window.__on.reload(); await sleep(400);
    // Keys added by hand somewhere else: the list stays where it is, and nothing asks for a tap
    window.__view = { ...window.__view, items: seven }; window.__on.reload(); await sleep(400);
    const plain = [scroll(), lastShows(), tip()];
    window.__view = { ...window.__view, items: four }; window.__on.reload(); await sleep(400);
    // The card: Rust says which places it added, before the drawing that has them
    window.__on.added({ payload: added });
    window.__view = { ...window.__view, items: seven }; window.__on.reload(); await sleep(30);
    const during = soft();
    await sleep(600);
    out[edge] = { plain, scrolled: scroll() > 0, shows: lastShows(), soft: during > 0, tip: tip() };
    // ...and after a drawing that already has them
    window.__view = { ...window.__view, items: four }; window.__on.reload(); await sleep(400);
    window.__view = { ...window.__view, items: seven }; window.__on.reload(); await sleep(400);
    document.getElementById('list').scrollTo(0, 0); await sleep(50);
    window.__on.added({ payload: added }); await sleep(600);
    out[edge].late = [lastShows(), tip()];
    // A drawing closes the labels; the words come back after it
    window.__on.reload(); await sleep(100);
    out[edge].redrawn = tip();
    if (edge === 'right') {
      // The cursor over the keys leaves them up; the first tap on a key takes them away
      const box = list.getBoundingClientRect();
      const key = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2).closest('.item');
      window.__on.hover({ payload: mid(key) }); await sleep(400);
      out[edge].hovered = tip();
      window.__on.hover({ payload: null });
      fire(key, 'pointerdown'); await sleep(50); fire(key, 'pointerup'); await sleep(50);
      out[edge].tapped = [tip(), window.__calls.filter(c => c[0] === 'tap').length];
    } else {
      // A program brings its keys: its note takes the place of the words for a moment, and they
      // come back after it, keeping their own six seconds
      window.__on.set({ payload: 'Клавиши для Paint' }); await sleep(300);
      out[edge].note = tip();
      await sleep(2200); out[edge].back = tip();
      // Untouched, they go after six seconds
      await sleep(1500); out[edge].four = tip();
      await sleep(2500); out[edge].six = tip();
    }
  }
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const ad = found(renderedDom(items.slice(0, 4), addScript, 20000));

test('story 45: keys the card adds scroll into view on the capsule and come in softly, at the side and the top edge', () => {
  for (const edge of ['right', 'top']) {
    assert.deepEqual(ad[edge].plain, [0, false, null], edge);
    assert.deepEqual([ad[edge].scrolled, ad[edge].shows, ad[edge].soft, ad[edge].late[0]], [true, true, true, true], edge);
  }
});

test('story 45: after the card adds keys the capsule says to tap one, until a tap or six seconds', () => {
  for (const edge of ['right', 'top']) {
    assert.deepEqual([ad[edge].tip, ad[edge].late[1], ad[edge].redrawn], ['Коснитесь клавиши', 'Коснитесь клавиши', 'Коснитесь клавиши'], edge);
  }
  assert.equal(ad.right.hovered, 'Коснитесь клавиши');
  assert.deepEqual(ad.right.tapped, [null, 1]);
  assert.deepEqual([ad.top.note, ad.top.back], ['Клавиши для Paint', 'Коснитесь клавиши']);
  assert.deepEqual([ad.top.four, ad.top.six], ['Коснитесь клавиши', null]);
});

// ---------- the end grip
// Eight items and the lengths Rust sends for them (`pill_length` in layout.rs): 319 px at rest and
// 401 with the two signs and the grip; stretched with the signs, 185 px and 54 a cell, and the
// grip may go from two cells to seven (`reach`).
const reachOf = cells => cells.map(c => [c, 185 + 54 * c]);
const stretchScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const steps = () => calls('resize').filter(c => c.step === 'step');
  const views = () => calls('get_view').length;
  const keys = document.getElementById('keys'), pill = document.getElementById('pill');
  const grip = () => document.getElementById('stretch');
  const plus = () => document.querySelector('#keys > .item.add');
  const at = (type, x, y, id = 1) => new PointerEvent(type, { bubbles: true, pointerId: id, pointerType: 'touch', clientX: x, clientY: y });
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  const tap = el => { const [x, y] = mid(el); el.dispatchEvent(at('pointerdown', x, y)); window.dispatchEvent(at('pointerup', x, y)); };
  const long = () => flat() ? pill.getBoundingClientRect().width : pill.getBoundingClientRect().height;
  const flat = () => document.body.dataset.edge === 'top';
  window.__view = { ...window.__view, pill: [319, 401], reach: ${JSON.stringify(reachOf([2, 3, 4, 5, 6, 7]))}, length: null };
  window.__on.reload(); await sleep(50);
  // At rest there is no grip; the plus brings it with the gear and the arrow, last, at the far end
  out.rest = [!!grip(), long()];
  tap(plus());
  const fold = [...keys.querySelectorAll('.item.add')].pop();
  const g = grip().getBoundingClientRect(), p = pill.getBoundingClientRect();
  out.open = [!!grip(), keys.lastElementChild === grip(), grip().previousElementSibling === fold, Math.round(p.bottom - g.bottom), Math.round(g.height), long()];
  // It goes with the other signs, after four seconds untouched
  await sleep(3900); out.at3900 = !!grip();
  await sleep(300); out.at4200 = !!grip();
  // Dragged by a finger: a cell at a time, the pill as long as Rust said, the window sized by Rust
  tap(plus());
  const closes = calls('foot').filter(c => !c.open).length;
  let [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y));
  grip().dispatchEvent(at('pointermove', x, y + 20));
  out.small = [steps().length, long(), calls('resize')[0].step];
  grip().dispatchEvent(at('pointermove', x, y + 60));
  out.five = [steps().map(c => c.cells), long()];
  grip().dispatchEvent(at('pointermove', x, y + 115));
  out.six = [steps().map(c => c.cells), long()];
  // Further than fits: no more than Rust allows
  grip().dispatchEvent(at('pointermove', x, y + 900));
  out.most = [steps().map(c => c.cells), long()];
  grip().dispatchEvent(at('pointermove', x, y + 115));
  // Held past four seconds: no drawing, no save, the signs stay up
  const viewsBefore = views();
  await sleep(4300);
  out.held = [views() - viewsBefore, !!grip(), calls('set_length').length, calls('foot').filter(c => !c.open).length - closes];
  // Let go: one save of the snapped length, then the capsule is drawn anew
  window.__view = { ...window.__view, length: 6, pill: [427, 509] };
  grip().dispatchEvent(at('pointerup', x, y + 115)); await sleep(50);
  out.saved = [calls('set_length').map(c => c.cells), views() - viewsBefore, long(), calls('tap').length, calls('move_item').length];
  // A touch of the grip that does not move it saves nothing
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointerup', x, y)); await sleep(50);
  out.still = calls('set_length').length;
  // The capture is lost mid-drag but the pointer's events still come: the drag goes on and saves
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y - 110));
  grip().dispatchEvent(new PointerEvent('lostpointercapture', { bubbles: true, pointerId: 1 })); await sleep(50);
  window.dispatchEvent(at('pointermove', x, y - 170)); window.dispatchEvent(at('pointerup', x, y - 170)); await sleep(50);
  out.lost = calls('set_length').map(c => c.cells);
  // Taken away by Windows: nothing is saved, and Rust is told why, for the log
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y + 60));
  window.dispatchEvent(at('pointercancel', x, y + 60)); await sleep(50);
  out.cancelled = [calls('set_length').length, calls('resize').pop().why, grip().classList.contains('pulled')];
  // The capture lost and nothing after it: the drag ends after a moment, without a save
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y + 60));
  grip().dispatchEvent(new PointerEvent('lostpointercapture', { bubbles: true, pointerId: 1 })); await sleep(300);
  out.lostAlive = calls('resize').pop().step;
  await sleep(500);
  out.lostQuiet = [calls('set_length').length, calls('resize').pop().why];
  // Rust ends the drag (the keyboard went, another program came to the front): nothing is saved,
  // and the capsule is drawn anew
  await sleep(50);
  [x, y] = mid(grip());
  const viewsAtDrop = views(), sentAtDrop = calls('set_length').length;
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y + 60));
  window.__on.stretch({ payload: 'the keyboard' }); await sleep(50);
  grip().dispatchEvent(at('pointermove', x, y + 120)); grip().dispatchEvent(at('pointerup', x, y + 120)); await sleep(50);
  out.dropped = [grip().classList.contains('pulled'), calls('set_length').length - sentAtDrop, views() - viewsAtDrop];
  // Shortened with the meters and the divider pulled far to the keys: each part keeps one whole key
  // or meter while the grip is dragged
  window.__view = { ...window.__view, length: 6, pill: [545, 627], divider: 0.9, generation: 1, reach: ${JSON.stringify([2, 3, 4, 5, 6, 7].map(c => [c, 185 + 54 * c + 118]))},
    meters: [{ name: 'Render', icon: null, target: 'D:/Tools/render.exe', value: 42 }, { name: 'backup', icon: null, target: null, value: null }] };
  window.__on.reload(); await sleep(50);
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y - 216));
  out.shortMeters = [long(), Math.round(document.getElementById('list').getBoundingClientRect().height), Math.round(document.getElementById('meters').getBoundingClientRect().height)];
  grip().dispatchEvent(at('pointerup', x, y - 216)); await sleep(50);
  out.shortSaved = calls('set_length').pop().cells;
  // The length in the file shares the room with the meters: six cells and the peek against two meters
  window.__view = { ...window.__view, length: 6, pill: [545, 627], divider: null, generation: 1, reach: ${JSON.stringify(reachOf([2, 3, 4, 5, 6, 7]))},
    meters: [{ name: 'Render', icon: null, target: 'D:/Tools/render.exe', value: 42 }, { name: 'backup', icon: null, target: null, value: null }] };
  window.__on.reload(); await sleep(50);
  out.restored = [long(), Math.round(document.getElementById('list').getBoundingClientRect().height), Math.round(document.getElementById('meters').getBoundingClientRect().height)];
  // At the top edge the grip is at the right end and is dragged sideways
  window.__view = { ...window.__view, edge: 'top', meters: [], length: null, pill: [319, 401] };
  window.__on.reload(); await sleep(50);
  if (!grip()) tap(plus());
  const t = grip().getBoundingClientRect(), q = pill.getBoundingClientRect();
  out.top = [Math.round(q.right - t.right), Math.round(t.width), long()];
  // The grip is the right end of the pill and nothing past it
  out.topZone = [q.right - 5, q.right - 46, q.right + 5].map(x => document.elementFromPoint(x, q.top + 30)?.id || null);
  const sent = calls('set_length').length;
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x - 50, y));
  grip().dispatchEvent(at('pointerup', x - 50, y)); await sleep(50);
  out.topSaved = calls('set_length').slice(sent).map(c => c.cells);
  // Folded: no grip
  window.__view = { ...window.__view, edge: 'right', folded: true }; window.__on.reload(); await sleep(50);
  out.folded = !!grip();
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const st = found(renderedDom(Array.from({ length: 8 }, (_, at) => ({ at, name: 'Клавиша ' + at, icon: null })), stretchScript, 16000));

// One key and no length: the pill (139 px, 221 with the signs) is shorter than the least step of
// two cells (275 with the signs); a real move grows it to two cells, a touch does nothing
const oneKeyScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const steps = () => calls('resize').filter(c => c.step === 'step');
  const pill = document.getElementById('pill'), grip = () => document.getElementById('stretch');
  // As Rust may: each resize is followed by a new set of keys and a reload, which wait for the release
  const ask = window.__TAURI__.core.invoke;
  window.__TAURI__.core.invoke = async (c, a) => { const r = await ask(c, a); if (c === 'resize') { window.__on.set({ payload: null }); window.__on.reload(); } return r; };
  const at = (type, x, y) => new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'mouse', clientX: x, clientY: y });
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  window.__view = { ...window.__view, pill: [139, 221], reach: ${JSON.stringify(reachOf([2, 3, 4]).map(([c, l]) => [c, l - 18]))}, length: null };
  window.__on.reload(); await sleep(50);
  const plus = document.querySelector('#keys > .item.add'), [px, py] = mid(plus);
  plus.dispatchEvent(at('pointerdown', px, py)); window.dispatchEvent(at('pointerup', px, py));
  let [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointerup', x, y)); await sleep(50);
  out.touch = [steps().length, calls('set_length').length, pill.getBoundingClientRect().height, calls('resize').map(c => c.step)];
  grip().dispatchEvent(at('pointerdown', x, y)); grip().dispatchEvent(at('pointermove', x, y + 5));
  out.nudge = [steps().length, pill.getBoundingClientRect().height];
  grip().dispatchEvent(at('pointermove', x, y + 40));
  await sleep(50);
  out.grown = [steps().map(c => c.cells), pill.getBoundingClientRect().height, !!grip() && grip().classList.contains('pulled')];
  grip().dispatchEvent(at('pointerup', x, y + 40)); await sleep(50);
  out.saved = calls('set_length').map(c => c.cells);
  // The mouse's movement may not reach the page at all: the cursor Rust reports moves the grip
  [x, y] = mid(grip());
  grip().dispatchEvent(at('pointerdown', x, y));
  window.__on.hover({ payload: [x, y + 100] }); await sleep(50);
  grip().dispatchEvent(at('pointerup', x, y + 100)); await sleep(50);
  out.byCursor = calls('set_length').map(c => c.cells);
  // A finger: only its own events move the grip; the cursor Rust reports stays elsewhere and is
  // ignored. The press neither taps the foot nor scrolls the list
  const by = (type, x, y, pointerType) => new PointerEvent(type, { bubbles: true, pointerId: 5, pointerType, clientX: x, clientY: y });
  const [taps, feet] = [calls('tap').length, calls('foot').length];
  [x, y] = mid(grip());
  grip().dispatchEvent(by('pointerdown', x, y, 'touch'));
  window.__on.hover({ payload: [x, y + 200] });
  grip().dispatchEvent(by('pointermove', x, y + 30, 'touch'));
  window.__on.hover({ payload: [x, y + 200] });
  grip().dispatchEvent(by('pointermove', x, y + 60, 'touch'));
  window.__on.hover({ payload: [x, y + 200] }); await sleep(50);
  out.touchDrawn = pill.getBoundingClientRect().height;
  grip().dispatchEvent(by('pointerup', x, y + 60, 'touch')); await sleep(50);
  out.touch2 = [calls('set_length').map(c => c.cells), calls('resize').filter(c => c.step === 'start').pop().pointer, calls('tap').length - taps, calls('foot').length - feet, document.getElementById('list').scrollTop];
  // A pen, the same way
  [x, y] = mid(grip());
  grip().dispatchEvent(by('pointerdown', x, y, 'pen'));
  grip().dispatchEvent(by('pointermove', x, y + 110, 'pen'));
  window.__on.hover({ payload: [x, y + 300] });
  grip().dispatchEvent(by('pointerup', x, y + 110, 'pen')); await sleep(50);
  out.pen = [calls('set_length').map(c => c.cells), calls('resize').filter(c => c.step === 'start').pop().pointer];
  // A finger's target of 44 px, as large as the handle's, at the end of the pill and not past it
  const g = grip().getBoundingClientRect(), end = pill.getBoundingClientRect().bottom, handle = document.getElementById('grip').getBoundingClientRect();
  out.zone = [g.top + 1, g.top + 43, end + 3].map(y => document.elementFromPoint(g.left + g.width / 2, y)?.id || null);
  out.sameAsHandle = [g.width, g.height, handle.width, handle.height];
  for (const from of [g.top + 1, g.top + 43]) {
    const el = document.elementFromPoint(g.left + 30, from);
    el.dispatchEvent(by('pointerdown', g.left + 30, from, 'touch'));
    window.dispatchEvent(by('pointermove', g.left + 30, from + 60, 'touch'));
    window.dispatchEvent(by('pointerup', g.left + 30, from + 60, 'touch')); await sleep(50);
  }
  out.zoneDrags = [calls('set_length').map(c => c.cells).slice(-2), Math.round(end - g.top)];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const one = found(renderedDom(items.slice(0, 1), oneKeyScript, 3000));

// Four of eight keys showing; a finger drags the grip three cells on, a few pixels at a time. Only
// the drag's own calls go to Rust meanwhile: nothing that sizes the window or cuts its region
const sevenScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const out = {};
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const pill = document.getElementById('pill'), grip = () => document.getElementById('stretch');
  const by = (type, x, y) => new PointerEvent(type, { bubbles: true, pointerId: 7, pointerType: 'touch', clientX: x, clientY: y });
  const mid = el => { const r = el.getBoundingClientRect(); return [r.left + r.width / 2, r.top + r.height / 2]; };
  window.__view = { ...window.__view, pill: [319, 401], reach: ${JSON.stringify([...reachOf([2, 3, 4, 5, 6, 7]), [8, 599]])}, length: null };
  window.__on.reload(); await sleep(50);
  const plus = document.querySelector('#keys > .item.add'), [px, py] = mid(plus);
  plus.dispatchEvent(by('pointerdown', px, py)); window.dispatchEvent(by('pointerup', px, py)); await sleep(50);
  const [x, y] = mid(grip());
  const before = window.__calls.length;
  grip().dispatchEvent(by('pointerdown', x, y));
  for (let d = 9; d <= 162; d += 9) { window.dispatchEvent(by('pointermove', x, y + d)); await sleep(10); }
  out.during = [...new Set(window.__calls.slice(before).map(c => c[0]))];
  out.drawn = pill.getBoundingClientRect().height;
  window.dispatchEvent(by('pointerup', x, y + 162)); await sleep(50);
  out.saved = calls('set_length').map(c => c.cells);
  out.steps = calls('resize').filter(c => c.step === 'step').map(c => c.cells);
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const seven = found(renderedDom(Array.from({ length: 8 }, (_, at) => ({ at, name: 'Клавиша ' + at, icon: null })), sevenScript, 3000));

test('a finger drags the grip three cells on: one save of seven, nothing but the drag sent meanwhile', () => {
  assert.deepEqual(seven.during, ['resize']);
  assert.equal(seven.drawn, 563);
  assert.deepEqual(seven.steps, [5, 6, 7]);
  assert.deepEqual(seven.saved, [7]);
});

test('the end grip shows only with the signs, at the far end, and goes with them', () => {
  assert.deepEqual(st.rest, [false, 319]);
  // After the arrow, the last of the foot: 44 px like the handle (`STRETCH`), 3 px from the pill's end (`END`)
  assert.deepEqual(st.open, [true, true, true, 3, 44, 401]);
  assert.equal(st.at3900, true);
  assert.equal(st.at4200, false);
  assert.deepEqual(st.top, [3, 44, 401]);
  assert.deepEqual(st.topZone, ['stretch', 'stretch', null]);
  assert.equal(st.folded, false);
});

test('the end grip stretches the capsule a cell at a time and the release saves the length once', () => {
  // Less than half a cell moves nothing; then five cells, six, and no more than seven
  assert.deepEqual(st.small, [0, 401, 'start']);
  assert.deepEqual(st.five, [[5], 455]);
  assert.deepEqual(st.six, [[5, 6], 509]);
  assert.deepEqual(st.most, [[5, 6, 7], 563]);
  // Held: no drawing, no save, and the signs stay past their four seconds
  assert.deepEqual(st.held, [0, true, 0, 0]);
  // One save of six cells, then one drawing with the new length; no key was tapped or carried
  assert.deepEqual(st.saved, [[6], 1, 509, 0, 0]);
  assert.equal(st.still, 1);
  assert.deepEqual(st.lost, [6, 3]);
  assert.deepEqual(st.cancelled, [2, 'pointercancel', false]);
  assert.equal(st.lostAlive, 'step');
  assert.deepEqual(st.lostQuiet, [2, 'lost capture']);
  // Shorter at the top edge: three cells
  assert.deepEqual(st.topSaved, [3]);
});

test('a drag Rust ends is dropped without a save and the capsule is drawn anew', () => {
  // The grip lets go, nothing is saved, the capsule is drawn anew
  assert.deepEqual(st.dropped, [false, 0, 1]);
});

test('shortened over the meters, the keys and the meters each keep one whole cell while dragged', () => {
  // Two cells and the peek with two meters: 411 px with the signs
  assert.equal(st.shortMeters[0], 411);
  assert.ok(st.shortMeters[1] >= 54 && st.shortMeters[2] >= 52, String(st.shortMeters));
  assert.equal(st.shortSaved, 2);
});

test('a one-key capsule grows to two cells after a real move of the grip, not on a touch', () => {
  // A touch: the window grows for the drag and goes back, nothing is saved
  assert.deepEqual(one.touch, [0, 0, 221, ['start', 'end']]);
  assert.deepEqual(one.nudge, [0, 221]);
  // The new set and the reload Rust sends after each resize do not end the drag
  assert.deepEqual(one.grown, [[2], 275, true]);
  assert.deepEqual(one.saved, [2]);
  assert.deepEqual(one.byCursor, [2, 3]);
});

test('a finger or a pen drags the grip by its own events, and the cursor Rust reports is ignored', () => {
  // Two cells by the finger, whatever the cursor said; no tap, no foot, no scroll
  assert.equal(one.touchDrawn, 275);
  assert.deepEqual(one.touch2, [[2, 3, 2], 'touch', 0, 0, 0]);
  assert.deepEqual(one.pen, [[2, 3, 2, 3], 'pen']);
});

test('a finger starts the drag anywhere in the grip, a strip as large as the handle at the end of the pill', () => {
  // The grip is 44 px, as large as the handle, 3 px from the pill's end; past the end is not it
  assert.deepEqual(one.zone, ['stretch', 'stretch', null]);
  assert.deepEqual(one.sameAsHandle.slice(0, 2), one.sameAsHandle.slice(2));
  assert.deepEqual(one.zoneDrags, [[2, 2], 47]);
});

test('the length from the file draws the capsule and shares the room with the meters', () => {
  // The signs are still up: the pill with them; of its 446 px of room both meters take what they
  // need, the keys the rest
  assert.deepEqual(st.restored, [627, 342, 104]);
});

// ---------- the settings window
const glyph ='<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 4h16"/></svg>';

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
  version: '1.0.0', error: null,
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
  // A card saves after reading the settings again
  const save = () => { document.querySelector('#foot .acc').click(); return sleep(20); };
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
  out.find = [$('find').textContent, $('find-line').hidden, $('shade').classList.contains('on')];
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
  // A set from a file: two buttons under the ready sets. The file shows as one more card; its keys
  // go to the end of the list as the file has them, whatever is chosen above, and one line says
  // what came of it
  const fileSet = [{ name: 'Кисть', icon: 'snip', action: 'hotkey', keys: 'b', only_in: 'absent.exe' },
    { name: 'Нет', action: 'open', target: 'D:/gone.exe' }, { name: 'Снимок', action: 'hotkey', keys: 'win+shift+s' }];
  window.__keySet = () => ({ name: 'От друга', items: fileSet });
  window.__keySetItems = a => ({ added: [a.set[0]], had: 1, missing: 1 });
  const tool = n => document.querySelectorAll('#set-tools .btn')[n].click();
  const cards = () => [...document.querySelectorAll('#body .bundle')];
  kind(4); await sleep(20);
  out.setTools = texts('#set-tools .btn');
  out.setUnder = !!(cards().pop().compareDocumentPosition($('set-tools')) & Node.DOCUMENT_POSITION_FOLLOWING);
  seg(1); chip('Paint Studio');
  tool(0); await sleep(20);
  out.setSave = [last('save_key_set'), $('toast-text').textContent];
  tool(1); await sleep(20);
  const fileCard = cards().pop();
  out.setCard = [cards().length, fileCard.querySelector('b').textContent, fileCard.querySelector('small').textContent, fileCard.querySelectorAll('.key').length, fileCard.querySelector('.btn').textContent];
  const setSaves = calls('save_items').length;
  fileCard.querySelector('.btn').click(); await sleep(20);
  out.setCall = last('key_set_items');
  out.setSaved = last('save_items').items;
  out.setToast = $('toast-text').textContent;
  out.setClosed = !$('shade').classList.contains('on');
  await again();
  // A file that is not a key set: one line, no card, nothing saved
  window.__keySet = () => { throw 'not a key set'; };
  kind(4); await sleep(20); tool(1); await sleep(20);
  out.setBad = [$('toast-text').textContent, cards().length, calls('save_items').length - setSaves];
  closeDialog();
  // A site, file or folder: a bare address is a site
  kind(2);
  const inputs = () => [...document.querySelectorAll('#body input[type=text]')];
  inputs()[0].value = 'example.com'; inputs()[1].value = 'Сайт'; await save();
  out.site = last('save_items').items[4];
  await again();
  // A shortcut: put together with the buttons, or pressed on a keyboard
  kind(3);
  const mods = () => [...document.querySelectorAll('#body .field .btn')];
  mods()[0].click(); mods()[3].click();
  const key = document.querySelector('#body select'); key.value = 'k'; key.onchange();
  out.keysShown = texts('#body .does .kc');
  await save();
  out.keysByButtons = last('save_items').items[4];
  await again();
  kind(3);
  window.dispatchEvent(new KeyboardEvent('keydown', { code: 'F5', ctrlKey: true, shiftKey: true, bubbles: true }));
  await save();
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
  await save();
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
  await save();
  out.untouched = last('save_items').items[4];
  await again();
  // A program no longer running is still offered, so it is not lost
  rows()[5].click(); await sleep(20);
  out.keptPrograms = texts('#body .prog'); out.keptOn = texts('#body .prog.on');
  closeDialog();
  // Everywhere: the field goes; one program: the field comes
  rows()[4].click(); await sleep(20);
  seg(0); await save();
  out.everywhere = last('save_items').items[4];
  await again();
  rows()[2].click(); await sleep(20);
  seg(1); const before = calls('save_items').length; await save(); out.needProgram = [calls('save_items').length - before, $('toast-text').textContent];
  chip('Sketch'); await save();
  out.chosen = last('save_items').items[2];
  await again();
  // Every kind of item in words, with no path, address or file name
  window.__settings = { ...window.__settings, items: window.__settings.kinds }; await again();
  out.kindLines = rows().map(r => r.querySelector('.lbl small').textContent);
  out.kindKeys = rows().map(r => { const k = r.querySelector('.key'); return !!k.querySelector('svg') && !k.textContent.trim(); });
  // English: Rust says the language, the page changes every text
  window.__settings = { ...window.__settings, items: window.__settings.kinds.slice(0, 0).concat(window.__first), russian: false, theme: 'light' }; window.__on.reload(); await sleep(50);
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

// The card of programs from the quiet line under the items: the programs of this machine, as a dialog
const finding = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const chips = () => [...document.querySelectorAll('#body .chip')];
  const buttons = () => [...document.querySelectorAll('#foot button')];
  const press = text => buttons().find(b => b.textContent === text)?.click();
  const open = () => $('shade').classList.contains('on');
  const out = {};
  await sleep(100);
  // Nothing opens by itself, and the machine is not asked for its programs
  out.quiet = [open(), calls('first_programs').length];
  $('find').click(); await sleep(50);
  out.programs = [$('title').textContent, $('body').querySelector('.sub').textContent, chips().map(c => [c.textContent, c.classList.contains('on'), !!c.querySelector('.key img, .key svg')])];
  chips()[1].click(); out.unticked = chips().map(c => c.classList.contains('on'));
  chips()[1].click(); chips()[1].click();
  // The file cannot be written: the card stays as it was, and the window says why
  window.__itemsFail = true; press('Добавить'); await sleep(20);
  out.failed = [open(), $('toast-text').textContent, buttons().find(b => b.textContent === 'Добавить').disabled];
  window.__itemsFail = false;
  // Meanwhile Render was added to the list, written another way: it is not added twice
  window.__settings = { ...window.__settings, items: [...window.__settings.items, { raw: { name: 'Render', action: 'open', target: 'd:/render/RENDER.exe' }, icon: null, glyph: null, problem: null }] };
  window.__on.reload(); await sleep(20);
  press('Добавить'); await sleep(20);
  const saved = calls('save_items').pop();
  out.added = [saved.items.slice(-3), saved.items.length, saved.added, open()];
  // While a save runs, a chip does not let Add go again, and a second press writes nothing
  let release;
  window.__itemsHold = new Promise(r => { release = r; });
  $('find').click(); await sleep(50);
  const before = calls('save_items').length;
  press('Добавить'); await sleep(20);
  chips()[0].click(); chips()[0].click();
  const addButton = buttons().find(b => b.textContent === 'Добавить');
  out.inFlight = [addButton.disabled];
  addButton.disabled = false; addButton.click(); await sleep(20);
  out.inFlight.push(calls('save_items').length - before);
  window.__itemsHold = null; release(); await sleep(20);
  out.inFlight.push(open());
  // Not now adds nothing
  const saves = calls('save_items').length;
  $('find').click(); await sleep(50); press('Не сейчас');
  out.notNow = [open(), calls('save_items').length - saves];
  // Nothing found: a line that says so, and only a way out
  window.__found = []; $('find').click(); await sleep(50);
  out.nothing = [$('body').querySelector('.sub').textContent, chips().length, buttons().map(b => b.textContent)];
  press('Закрыть');
  out.nothingClosed = [open(), calls('close_welcome').length];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const PROGRAMS = [{ name: 'Paint Studio', target: 'C:\\Tools\\paint.exe' }, { name: 'Notes', target: 'shell:AppsFolder\\Vendor.Notes!App' }, { name: 'Render', target: 'D:\\Render\\render.exe' }];
const fp = found(renderedDom([], `window.__found = ${JSON.stringify(PROGRAMS)};` + finding, 5000, 'settings.html', settings));

test('story 43: the quiet line under the items finds my programs and adds the ticked ones', () => {
  assert.deepEqual(fp.quiet, [false, 0]);
  assert.deepEqual(fp.programs, ['Найти мои программы', 'Отмеченные станут клавишами капсулы. Лишнюю снимите касанием.',
    [['Paint Studio', true, true], ['Notes', true, true], ['Render', true, true]]]);
  assert.deepEqual(fp.unticked, [true, false, true]);
  assert.deepEqual(fp.failed, [true, 'the file is locked', false]);
  // The ticked ones go to the end of the list as programs, the unticked one and the one there does not
  assert.deepEqual(fp.added, [[settings.items[settings.items.length - 1].raw, { name: 'Render', action: 'open', target: 'd:/render/RENDER.exe' }, { name: 'Paint Studio', action: 'open', target: 'C:\\Tools\\paint.exe' }],
    settings.items.length + 2, 1, false]);
  assert.deepEqual(fp.inFlight, [true, 1, false]);
  assert.deepEqual(fp.notNow, [false, 0]);
  assert.deepEqual(fp.nothing, ['Своих программ не нашлось. Добавить их можно в настройках кнопкой «Добавить».', 0, ['Закрыть']]);
  // The settings window never closes the welcome window
  assert.deepEqual(fp.nothingClosed, [false, 0]);
});

// The welcome window of the first run: the same page opened as settings.html#welcome
const welcoming = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const press = text => [...$('welcome').querySelectorAll('button')].find(b => b.textContent === text)?.click();
  await sleep(100);
  const out = {};
  // A right click and the reload and print keys do nothing of the browser's
  const menu = () => { const e = new MouseEvent('contextmenu', { bubbles: true, cancelable: true }); $('welcome').dispatchEvent(e); return e.defaultPrevented; };
  const key = (k, ctrlKey) => { const e = new KeyboardEvent('keydown', { key: k, ctrlKey, bubbles: true, cancelable: true }); document.body.dispatchEvent(e); return e.defaultPrevented; };
  out.browser = [menu(), key('F5', false), key('r', true), key('p', true), key('a', true)];
  out.view = [getComputedStyle($('app')).display, $('welcome').hidden, $('welcome').querySelector('h1').textContent, $('welcome').querySelector('.sub').textContent,
    [...$('welcome').querySelectorAll('.chip')].map(c => c.textContent), [...$('welcome').querySelectorAll('.acts button')].map(b => b.textContent)];
  // A grey key's card is no business of this window
  out.card = $('shade').classList.contains('on');
  // Not now, or Close when nothing was found
  for (const text of ['Не сейчас', 'Close']) press(text);
  await sleep(20);
  out.closed = [calls('close_welcome').length, calls('save_items').length];
  press('Добавить'); await sleep(50);
  out.added = [calls('save_items').map(c => [c.items.slice(-2).map(i => i.name), c.added]), calls('close_welcome').length];
  // The window is asked to be as tall as its card, programs whole
  const box = $('welcome');
  // ...once, with the programs found: never while it lists them, which gave a strip of a title
  out.fits = calls('fit_welcome').map(c => c.height);
  out.fit = out.fits[out.fits.length - 1];
  // In a window of that height nothing scrolls, the buttons end with it, and nothing is left between
  document.documentElement.style.height = out.fit + 'px'; await sleep(20);
  const chips = box.querySelector('.chips'), acts = box.querySelector('.acts');
  out.whole = [chips.scrollHeight <= chips.clientHeight, Math.abs(acts.getBoundingClientRect().bottom + 16 - out.fit) < 2,
    Math.abs(acts.getBoundingClientRect().top - 8 - chips.getBoundingClientRect().bottom) < 3];
  // In a shorter one the programs scroll and the buttons still show
  document.documentElement.style.height = '150px'; await sleep(20);
  out.short = [acts.getBoundingClientRect().bottom <= 150, chips.scrollHeight > chips.clientHeight || !chips.children.length];
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
// The window asks for the programs as soon as the settings come, so they are there before it starts
const wl = found(renderedDom([], welcoming, 4000, 'settings.html', { ...settings, card: 2 }, { hash: '#welcome', before: `window.__found = ${JSON.stringify([...PROGRAMS, { name: 'Terminal', target: 'C:\Tools\term.exe' }])};` }));
const wlNone = found(renderedDom([], welcoming, 4000, 'settings.html', { ...settings, russian: false }, { hash: '#welcome', before: 'window.__found = [];' }));

test('story 45: the welcome window is one screen: my programs, Add and Not now', () => {
  assert.deepEqual(wl.view, ['none', false, 'Найти мои программы', 'Отмеченные станут клавишами капсулы. Лишнюю снимите касанием.', ['Paint Studio', 'Notes', 'Render', 'Terminal'], ['Не сейчас', 'Добавить']]);
  assert.equal(wl.card, false);
  // No browser menu on a right click, no reload or print; other keys are left alone
  assert.deepEqual(wl.browser, [true, true, true, true, false]);
  // Not now closes it and adds nothing; Add writes both, says how many were added, and closes it
  assert.deepEqual(wl.closed, [1, 0]);
  assert.deepEqual(wl.added, [[[['Render', 'Terminal'], 4]], 2]);
  // Nothing found: a short line and only a close button, in the language of the settings
  assert.deepEqual(wlNone.view, ['none', false, 'Find my programs', 'No programs of yours were found. Add them in the settings with Add.', [], ['Close']]);
  assert.deepEqual(wlNone.closed, [1, 0]);
  // The window fits its card: as tall as four ticked programs in two rows need, outlines and all,
  // with nothing to scroll; shorter with none; scrolling when cut
  assert.ok(wl.fit > wlNone.fit && wl.fit < 360, `${wl.fit} ${wlNone.fit}`);
  assert.deepEqual([wl.fits.length, wlNone.fits.length], [1, 1]);
  assert.deepEqual(wl.whole, [true, true, true]);
  assert.deepEqual(wlNone.whole, [true, true, true]);
  assert.deepEqual(wl.short, [true, true]);
});

test('the settings window lists the items of the file in words, broken ones included', () => {
  assert.deepEqual(set.rows, ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.lines, ['Выделить область', 'Сочетание Ctrl+Space', 'Программа', 'Пункт не работает: коснитесь, чтобы исправить']);
  assert.deepEqual(set.keys, [[true, ''], [true, ''], [true, ''], [true, '']]);
  // The card of programs opens only from its quiet line
  assert.deepEqual(set.find, ['Найти мои программы', false, false]);
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

test('story 37: keys are saved to a file, and a set from a file is a card that adds its keys with one line of what came of it', () => {
  assert.deepEqual(set.setTools, ['Сохранить мои клавиши…', 'Набор из файла…']);
  assert.equal(set.setUnder, true);
  assert.deepEqual(set.setSave, [{ name: 'Мои клавиши', onlyIn: null }, 'Сохранено клавиш: 3']);
  assert.deepEqual(set.setCard, [3, 'От друга', 'Клавиш: 3', 3, 'Добавить']);
  assert.equal(set.setCall.set.length, 3);
  assert.deepEqual(set.setCall.items.map(i => i.name), ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.setSaved.map(i => i.name), ['Снимок', 'Диктовка', 'Заметки', 'Потом', 'Кисть']);
  assert.equal(set.setSaved[4].only_in, 'absent.exe');
  assert.equal(set.setToast, 'Добавлено 1. Уже были: 1. Нет программы: 1');
  assert.equal(set.setClosed, true);
  assert.deepEqual(set.setBad, ['Это не набор клавиш Tapka', 2, 1]);
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

// The meters in the settings window: added from the program list or a file, removed and
// restored; what the window does not show of a meter stays in it
const withMeters = { ...settings, meters: [{ raw: { name: 'Render', program: 'C:\\Tools\\render.exe', own: 1 }, icon: PNG, problem: null },
  { raw: { name: 'Backup', file: 'D:\\data\\backup.txt' }, icon: null, problem: null }, { raw: { name: 'Both', program: 'a.exe', file: 'b.txt' }, icon: null, problem: 'both a program and a file' }] };
const meterSettings = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const saved = () => window.__calls.filter(c => c[0] === 'save_meters').map(c => c[1].meters).pop();
  const texts = sel => [...document.querySelectorAll(sel)].map(e => e.textContent);
  window.__pinned = [{ name: 'Paint Studio', target: 'C:\\\\Apps\\\\paint.exe', app: 'Vendor.Paint' }];
  window.__TAURI__.dialog = { open: async () => 'D:\\\\data\\\\Report.final.txt' };
  await sleep(50);
  const out = {};
  out.head = $('meters-head').textContent;
  out.rows = texts('#meters .row .lbl b');
  out.lines = texts('#meters .row .lbl small');
  out.icon = !!document.querySelector('#meters .row .key img');
  $('add-meter').click();
  out.kinds = texts('#body .row .lbl b');
  document.querySelectorAll('#body .row')[0].click(); await sleep(50);
  document.querySelector('#pinned-list .row').click(); await sleep(20);
  out.program = saved();
  window.__on.reload(); await sleep(20);
  $('add-meter').click();
  document.querySelectorAll('#body .row')[1].click(); await sleep(50);
  out.file = saved();
  window.__on.reload(); await sleep(20);
  document.querySelector('#meters .row .btn.danger').click();
  out.removed = saved();
  out.toast = $('toast').classList.contains('on');
  $('toast-undo').click();
  out.restored = saved();
  // The file cannot be written: the meter comes back, the page says why and reads the file again
  await sleep(20);
  window.__saveFails = true;
  const reads = () => window.__calls.filter(c => c[0] === 'get_settings').length, readsBefore = reads();
  document.querySelector('#meters .row .btn.danger').click();
  out.failingRows = texts('#meters .row .lbl b');
  await sleep(20);
  out.failedRows = texts('#meters .row .lbl b');
  out.failedToast = $('toast-text').textContent;
  out.readAgain = reads() - readsBefore;
  window.__saveFails = false;
  window.__settings = { ...window.__settings, meters: [] }; window.__on.reload(); await sleep(20);
  out.empty = texts('#meters .say');
  window.__settings = { ...window.__settings, russian: false }; window.__on.reload(); await sleep(20);
  out.en = [$('meters-head').textContent, $('meters-foot').textContent];
  out.privacy = texts('#privacy .lbl b').some(t => t.includes('meters'));
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const ms = found(renderedDom([], meterSettings, 4000, 'settings.html', withMeters));

test('story 39: a meter is added from the program list or a file, removed and restored, keeping the rest', () => {
  assert.equal(ms.head, 'ШкалыДобавить шкалу');
  assert.deepEqual(ms.rows, ['Render', 'Backup', 'Both']);
  assert.deepEqual(ms.lines, ['Прогресс программы', 'Число из файла backup.txt', 'Шкала не работает: удалите её или исправьте файл настроек']);
  assert.equal(ms.icon, true);
  assert.deepEqual(ms.kinds, ['Программа', 'Файл']);
  const before = withMeters.meters.map(m => m.raw);
  assert.deepEqual(ms.program, [...before, { name: 'Paint Studio', program: 'C:\\Apps\\paint.exe', app: 'Vendor.Paint' }]);
  assert.deepEqual(ms.file, [...before, { name: 'Report.final', file: 'D:\\data\\Report.final.txt' }]);
  assert.deepEqual(ms.removed, before.slice(1));
  assert.equal(ms.toast, true);
  assert.deepEqual(ms.restored, before);
  assert.deepEqual(ms.failingRows, ['Backup', 'Both']);
  assert.deepEqual(ms.failedRows, ['Render', 'Backup', 'Both']);
  assert.equal(ms.failedToast, 'the file is locked');
  assert.equal(ms.readAgain, 1);
  assert.deepEqual(ms.empty, ['Шкал нет. Шкала показывает прогресс программы или число из файла']);
  assert.deepEqual(ms.en, ['MetersAdd a meter', 'Meters sit in the capsule under the keys. Drag the line between the keys and the meters with a finger.']);
  assert.equal(ms.privacy, true);
});

// Docking in the settings window: the docked panels by program, one that goes back by itself
// and one Windows will not let be moved said so; the panels at the edge offered with a tick, those
// docked already left out, by path and class or, for an old entry, by file name; removed and
// restored; a save that fails puts the list back
const notch = { exe: 'notch.exe', path: 'C:\\Apps\\notch.exe', class: 'NotchWindow' };
const withDocked = { ...settings, docked: [
  { raw: { ...notch, own: 1 }, name: 'Notch', icon: PNG, state: 'placed' },
  { raw: 'side.exe', name: 'Side Bar', icon: null, state: 'returns' },
  { raw: { exe: 'lock.exe', path: 'C:\\Apps\\lock.exe', class: 'L' }, name: 'Lock', icon: null, state: 'refused' },
  { raw: 7, name: '—', icon: null, state: 'broken' },
] };
const dockSettings = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const saved = () => window.__calls.filter(c => c[0] === 'save_docked').map(c => c[1].docked).pop();
  const texts = sel => [...document.querySelectorAll(sel)].map(e => e.textContent);
  window.__candidates = [
    { raw: ${JSON.stringify(notch)}, name: 'Notch', icon: ${JSON.stringify(PNG)}, state: 'placed' },
    { raw: { exe: 'side.exe', path: 'C:\\\\Apps\\\\side.exe', class: 'Bar' }, name: 'Side Bar', icon: null, state: 'placed' },
    { raw: { exe: 'notch.exe', path: 'C:\\\\Apps\\\\notch.exe', class: 'NotchPopup' }, name: 'Notch', icon: null, state: 'placed' },
    { raw: { exe: 'dial.exe', path: 'C:\\\\Apps\\\\dial.exe', class: 'D' }, name: 'Dial', icon: null, state: 'placed', window: '21:5:1' },
    { raw: { exe: 'dial.exe', path: 'C:\\\\Apps\\\\dial.exe', class: 'D' }, name: 'Dial', icon: null, state: 'placed', window: '22:5:1' },
    { raw: { exe: 'clock.exe', path: 'C:\\\\Apps\\\\clock.exe', class: 'C' }, name: 'Clock', icon: ${JSON.stringify(PNG)}, state: 'placed' }];
  await sleep(50);
  const out = {};
  out.head = $('docked-head').textContent;
  out.rows = texts('#docked .row .lbl b');
  out.lines = texts('#docked .row .lbl small');
  out.problem = [...document.querySelectorAll('#docked .row .lbl small')].map(e => e.classList.contains('problem'));
  out.icon = !!document.querySelector('#docked .row .key img');
  out.foot = $('docked-foot').textContent;
  $('add-docked').click(); await sleep(20);
  out.title = $('title').textContent;
  out.offered = texts('#dock-list .row .lbl b');
  out.offeredSub = texts('#dock-list .row .lbl small');
  // Nothing ticked: nothing docked, and the page says to tick
  $('foot').querySelector('.btn.acc').click();
  out.untickedSaved = saved() === undefined;
  out.untickedToast = $('toast-text').textContent;
  const rows = document.querySelectorAll('#dock-list .row');
  rows[2].click(); rows[1].click(); rows[2].click();
  out.ticks = [...rows].map(r => r.classList.contains('on'));
  $('foot').querySelector('.btn.acc').click();
  out.docked = saved();
  out.chosen = window.__calls.filter(c => c[0] === 'save_docked').map(c => c[1].chosen).pop();
  out.closed = !$('shade').classList.contains('on');
  out.afterDock = texts('#docked .row .lbl b');
  window.__settings = withDockedThen; window.__on.reload(); await sleep(20);
  document.querySelectorAll('#docked .row .btn.danger')[0].click();
  out.removed = saved();
  out.removedRows = texts('#docked .row .lbl b');
  out.toast = $('toast-text').textContent;
  $('toast-undo').click();
  out.restored = saved();
  out.restoredRows = texts('#docked .row .lbl b');
  // The file cannot be written: the row comes back and the page says why
  window.__saveFails = true;
  document.querySelectorAll('#docked .row .btn.danger')[1].click();
  out.failingRows = texts('#docked .row .lbl b');
  await sleep(20);
  out.failedRows = texts('#docked .row .lbl b');
  out.failedToast = $('toast-text').textContent;
  window.__saveFails = false;
  // No panel at the edge
  window.__candidates = [{ raw: ${JSON.stringify(notch)}, name: 'Notch', icon: null, state: 'placed' }];
  $('add-docked').click(); await sleep(20);
  out.none = texts('#dock-list .say');
  $('close').click();
  window.__settings = { ...window.__settings, docked: [] }; window.__on.reload(); await sleep(20);
  out.empty = texts('#docked .say');
  window.__settings = { ...withDockedThen, russian: false }; window.__on.reload(); await sleep(20);
  out.en = [$('docked-head').textContent, $('docked-foot').textContent, ...texts('#docked .row .lbl small')];
  out.privacy = texts('#privacy .lbl b').some(t => t.includes('docked'));
  // A program with several windows the entry could be: the page asks for it to be picked again
  const several = { ...withDockedThen, docked: [{ raw: ${JSON.stringify(notch)}, name: 'Notch', icon: null, state: 'several' }] };
  window.__settings = several; window.__on.reload(); await sleep(20);
  out.several = [...texts('#docked .row .lbl small'), document.querySelector('#docked .row .lbl small').classList.contains('problem')];
  window.__settings = { ...several, russian: false }; window.__on.reload(); await sleep(20);
  out.severalEn = texts('#docked .row .lbl small');
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const dk = found(renderedDom([], `const withDockedThen = ${JSON.stringify(withDocked)};` + dockSettings, 4000, 'settings.html', withDocked));

test('story 41: a panel at the edge is ticked and docked, a docked one is removed and restored, in Russian and English', () => {
  const raws = withDocked.docked.map(d => d.raw);
  assert.equal(dk.head, 'СтыковкаПристыковать панель');
  assert.deepEqual(dk.rows, ['Notch', 'Side Bar', 'Lock', '—']);
  assert.deepEqual(dk.lines, ['Стоит под капсулой', 'Возвращается на своё место, Tapka её не двигает', 'Windows не даёт Tapka её двигать',
    'Запись не работает: удалите её или исправьте файл настроек']);
  assert.deepEqual(dk.problem, [false, true, true, true]);
  assert.equal(dk.icon, true);
  assert.equal(dk.foot, 'Tapka ставит выбранные панели под капсулой и ведёт их вместе с ней.');
  assert.equal(dk.title, 'Панели у края капсулы');
  // The docked window is not offered again, nor any window of the program an old entry names;
  // another window of a docked program is
  assert.deepEqual(dk.offered, ['Notch', 'Dial', 'Dial', 'Clock']);
  assert.deepEqual(dk.offeredSub, ['notch.exe', 'dial.exe', 'dial.exe', 'clock.exe']);
  assert.equal(dk.untickedSaved, true);
  assert.equal(dk.untickedToast, 'Отметьте панели, которые пристыковать');
  // Two windows of one program and class are one entry: ticking one unticks the other
  assert.deepEqual(dk.ticks, [false, false, true, false]);
  assert.deepEqual(dk.chosen, ['22:5:1']);
  // The entry Rust made for the window is saved as it is; what the window does not show stays
  assert.deepEqual(dk.docked, [...raws, { exe: 'dial.exe', path: 'C:\\Apps\\dial.exe', class: 'D' }]);
  assert.equal(dk.closed, true);
  assert.deepEqual(dk.afterDock, ['Notch', 'Side Bar', 'Lock', '—', 'Dial']);
  assert.deepEqual(dk.removed, raws.slice(1));
  assert.deepEqual(dk.removedRows, ['Side Bar', 'Lock', '—']);
  assert.equal(dk.toast, 'Панель отстыкована: Notch');
  assert.deepEqual(dk.restored, raws);
  assert.deepEqual(dk.restoredRows, ['Notch', 'Side Bar', 'Lock', '—']);
  assert.deepEqual(dk.failingRows, ['Notch', 'Lock', '—']);
  assert.deepEqual(dk.failedRows, ['Notch', 'Side Bar', 'Lock', '—']);
  assert.equal(dk.failedToast, 'the file is locked');
  assert.deepEqual(dk.none, ['У края капсулы сейчас нет панелей других программ поверх окон']);
  assert.deepEqual(dk.empty, ['Пристыкованных панелей нет. Панель другой программы у того же края можно поставить под капсулу']);
  assert.deepEqual(dk.en, ['DockingDock a panel', 'Tapka puts the chosen panels under the capsule and carries them along.',
    'Stands under the capsule', 'Returns to its own place, Tapka leaves it alone', 'Windows does not let Tapka move it',
    'This entry does not work: remove it or fix the settings file']);
  assert.equal(dk.privacy, true);
  assert.deepEqual(dk.several, ['Несколько окон — выберите заново', true]);
  assert.deepEqual(dk.severalEn, ['Several windows — pick again']);
});

// ---------- story 42: a program that is gone
const lostItems = [
  { at: 0, name: 'Снимок', hint: 'Выделить область экрана', icon: null },
  { at: 2, name: 'Paint', hint: 'Рисовать', icon: PNG, lost: true },
  { at: 3, name: 'Calc', icon: null, lost: true },
];
const lostScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const card = () => { const c = document.getElementById('card'); return c.hidden ? null : [c.querySelector('.c-title').textContent, c.querySelector('.c-sub').textContent]; };
  const fire = (el, type) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 1, pointerType: 'touch', clientX: 10, clientY: 10 }));
  const out = {};
  for (const [theme, words] of [['dark', 'Программа не найдена — коснитесь, чтобы выбрать'], ['light', 'Program not found — tap to choose']]) {
    window.__view = { ...window.__view, theme, lost: words }; window.__on.reload(); await sleep(50);
    const b = document.querySelectorAll('.item');
    const look = el => { const k = getComputedStyle(el.querySelector('.key')); return [k.opacity, k.filter]; };
    out[theme] = { lost: [...b].slice(0, 3).map(el => el.classList.contains('lost')), looks: [look(b[0]), look(b[1]), look(b[2])] };
    // The label of a grey key says why under its name; another key keeps its hint
    fire(b[1], 'pointerdown'); await sleep(450); out[theme].label = card(); fire(b[1], 'pointerup'); await sleep(20);
    fire(b[0], 'pointerdown'); await sleep(450); out[theme].other = card(); fire(b[0], 'pointerup'); await sleep(20);
  }
  // A tap on a grey key goes to Rust with its place, as any key's: Rust opens its card
  window.__calls = [];
  const b = document.querySelectorAll('.item');
  fire(b[2], 'pointerdown'); await sleep(60); fire(b[2], 'pointerup');
  out.tap = window.__calls.filter(c => c[0] === 'tap').map(c => c[1].index);
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const lostSeen = found(renderedDom(lostItems, lostScript, 4000));

test('story 42: a key whose program is gone is grey in both themes, says so in its label, and a tap goes to Rust', () => {
  for (const theme of ['dark', 'light']) {
    assert.deepEqual(lostSeen[theme].lost, [false, true, true], theme);
    // A key with a picture and a key with a letter alike: faded and without colour
    assert.deepEqual(lostSeen[theme].looks, [['1', 'none'], ['0.45', 'grayscale(1)'], ['0.45', 'grayscale(1)']], theme);
  }
  assert.deepEqual(lostSeen.dark.label, ['Paint', 'Программа не найдена — коснитесь, чтобы выбрать']);
  assert.deepEqual(lostSeen.light.label, ['Paint', 'Program not found — tap to choose']);
  assert.deepEqual(lostSeen.dark.other, ['Снимок', 'Выделить область экрана']);
  assert.deepEqual(lostSeen.tap, [3]);
});

const lostSettings = {
  ...settings,
  items: [
    settings.items[0],
    { raw: { name: 'Потом', action: 'script' }, icon: null, glyph: null, problem: 'unknown action "script"' },
    { raw: { name: 'Paint', action: 'open', target: 'C:\\Apps\\app-1.2.3\\Paint.exe', mine: 1 }, icon: null, glyph: null, problem: null, lost: true, found: 'D:\\Other\\Paint.exe' },
  ],
  docked: [{ raw: { exe: 'dial.exe', path: 'C:\\Apps\\dial.exe', class: 'D' }, name: 'Dial', icon: null, state: 'missing' }],
  repaired: [{ name: 'Sketch', path: 'C:\\Apps\\Sketch\\1.3.0\\sketch.exe' }],
  card: 2,
};
const lostSettingsScript = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const texts = sel => [...document.querySelectorAll(sel)].map(e => e.textContent);
  await sleep(50);
  const out = {};
  // Opened from a grey key: the window shows that item's card, with its path open to choose
  out.card = [$('shade').classList.contains('on'), $('title').textContent, document.querySelector('#body .namebox input').value,
    document.querySelector('#body .does small').textContent, document.querySelector('#body details').open];
  $('close').click();
  out.rowLine = [texts('#items .row .lbl small')[2], document.querySelectorAll('#items .row .lbl small')[2].classList.contains('problem')];
  out.repaired = [$('repaired').hidden, texts('#repaired .lbl b'), texts('#repaired .lbl small')];
  out.dock = [texts('#docked .row .lbl small'), document.querySelector('#docked .row .lbl small').classList.contains('problem')];
  // Read again, Rust no longer asks for the card nor has the line: neither comes back
  window.__settings = { ...window.__settings, card: null, repaired: [] }; window.__on.reload(); await sleep(20);
  out.again = [$('shade').classList.contains('on'), $('repaired').hidden];
  window.__settings = { ...window.__settings, russian: false, repaired: [{ name: 'Sketch', path: 'D:\\\\sketch.exe' }] }; window.__on.reload(); await sleep(20);
  out.en = [texts('#repaired .lbl b')[0], texts('#items .row .lbl small')[2], texts('#docked .row .lbl small')[0]];
  const rows = () => document.querySelectorAll('#items .row');
  const name = () => document.querySelector('#body .namebox input');
  const last = c => window.__calls.filter(x => x[0] === c).map(x => x[1]).pop();
  // Found in another folder: offered on the card, written only by a tap on Use it
  rows()[2].click(); await sleep(20);
  const offer = document.querySelector('#body .inline .sub');
  out.offer = [offer && offer.textContent, !!last('use_path')];
  offer.parentElement.querySelector('.btn.acc').click(); await sleep(20);
  out.used = [last('use_path'), $('shade').classList.contains('on')];
  // A card with an edit not saved yet stays when a grey key asks for another card
  rows()[2].click(); await sleep(20);
  name().value = 'Paint 2';
  window.__settings = { ...window.__settings, card: 0 }; window.__on.reload(); await sleep(20);
  out.busy = [$('shade').classList.contains('on'), name().value, $('toast-text').textContent];
  // Meanwhile the panel put the path right: the save changes the name on the item as it is now
  window.__settings = { ...window.__settings, card: null, items: window.__settings.items.map((it, n) => n === 2 ? { ...it, raw: { ...it.raw, target: 'C:\\\\Apps\\\\app-1.3\\\\Paint.exe' }, lost: false, found: null } : it) };
  window.__on.reload(); await sleep(20);
  document.querySelector('#foot .acc').click(); await sleep(30);
  out.saved = last('save_items').items[2];
  // Without an edit, the card asked for opens in place of the one on screen
  rows()[2].click(); await sleep(20);
  window.__settings = { ...window.__settings, card: 0 }; window.__on.reload(); await sleep(20);
  out.replaced = name().value;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const ls = found(renderedDom([], lostSettingsScript, 4000, 'settings.html', lostSettings));

test('story 42: a tap on a grey key opens its card; the window says which paths were updated and which programs are gone', () => {
  assert.deepEqual(ls.card, [true, 'Изменить', 'Paint', 'Программа не найдена', true]);
  assert.deepEqual(ls.rowLine, ['Программа не найдена', true]);
  assert.deepEqual(ls.repaired, [false, ['Путь обновлён: Sketch'], ['C:\\Apps\\Sketch\\1.3.0\\sketch.exe']]);
  assert.deepEqual(ls.dock, [['Программа не найдена'], true]);
  assert.deepEqual(ls.again, [false, true]);
  assert.deepEqual(ls.en, ['Path updated: Sketch', 'Program not found', 'Program not found']);
});

test('story 42: a program found in another folder is offered on its card; an open card keeps its edits and saves onto the fresh item', () => {
  assert.deepEqual(ls.offer, ['Found in D:\\Other\\Paint.exe', false]);
  assert.deepEqual(ls.used, [{ target: 'C:\\Apps\\app-1.2.3\\Paint.exe' }, false]);
  assert.deepEqual(ls.busy, [true, 'Paint 2', 'Save or close the open card first']);
  assert.deepEqual(ls.saved, { name: 'Paint 2', action: 'open', target: 'C:\\Apps\\app-1.3\\Paint.exe', mine: 1 });
  assert.equal(ls.replaced, 'Снимок');
});
