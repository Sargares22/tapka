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
// stand-in for Tauri whose `get_view` answers with window.__view and `get_settings` with
// window.__settings; every call is kept in window.__calls and the page's event listeners in
// window.__on, by event name.
// `script` runs after the page's own script and may leave its findings in the DOM.
function renderedDom(items, script = '', ms = 4000, page = 'index.html', settings = null) {
  assert.ok(browser, 'Google Chrome not found');
  const work = mkdtempSync(join(tmpdir(), 'tapka-check-'));
  try {
    const stub = `<script>window.__calls=[];window.__on={};window.__view={items:${JSON.stringify(items)},cell:54,tablet:false,edge:'right',theme:'dark',accent:'#8ab4ff',add:['Добавить','Открыть редактор пунктов']};window.__settings=${JSON.stringify(settings)};window.__TAURI__={core:{invoke:async(c,a)=>{window.__calls.push([c,a]);return c==='get_view'?window.__view:c==='get_settings'?JSON.parse(JSON.stringify(window.__settings)):null}},event:{listen(n,cb){window.__on[n]=cb}}}</script>`;
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
const items = [
  { name: 'Снимок', hint: 'Выделить область экрана', icon: null },
  { name: 'диктовка', icon: null },
  { name: 'Сайт', icon: PNG },
  { name: 'Calc', icon: null },
  { name: 'Вставить', icon: null, glyph: '<svg viewBox="0 0 24 24" stroke="currentColor"><path d="M4 4h16"/></svg>' },
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
  // The last key is not an item: a tap on it asks for the editor by the index after the items
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
  assert.equal(seen.plusTap, items.length);
  assert.equal(seen.plusLabel, 'Добавить');
});

test('the capsule takes its theme and accent from the settings', () => {
  assert.equal(seen.themeBefore, 'dark');
  assert.equal(seen.themeAfter, 'light');
  assert.equal(seen.accentAfter, '#ff8800');
  assert.equal(seen.pillLight, 'rgb(238, 240, 244)');
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
  // What a hand may leave in the file: not an object, a name that is not text
  odd: [
    { raw: null, icon: null, glyph: null, problem: 'no name' },
    { raw: 'text', icon: null, glyph: null, problem: 'no name' },
    { raw: { name: 5, action: 'hotkey', keys: 7, hint: { toString: 0 } }, icon: null, glyph: null, problem: 'no name' },
    { raw: { name: 'Крив', action: 'open', target: 'x', hint: { toString: 0 } }, icon: null, glyph: null, problem: null },
    { raw: { name: 'Цел', action: 'open', target: 'x' }, icon: null, glyph: null, problem: null },
  ],
  scale: 1, edge: 'right', theme: 'dark', accent: '#8ab4ff', lang: 'system', russian: true, autostart: false, updates: true,
  version: '1.0.0', error: null, intro: true,
  presets: [{ id: 'snip', item: { name: 'Снимок', icon: 'snip', hint: 'Выделить область экрана', action: 'hotkey', keys: 'win+shift+s' } },
    { id: 'settings', item: { name: 'Параметры', icon: 'gear', hint: 'Открыть параметры Windows', action: 'open', target: 'ms-settings:' } }],
  glyphs: { snip: glyph, mic: glyph, gear: glyph, keyboard: glyph, browser: glyph },
};
const editing = `(async () => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const $ = id => document.getElementById(id);
  const calls = name => window.__calls.filter(c => c[0] === name).map(c => c[1]);
  const last = name => calls(name).pop();
  // The stand-in for Rust keeps the file as it was: each step starts from that list again
  const again = async () => { window.__on.reload(); await sleep(20); };
  await sleep(50);
  const out = {};
  const rows = () => [...document.querySelectorAll('#items .row')];
  out.rows = rows().map(r => r.querySelector('.label span').textContent);
  out.lines = rows().map(r => r.querySelector('.label small').textContent);
  out.intro = !$('intro').hidden;
  out.navRu = [...document.querySelectorAll('.nav span')].map(s => s.textContent);
  out.theme = document.documentElement.dataset.theme;
  out.accent = document.documentElement.style.getPropertyValue('--accent');
  // Look: a change is shown at once and goes to Rust as one setting
  $('theme').value = 'light'; $('theme').onchange();
  out.themeAfter = document.documentElement.dataset.theme; out.themeCall = last('set_pref');
  document.querySelectorAll('.swatch')[3].click();
  out.accentAfter = document.documentElement.style.getPropertyValue('--accent'); out.accentCall = last('set_pref');
  $('scale').children[2].click(); out.scaleCall = last('set_pref');
  $('edge').children[0].click(); out.edgeCall = last('set_pref');
  $('autostart').click(); out.autostartCall = last('set_pref');
  // The add dialog offers four kinds; a Windows action is added by one tap
  $('add').click();
  out.kinds = [...document.querySelectorAll('#body .pick b')].map(b => b.textContent);
  document.querySelectorAll('#body .pick')[3].click();
  out.presets = [...document.querySelectorAll('#body .pick b')].map(b => b.textContent);
  document.querySelectorAll('#body .pick')[1].click();
  out.added = last('save_items').items.map(i => i.name); out.addedItem = last('save_items').items[4];
  out.closedAfterAdd = !$('shade').classList.contains('on');
  await again();
  // A site, file or folder: an address and a name
  $('add').click(); document.querySelectorAll('#body .pick')[1].click();
  const inputs = () => [...document.querySelectorAll('#body input[type=text]')];
  inputs()[0].value = 'https://example.com'; inputs()[1].value = 'Сайт'; document.querySelector('#foot .accent').click();
  out.site = last('save_items').items[4];
  await again();
  // A shortcut: put together with the buttons, or pressed on a keyboard
  $('add').click(); document.querySelectorAll('#body .pick')[2].click();
  const chips = () => [...document.querySelectorAll('#body .chip')];
  chips()[0].click(); chips()[3].click();
  const key = document.querySelector('#body select'); key.value = 'k'; key.onchange();
  document.querySelector('#foot .accent').click();
  out.keysByButtons = last('save_items').items[4];
  await again();
  $('add').click(); document.querySelectorAll('#body .pick')[2].click();
  window.dispatchEvent(new KeyboardEvent('keydown', { code: 'F5', ctrlKey: true, shiftKey: true, bubbles: true }));
  document.querySelector('#foot .accent').click();
  out.keysPressed = last('save_items').items[4].keys;
  await again();
  // Editing keeps what the window does not show: the lit rule stays in the item
  rows()[1].querySelectorAll('.icon-btn')[0].click();
  inputs()[0].value = 'Запись'; inputs()[1].value = 'Подсказка';
  document.querySelector('#foot .accent').click();
  out.edited = last('save_items').items[1];
  out.editedOthers = last('save_items').items.map(i => i.name);
  await again();
  // Removing takes the item out and offers to put it back
  rows()[0].querySelectorAll('.icon-btn')[1].click();
  out.removed = last('save_items').items.map(i => i.name);
  out.toast = $('toast').classList.contains('on');
  $('toast-undo').click(); out.restored = last('save_items').items.map(i => i.name);
  // Order: the first item carried below the second
  const grip = rows()[0].querySelector('.grip'), h = rows()[0].offsetHeight;
  grip.setPointerCapture = () => {};
  const at = (type, y) => grip.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, pointerType: 'touch', clientX: 20, clientY: y }));
  at('pointerdown', 100); at('pointermove', 100 + h); at('pointerup', 100 + h);
  out.moved = last('save_items').items.map(i => i.name);
  // English: Rust says the language, the page changes every text
  window.__settings = { ...window.__settings, russian: false, intro: false, theme: 'light' }; window.__on.reload(); await sleep(50);
  out.navEn = [...document.querySelectorAll('.nav span')].map(s => s.textContent);
  out.addEn = $('add').textContent; out.introGone = $('intro').hidden;
  // A file with odd items: the list is still drawn, and an odd item can be removed
  window.__settings = { ...window.__settings, items: window.__settings.odd }; window.__on.reload(); await sleep(50);
  out.oddRows = rows().map(r => r.querySelector('.label span').textContent);
  rows()[0].querySelectorAll('.icon-btn')[0].click(); out.oddEditOpens = $('shade').classList.contains('on');
  rows()[0].querySelectorAll('.icon-btn')[1].click(); out.oddRemoved = last('save_items').items.length;
  // Two removals in a hurry, before Rust has answered the first: the second starts from the first
  rows()[0].querySelectorAll('.icon-btn')[1].click(); out.oddRemovedTwice = last('save_items').items.length;
  document.documentElement.dataset.check = JSON.stringify(out);
})();`;
const set = found(renderedDom([], editing, 4000, 'settings.html', settings));

test('the settings window lists the items of the file, broken ones included', () => {
  assert.deepEqual(set.rows, ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.lines.slice(0, 3), ['Выделить область', 'Ctrl + Space', 'C:\\Tools\\notes.exe']);
  assert.match(set.lines[3], /unknown action/);
  assert.equal(set.intro, true);
});

test('theme, accent, size, edge and autostart apply at once and go to Rust one by one', () => {
  assert.deepEqual([set.theme, set.accent], ['dark', '#8ab4ff']);
  assert.equal(set.themeAfter, 'light');
  assert.deepEqual(set.themeCall, { key: 'theme', value: 'light' });
  assert.equal(set.accentAfter, '#ffd166');
  assert.deepEqual(set.accentCall, { key: 'accent', value: '#ffd166' });
  assert.deepEqual(set.scaleCall, { key: 'scale', value: 1.25 });
  assert.deepEqual(set.edgeCall, { key: 'edge', value: 'left' });
  assert.deepEqual(set.autostartCall, { key: 'autostart', value: true });
});

test('an item is added in four ways', () => {
  assert.equal(set.kinds.length, 4);
  assert.deepEqual(set.presets, ['Снимок', 'Параметры']);
  assert.deepEqual(set.added, ['Снимок', 'Диктовка', 'Заметки', 'Потом', 'Параметры']);
  assert.deepEqual(set.addedItem, settings.presets[1].item);
  assert.equal(set.closedAfterAdd, true);
  assert.deepEqual(set.site, { name: 'Сайт', action: 'open', target: 'https://example.com' });
  assert.deepEqual(set.keysByButtons, { name: 'Ctrl + Win + K', icon: 'keyboard', action: 'hotkey', keys: 'ctrl+win+k' });
  assert.equal(set.keysPressed, 'ctrl+shift+f5');
});

test('an item is edited, removed, restored and moved; fields the window does not show are kept', () => {
  assert.deepEqual(set.edited, { name: 'Запись', icon: 'mic', action: 'hotkey', keys: 'ctrl+space', lit: { program: 'recorder.exe', window: 'Recording' }, hint: 'Подсказка' });
  assert.deepEqual(set.editedOthers, ['Снимок', 'Запись', 'Заметки', 'Потом']);
  assert.deepEqual(set.removed, ['Диктовка', 'Заметки', 'Потом']);
  assert.equal(set.toast, true);
  assert.deepEqual(set.restored, ['Снимок', 'Диктовка', 'Заметки', 'Потом']);
  assert.deepEqual(set.moved, ['Диктовка', 'Снимок', 'Заметки', 'Потом']);
});

test('odd items in the file do not break the list', () => {
  assert.deepEqual(set.oddRows, ['—', '—', '—', 'Крив', 'Цел']);
  assert.equal(set.oddEditOpens, false);
  assert.equal(set.oddRemoved, 4);
  assert.equal(set.oddRemovedTwice, 3);
});

test('the settings window speaks Russian and English', () => {
  assert.deepEqual(set.navRu, ['Пункты', 'Вид', 'Общие', 'О программе']);
  assert.deepEqual(set.navEn, ['Items', 'Look', 'General', 'About']);
  assert.equal(set.addEn, 'Add');
  assert.equal(set.introGone, true);
});
