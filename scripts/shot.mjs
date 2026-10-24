// Pictures of the capsule and the settings window in both themes, into target/shots. The pages
// are rendered in headless Chrome with sample data and a stand-in for Tauri, so this shows
// layout, texts, icons and themes, and nothing about real windows. Run: node scripts/shot.mjs
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const out = join(root, 'target', 'shots');
const chrome = ['C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe'].find(existsSync);
if (!chrome) throw new Error('Google Chrome not found');
mkdirSync(out, { recursive: true });

const glyphs = Object.fromEntries(readdirSync(join(root, 'glyphs')).filter(f => f.endsWith('.svg'))
  .map(f => [f.slice(0, -4), readFileSync(join(root, 'glyphs', f), 'utf8')]));
const sample = [
  { name: 'Snip', icon: 'snip', hint: 'Select a screen area', action: 'hotkey', keys: 'win+shift+s' },
  { name: 'Paste', icon: 'paste', hint: 'Paste the clipboard', action: 'hotkey', keys: 'ctrl+v' },
  { name: 'Voice typing', icon: 'mic', action: 'hotkey', keys: 'win+h' },
  { name: 'Notes', action: 'open', target: 'C:\\Tools\\notes.exe' },
  { name: 'File Explorer', icon: 'folder', action: 'hotkey', keys: 'win+e' },
  // Every built-in icon, so a new one can be looked at next to the others
  ...Object.keys(glyphs).map(icon => ({ name: icon, icon, action: 'hotkey', keys: 'f1' })),
];

function shoot(page, name, size, data, after = '', hash = '') {
  const work = mkdtempSync(join(tmpdir(), 'tapka-shot-'));
  const stub = `<script>window.__on={};window.__TAURI__={core:{invoke:async c=>(${JSON.stringify(data)})[c]??null},event:{listen(n,cb){window.__on[n]=cb}}}</script>`;
  const html = readFileSync(join(root, 'ui', page), 'utf8').replace('<head>', '<head>' + stub).replace('</body>', after + '</body>');
  writeFileSync(join(work, 'page.html'), html);
  execFileSync(chrome, ['--headless=new', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=2', `--window-size=${size}`,
    '--default-background-color=00000000', `--user-data-dir=${join(work, 'profile')}`, '--virtual-time-budget=1500',
    `--screenshot=${join(out, name + '.png')}`, pathToFileURL(join(work, 'page.html')).href + hash], { stdio: 'ignore' });
  rmSync(work, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  console.log(join('target', 'shots', name + '.png'));
}

for (const [theme, accent] of [['dark', '#8ab4ff'], ['light', '#ff8a3d']]) {
  const items = sample.slice(0, 6).map(i => ({ name: i.name, hint: i.hint, glyph: glyphs[i.icon] || null, icon: null, on: i.name === 'Notes', live: i.name === 'Voice typing' }));
  // The capsule's own window is 348 px wide: the strip and the room for the label
  const box = '<style>html,body{width:348px!important;height:340px!important}body{position:relative}</style>';
  const hover = "<script>setTimeout(()=>{const r=document.querySelector('.item').getBoundingClientRect();window.__on.hover({payload:[r.left+r.width/2,r.top+r.height/2]})},100)</script>";
  const view = { items, cell: 54, tablet: false, edge: 'right', theme, accent, add: ['Add', 'Settings and folding the capsule'], gear: { name: 'Settings', hint: 'Open the settings window', glyph: glyphs.gear }, fold: { name: 'Fold', hint: 'Fold the capsule into one key', glyph: glyphs.right }, unfold: glyphs.left };
  shoot('index.html', 'capsule-' + theme, '348,340', { get_view: view }, box + hover);
  // A tap on the plus: the gear, the arrow and the end grip after them, at the side and the top
  // edge, with the pill as long as layout.rs makes it for six keys with the signs
  const openFoot = "<script>setTimeout(()=>{const p=document.querySelector('#keys > .item.add'),r=p.getBoundingClientRect(),at=t=>new PointerEvent(t,{bubbles:true,pointerId:1,pointerType:'touch',clientX:r.left+r.width/2,clientY:r.top+r.height/2});p.dispatchEvent(at('pointerdown'));window.dispatchEvent(at('pointerup'))},100)</script>";
  const signed = { ...view, pill: [319, 401] };
  shoot('index.html', 'grip-' + theme, '348,426', { get_view: signed }, '<style>html,body{width:348px!important;height:426px!important}body{position:relative}</style>' + openFoot);
  shoot('index.html', 'grip-top-' + theme, '426,174', { get_view: { ...signed, edge: 'top' } }, '<style>html,body{width:426px!important;height:174px!important}</style>' + openFoot);
  // A key whose program is gone: grey, its label saying so
  const lostView = { ...view, items: items.map(it => ({ ...it, lost: it.name === 'Paste' })), lost: 'Program not found — tap to choose' };
  const hoverLost = hover.replace("querySelector('.item')", "querySelector('.item.lost')");
  shoot('index.html', 'lost-' + theme, '348,340', { get_view: lostView }, box + hoverLost);
  // With meters under the keys, at the three sizes and at the top edge: a program at 42 %, one
  // past 80 %, a file with nothing in it; the key of the first program has its ring. The pill is
  // as long as layout.rs makes it for six keys and three meters: four whole keys, every meter
  const metered = { ...view, items: items.map((it, i) => ({ ...it, ring: i === 3 ? 0 : null })), pill: [471, 553], room: [372, 372], divider: null,
    meters: [{ name: 'Notes', icon: null, target: 'D:/Tools/notes.exe', value: 42 }, { name: 'Render', icon: null, target: 'D:/Tools/render.exe', value: 86 }, { name: 'backup', icon: null, target: null, value: null }] };
  for (const scale of [0.8, 1, 1.25]) {
    const zoom = `<style>html,body{width:348px!important;height:513px!important}body{position:relative;zoom:${scale}}</style>`;
    shoot('index.html', `meters-${scale}-${theme}`, `${Math.round(348 * scale)},${Math.round(513 * scale)}`, { get_view: metered }, zoom);
  }
  // A file of two numbers and three lines of hint: the outer ring on its dial, and its label,
  // hovered, with the hint under the percent
  const ringed = { ...metered, meters: [{ name: 'backup', icon: null, target: null, value: 64, outer: 88, hint: ['Disk D: 64 %', 'Copied 1 240 of 1 930 files', 'Ends in about 12 min'] }, metered.meters[0], metered.meters[1]] };
  const hoverMeter = "<script>setTimeout(()=>{const r=document.querySelector('.meter').getBoundingClientRect();window.__on.hover({payload:[r.left+r.width/2,r.top+r.height/2]})},100)</script>";
  const tall = '<style>html,body{width:348px!important;height:513px!important}body{position:relative}</style>';
  shoot('index.html', 'meters-file-' + theme, '348,513', { get_view: ringed }, tall + hoverMeter);
  shoot('index.html', 'meters-top-' + theme, '513,174', { get_view: { ...metered, edge: 'top' } }, '<style>html,body{width:513px!important;height:174px!important}</style>');
  // The labels of meters, each hovered, in Russian, whose words are the longest: an AI limit with its
  // two rows and a problem; a file of two numbers with its own lines; a stale limit from Codex's log;
  // a limit never read; a program's one row; and an AI limit in English
  const hoverAt = i => `<script>setTimeout(()=>{const r=document.querySelectorAll('.meter')[${i}].getBoundingClientRect();window.__on.hover({payload:[r.left+r.width/2,r.top+r.height/2]})},100)</script>`;
  const labels = {
    limit: { name: 'Claude', limit: 'claude', value: 74, outer: 100, rows: ['5 часов', 'Неделя'], when: ['сброс через 23 ч 59 мин', 'сброс пн 2 ноя, 10:59'], problem: 'Нажмите, чтобы подключить' },
    file: { name: 'backup', value: 64, outer: 88, when: ['сброс через 2 ч 10 мин', 'сброс пн, 10:59'], hint: ['Диск D: 64 %', 'Скопировано 1 240 из 1 930 файлов'] },
    stale: { name: 'ChatGPT', limit: 'chatgpt', value: 30, outer: 20, stale: true, rows: ['5 часов', 'Неделя'], when: ['сбросился', 'сброс пт, 22:03'], ago: 'обновлено 2 ч 5 мин назад', hint: ['по последнему запуску Codex'], problem: 'Нажмите, чтобы подключить' },
    nodata: { name: 'Claude', limit: 'claude', value: null, problem: 'Нажмите, чтобы подключить' },
    program: { name: 'Render', value: 42 },
    'limit-en': { name: 'Antigravity · Claude and GPT', limit: 'antigravity-claude', value: 74, outer: 6, rows: ['5 hours', 'Week'], when: ['resets in 2 h 10 min', 'resets Mon, 10:59'], problem: 'Tap to connect' },
  };
  for (const [kind, meter] of Object.entries(labels)) {
    const shown = { ...metered, no_data: kind.endsWith('en') ? 'no data' : 'нет данных', used: kind.endsWith('en') ? 'used' : 'израсходовано', meters: [metered.meters[0], { icon: null, target: null, ...meter }, metered.meters[2]] };
    shoot('index.html', `label-${kind}-${theme}`, '348,513', { get_view: shown }, tall + hoverAt(1));
  }
  // Stretched to eight cells with three keys and two meters: the capsule ends with them, no empty
  // cells (handle 44, keys 162, line 14, meters 104, foot 41)
  const short = { ...view, items: items.slice(0, 3), length: 8, pill: [365, 447], room: [266, 266], divider: null, meters: metered.meters.slice(0, 2) };
  shoot('index.html', 'length-content-' + theme, '348,513', { get_view: short }, tall);
  shoot('index.html', 'length-content-signs-' + theme, '348,513', { get_view: short }, tall + "<script>setTimeout(()=>{const p=document.querySelector('#keys > .item.add'),r=p.getBoundingClientRect(),at=t=>new PointerEvent(t,{bubbles:true,pointerId:1,pointerType:'touch',clientX:r.left+r.width/2,clientY:r.top+r.height/2});p.dispatchEvent(at('pointerdown'));window.dispatchEvent(at('pointerup'))},100)</script>");
  // A key without a meter whose program shows progress on the taskbar
  const rimmed = { ...view, items: items.map((it, i) => ({ ...it, at: i, rim: i === 3 ? 64 : null })) };
  shoot('index.html', 'rim-' + theme, '348,340', { get_view: rimmed }, box);
  // Keys the card of programs just added: in view, and the label beside the list asks for a tap
  // Headless Chrome does not run a smooth scroll to its end: the scroll asked for is made at once
  const atOnce = "<script>{const l=document.getElementById('list'),by=l.scrollBy.bind(l);l.scrollBy=h=>{by({...h,behavior:'auto'});l.dispatchEvent(new Event('scrollend'))}}</script>";
  shoot('index.html', 'tip-' + theme, '348,340', { get_view: rimmed }, box + atOnce + "<script>setTimeout(()=>window.__on.added({payload:{at:[4,5],tip:'Tap a key'}}),100)</script>");
  const settings = {
    meters: [{ raw: { name: 'Notes', program: 'C:\\Tools\\notes.exe' }, icon: null, problem: null }, { raw: { name: 'backup', file: 'D:\\backup.txt' }, icon: null, problem: null },
      { raw: { name: 'Claude', limit: 'claude' }, icon: null, problem: null }, { raw: { name: 'ChatGPT', limit: 'chatgpt' }, icon: null, problem: null }],
    docked: [{ raw: 'sidebar.exe', name: 'Sidebar', icon: null, state: 'placed' }, { raw: 'dials.exe', name: 'Dials', icon: null, state: 'returns' },
      { raw: 'guard.exe', name: 'Guard', icon: null, state: 'refused' }],
    items: sample.map(raw => ({ raw, icon: null, glyph: glyphs[raw.icon] || null, problem: null })),
    scale: 1, edge: 'right', theme, accent, lang: 'en', russian: false, autostart: false, updates: true, tablet_only: false, version: '0.0.0', error: null,
    presets: sample.slice(0, 3).map(item => ({ id: item.icon, item })), pages: [], bundles: [], glyphs, update: { state: 'none' },
  };
  shoot('settings.html', 'settings-' + theme, '880,1100', { get_settings: settings });
  // The welcome window of the first run, in English and in Russian, and with nothing found; the card
  // of programs from the line under the items
  const first = [{ name: 'Paint Studio', target: 'C:\Tools\paint.exe' }, { name: 'Notes', target: 'C:\Tools\notes.exe' }, { name: 'Browser', target: 'C:\Tools\browser.exe' },
    { name: 'Mail', target: 'C:\Tools\mail.exe' }, { name: 'Sketchbook with a long name', target: 'C:\Tools\sketch.exe' }, { name: 'Terminal', target: 'C:\Tools\term.exe' },
    { name: 'Music', target: 'C:\Tools\music.exe' }, { name: 'Calendar', target: 'C:\Tools\calendar.exe' }];
  // Headless Chrome keeps a window wider than this one: the page is held to its size
  const small = '<style>html,body{width:420px!important;height:360px!important}</style>';
  shoot('settings.html', 'welcome-' + theme, '420,360', { get_settings: settings, first_programs: first }, small + "<script>setTimeout(()=>document.querySelectorAll('#welcome .chip')[2].click(),300)</script>", '#welcome');
  shoot('settings.html', 'welcome-ru-' + theme, '420,360', { get_settings: { ...settings, russian: true }, first_programs: first.slice(0, 3) }, small, '#welcome');
  // Four programs: the window as tall as its card says (fit_welcome), the rest of the picture empty
  const fitted = "<script>{const ask=window.__TAURI__.core.invoke;window.__TAURI__.core.invoke=(c,a)=>{if(c==='fit_welcome'){document.documentElement.style.setProperty('height',a.height+'px','important');document.body.style.setProperty('height',a.height+'px','important')}return ask(c,a)}}</script>";
  shoot('settings.html', 'welcome-four-' + theme, '420,360', { get_settings: settings, first_programs: first.slice(0, 4) }, small.replace('height:360px!important', 'height:auto') + fitted, '#welcome');
  shoot('settings.html', 'welcome-none-' + theme, '420,360', { get_settings: settings, first_programs: [] }, small, '#welcome');
  shoot('settings.html', 'find-line-' + theme, '880,640', { get_settings: { ...settings, russian: true } }, "<script>setTimeout(()=>document.getElementById('find-line').scrollIntoView({block:'center'}),100)</script>");
  shoot('settings.html', 'find-' + theme, '880,640', { get_settings: { ...settings, russian: true }, first_programs: first.slice(0, 5) }, "<script>setTimeout(()=>document.getElementById('find').click(),200)</script>");
  // A path put right, and a key and a docked panel whose program is gone
  const lostSettings = { ...settings, items: settings.items.map(i => ({ ...i, lost: i.raw.name === 'Notes' })), repaired: [{ name: 'Sketch', path: 'C:\\Apps\\Sketch\\app-1.3.0\\sketch.exe' }],
    docked: [...settings.docked, { raw: { exe: 'clock.exe', path: 'C:\\Apps\\clock.exe' }, name: 'Clock', icon: null, state: 'missing' }] };
  shoot('settings.html', 'lost-settings-' + theme, '880,640', { get_settings: lostSettings });
  shoot('settings.html', 'lost-docked-' + theme, '880,640', { get_settings: lostSettings }, "<script>setTimeout(()=>show('docked'),100)</script>");
  // The card of a grey key whose program was found in another folder: it offers that place
  const offered = { ...lostSettings, items: lostSettings.items.map(i => i.lost ? { ...i, found: 'D:\\Portable\\notes.exe' } : i), card: 3 };
  shoot('settings.html', 'lost-card-' + theme, '880,640', { get_settings: offered });
  // The three list pages in Russian. Claude is not connected, ChatGPT is; Antigravity's program is
  // missing for one group, and the other connects when added
  const ru = { ...settings, russian: true };
  const limitStatus = { claude: { state: 'sign_in' }, chatgpt: { state: 'ok' }, 'antigravity-gemini': { state: 'missing' }, 'antigravity-claude': { state: 'unknown' } };
  const page = s => `<script>setTimeout(()=>show('${s}'),300)</script>`;
  shoot('settings.html', 'page-keys-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, page('items'));
  shoot('settings.html', 'page-meters-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, page('meters'));
  shoot('settings.html', 'page-docked-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, page('docked'));
  // An indicator's card: Claude, not connected, with Connect; a nearby panel's card that keeps its place
  shoot('settings.html', 'meter-card-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, "<script>setTimeout(()=>{show('meters');document.querySelectorAll('#meters .row')[2].click()},300)</script>");
  shoot('settings.html', 'docked-card-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, "<script>setTimeout(()=>{show('docked');document.querySelectorAll('#docked .row')[1].click()},300)</script>");
  // Adding an indicator: the kinds, the AI services with where each stands, and Connect explained
  const kinds = "document.getElementById('add-meter').click()", services = kinds + ";document.querySelectorAll('#body .row')[1].click()";
  shoot('settings.html', 'meter-kinds-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, `<script>setTimeout(()=>{show('meters');${kinds}},300)</script>`);
  shoot('settings.html', 'meter-limit-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus }, `<script>setTimeout(()=>{show('meters');${services}},300)</script>`);
  shoot('settings.html', 'meter-limit-offer-' + theme, '880,640', { get_settings: ru, limit_status: limitStatus },
    `<script>setTimeout(()=>{show('meters');${services};setTimeout(()=>document.querySelectorAll('#body .row')[0].click(),100)},300)</script>`);
  // The panels at the edge offered, with one ticked
  shoot('settings.html', 'dock-pick-' + theme, '880,640', { get_settings: settings, dock_candidates: [{ raw: { exe: 'clock.exe' }, name: 'Clock', icon: null }, { raw: { exe: 'notes.exe' }, name: 'Notes', icon: null }] },
    "<script>setTimeout(()=>{show('docked');document.getElementById('add-docked').click();setTimeout(()=>document.querySelector('#dock-list .row').click(),100)},100)</script>");
  // The Look section, in Russian and with its switch on
  shoot('settings.html', 'look-' + theme, '880,640', { get_settings: { ...settings, russian: true, tablet_only: true } }, "<script>setTimeout(()=>show('look'),100)</script>");
}
