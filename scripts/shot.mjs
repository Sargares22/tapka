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

function shoot(page, name, size, data, after = '') {
  const work = mkdtempSync(join(tmpdir(), 'tapka-shot-'));
  const stub = `<script>window.__on={};window.__TAURI__={core:{invoke:async c=>(${JSON.stringify(data)})[c]??null},event:{listen(n,cb){window.__on[n]=cb}}}</script>`;
  const html = readFileSync(join(root, 'ui', page), 'utf8').replace('<head>', '<head>' + stub).replace('</body>', after + '</body>');
  writeFileSync(join(work, 'page.html'), html);
  execFileSync(chrome, ['--headless=new', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=2', `--window-size=${size}`,
    '--default-background-color=00000000', `--user-data-dir=${join(work, 'profile')}`, '--virtual-time-budget=1500',
    `--screenshot=${join(out, name + '.png')}`, pathToFileURL(join(work, 'page.html')).href], { stdio: 'ignore' });
  rmSync(work, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  console.log(join('target', 'shots', name + '.png'));
}

for (const [theme, accent] of [['dark', '#8ab4ff'], ['light', '#ff8a3d']]) {
  const items = sample.slice(0, 6).map(i => ({ name: i.name, hint: i.hint, glyph: glyphs[i.icon] || null, icon: null, on: i.name === 'Notes', live: i.name === 'Voice typing' }));
  // The capsule's own window is 348 px wide: the strip and the room for the label
  const box = '<style>html,body{width:348px!important;height:340px!important}body{position:relative}</style>';
  const hover = "<script>setTimeout(()=>{const r=document.querySelector('.item').getBoundingClientRect();window.__on.hover({payload:[r.left+r.width/2,r.top+r.height/2]})},100)</script>";
  shoot('index.html', 'capsule-' + theme, '348,340', { get_view: { items, cell: 54, tablet: false, edge: 'right', theme, accent, add: ['Add', 'Open the item editor'] } }, box + hover);
  const settings = {
    items: sample.map(raw => ({ raw, icon: null, glyph: glyphs[raw.icon] || null, problem: null })),
    scale: 1, edge: 'right', theme, accent, lang: 'en', russian: false, autostart: false, updates: true, version: '0.0.0', error: null, intro: false,
    presets: sample.slice(0, 3).map(item => ({ id: item.icon, item })), glyphs, update: { state: 'none' },
  };
  shoot('settings.html', 'settings-' + theme, '880,1100', { get_settings: settings });
}
