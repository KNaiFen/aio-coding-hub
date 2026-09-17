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
    drawFrame(canvas, frame, innerWidth, innerHeight, devicePixelRatio);
    document.querySelector('#drag').style.height = `${config.fontSize * 2.4}px`;
  } catch (error) { message.textContent = String(error); } finally { running = false; }
}
async function appearance(fontSize) {
  try { await invoke('float_appearance', { ...config, fontSize: Math.min(32, Math.max(8, fontSize)) }); await refresh(); }
  catch(error) { message.textContent = String(error); }
}
window.addEventListener('contextmenu', (event) => { event.preventDefault(); void invoke('float_menu').catch((error) => { message.textContent = String(error); }); });
document.querySelector('#drag').addEventListener('mousedown', (event) => { if (event.button === 0) void currentWindow.startDragging(); });
document.querySelectorAll('[data-edge]').forEach((edge) => edge.addEventListener('mousedown', (event) => { if (event.button === 0) { event.preventDefault(); void currentWindow.startResizeDragging(edge.dataset.edge); } }));
window.addEventListener('keydown', (event) => {
  const modifier = event.ctrlKey || event.metaKey;
  if (modifier && ['+', '=', '-', '0'].includes(event.key)) {
    event.preventDefault(); void appearance(event.key === '0' ? 12 : config.fontSize + (event.key === '-' ? -1 : 1)); return;
  }
  if (event.altKey || event.metaKey) return;
  event.preventDefault();
  void invoke('float_key', { key: event.key, control: event.ctrlKey }).then(refresh).catch((error) => { message.textContent = String(error); });
});
let wheelDelta = 0;
window.addEventListener('wheel', (event) => {
  event.preventDefault();
  wheelDelta += event.deltaY;
  if (Math.abs(wheelDelta) < 30) return;
  const down = wheelDelta > 0;
  wheelDelta = 0;
  if (event.ctrlKey || event.metaKey) void appearance(config.fontSize + (down ? -1 : 1));
  else void invoke('float_key', { key: down ? 'ArrowDown' : 'ArrowUp', control: false }).then(refresh).catch((error) => { message.textContent = String(error); });
}, { passive: false });
window.addEventListener('resize', () => { if (lastFrame) drawFrame(canvas, lastFrame, innerWidth, innerHeight, devicePixelRatio); void refresh(); });
await window.__TAURI__.event.listen('float-config', () => { void refresh(); });
await window.__TAURI__.event.listen('float-error', ({payload}) => { transientError = String(payload); message.textContent = transientError; setTimeout(() => { transientError = ''; }, 5000); });
await document.fonts.load('12px Cascadia');
await document.fonts.ready;
await refresh();
setInterval(refresh, 200);
