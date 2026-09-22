import { drawFrame, dimensions } from './renderer.js';
const { invoke } = window.__TAURI__.core;
const currentWindow = window.__TAURI__.window.getCurrentWindow();
const canvas = document.querySelector('#grid');
const message = document.querySelector('#message');
let config = { fontSize: 12, background: '#272B33', opacity: 1, alwaysOnTop: true, clickThrough: false };
let running = false;
let lastFrame;
let transientError = '';
function gridDimensions() {
  const statusHeight = message.textContent ? message.getBoundingClientRect().height + 6 : 0;
  return dimensions(innerWidth, innerHeight - statusHeight, config.fontSize);
}
async function refresh() {
  if (running || document.hidden) return;
  running = true;
  try {
    const size = gridDimensions();
    let frame = await invoke('float_frame', { columns: size.columns, rows: size.rows });
    config = frame.config;
    message.textContent = transientError || frame.error || (!frame.connected ? '尚未连接 AIO' : '');
    const actual = gridDimensions();
    if (actual.columns !== size.columns || actual.rows !== size.rows) frame = await invoke('float_frame', actual);
    lastFrame = frame;
    document.body.classList.toggle('locked', config.locked);
    drawFrame(canvas, frame, innerWidth, innerHeight, devicePixelRatio);
    document.querySelector('#drag').style.height = `${config.fontSize * 2.4}px`;
  } catch (error) { message.textContent = String(error); } finally { running = false; }
}
async function appearance(fontSize) {
  try { await invoke('float_appearance', { ...config, fontSize: Math.min(32, Math.max(8, fontSize)) }); await refresh(); }
  catch(error) { message.textContent = String(error); }
}
window.addEventListener('contextmenu', (event) => { event.preventDefault(); void invoke('float_menu').catch((error) => { message.textContent = String(error); }); });
document.querySelector('#drag').addEventListener('mousedown', (event) => { if (event.button === 0 && !config.locked) void currentWindow.startDragging().catch(showError); });
document.querySelectorAll('[data-edge]').forEach((edge) => edge.addEventListener('mousedown', (event) => { if (event.button === 0 && !config.locked) { event.preventDefault(); void currentWindow.startResizeDragging(edge.dataset.edge).catch(showError); } }));
function showError(error) { message.textContent = String(error); }
function paneAt(event) {
  const x = Math.floor((event.clientX - 6) / (config.fontSize * .6));
  const y = Math.floor((event.clientY - 6) / (config.fontSize * 1.2));
  return lastFrame?.regions.find(region => x >= region.x && x < region.x + region.width && y >= region.y && y < region.y + region.height)?.pane;
}
canvas.addEventListener('mousedown', event => {
  const pane = paneAt(event);
  if (event.button === 0 && pane) void invoke('float_focus', { pane }).then(refresh).catch(showError);
});
window.addEventListener('keydown', (event) => {
  if (event.isComposing || !document.hasFocus()) return;
  const arrows = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'];
  const divider = lastFrame?.macos
    ? event.metaKey && event.altKey && event.shiftKey && !event.ctrlKey
    : event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey;
  if (divider && arrows.includes(event.key)) {
    event.preventDefault(); void invoke('float_layout', { action: event.key }).then(refresh).catch(showError); return;
  }
  if (!event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey && ['m', 's', 'l'].includes(event.key)) {
    event.preventDefault();
    if (!event.repeat) void (event.key === 'l' ? invoke('float_toggle_lock') : invoke('float_layout', { action: event.key === 'm' ? 'cycle' : 'swap' })).then(refresh).catch(showError);
    return;
  }
  const modifier = event.ctrlKey || event.metaKey;
  if (modifier && ['+', '=', '-', '0'].includes(event.key)) {
    event.preventDefault(); void appearance(event.key === '0' ? 12 : config.fontSize + (event.key === '-' ? -1 : 1)); return;
  }
  if (event.altKey || event.metaKey || (event.ctrlKey && event.key !== 'c')) return;
  event.preventDefault();
  void invoke('float_key', { key: event.key, control: event.ctrlKey, target: null }).then(refresh).catch(showError);
});
let wheelDelta = 0;
window.addEventListener('wheel', (event) => {
  event.preventDefault();
  wheelDelta += event.deltaY;
  if (Math.abs(wheelDelta) < 30) return;
  const down = wheelDelta > 0;
  wheelDelta = 0;
  if (event.ctrlKey || event.metaKey) void appearance(config.fontSize + (down ? -1 : 1));
  else {
    const target = paneAt(event);
    if (target) void invoke('float_key', { key: down ? 'ArrowDown' : 'ArrowUp', control: false, target }).then(refresh).catch(showError);
  }
}, { passive: false });
window.addEventListener('resize', () => { if (lastFrame) drawFrame(canvas, lastFrame, innerWidth, innerHeight, devicePixelRatio); void refresh(); });
await window.__TAURI__.event.listen('float-config', () => { void refresh(); });
await window.__TAURI__.event.listen('float-error', ({payload}) => { transientError = String(payload); message.textContent = transientError; setTimeout(() => { transientError = ''; }, 5000); });
await document.fonts.load('12px Cascadia');
await document.fonts.ready;
await refresh();
setInterval(refresh, 200);
