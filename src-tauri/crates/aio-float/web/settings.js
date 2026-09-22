import { dimensions, drawFrame } from './renderer.js';

const { invoke } = window.__TAURI__.core;
const element = (id) => document.getElementById(id);
const tabs = [...document.querySelectorAll('[role=tab]')];
let previewPending = false;
let previewDirty = false;

function selectTab(tab, focus = false) {
  for (const item of tabs) {
    const selected = item === tab;
    item.setAttribute('aria-selected', String(selected));
    item.tabIndex = selected ? 0 : -1;
    element(item.getAttribute('aria-controls')).hidden = !selected;
  }
  if (focus) tab.focus();
  if (tab.id === 'appearance-tab') void updatePreview();
}
for (const [index, tab] of tabs.entries()) {
  tab.addEventListener('click', () => selectTab(tab));
  tab.addEventListener('keydown', (event) => {
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 :
      event.key === 'ArrowRight' ? (index + 1) % tabs.length :
      event.key === 'ArrowLeft' ? (index + tabs.length - 1) % tabs.length : -1;
    if (next >= 0) { event.preventDefault(); selectTab(tabs[next], true); }
  });
}

function feedback(id, message = '', state = '') {
  const target = element(id);
  target.textContent = message;
  target.className = `feedback ${state}`;
}
function appearance() {
  return {
    fontSize: Number(element('fontSize').value),
    background: element('background').value,
    opacity: Number(element('opacity').value) / 100,
    alwaysOnTop: element('alwaysOnTop').checked,
    clickThrough: element('clickThrough').checked,
    locked: element('locked').checked,
  };
}
function syncControls() {
  const config = appearance();
  element('opacity-value').value = `${Math.round(config.opacity * 100)}%`;
  element('opacity').style.setProperty('--progress', `${config.opacity * 100}%`);
  element('background-value').value = config.background.toUpperCase();
  element('font-down').disabled = config.fontSize <= 8;
  element('font-up').disabled = config.fontSize >= 32;
  document.querySelectorAll('[data-color]').forEach(button => {
    button.setAttribute('aria-pressed', String(button.dataset.color.toLowerCase() === config.background.toLowerCase()));
  });
}
async function updatePreview() {
  if (element('appearance-panel').hidden || !element('fontSize').validity.valid) return;
  if (previewPending) { previewDirty = true; return; }
  const canvas = element('preview');
  const { width, height } = canvas.getBoundingClientRect();
  if (!width || !height) return;
  previewPending = true;
  previewDirty = false;
  try {
    // Use the real TUI grid and palette; appearance changes stay local until applied.
    const config = appearance();
    const size = dimensions(width, height, config.fontSize);
    const frame = await invoke('float_frame', { columns: size.columns, rows: 24 });
    drawFrame(canvas, { ...frame, config: { ...frame.config, ...config } }, width, height, devicePixelRatio);
  } catch (error) { feedback('appearance-error', String(error)); }
  finally { previewPending = false; if (previewDirty) void updatePreview(); }
}
async function load() {
  try {
    const { config, hasToken, error } = await invoke('float_settings');
    for (const key of ['ip', 'port', 'fontSize', 'background']) element(key).value = config[key];
    element('opacity').value = Math.round(config.opacity * 100);
    for (const key of ['alwaysOnTop', 'clickThrough', 'locked']) element(key).checked = config[key];
    element('layout').value = config.layout;
    element('swap').disabled = config.layout === 'single';
    element('token').placeholder = hasToken ? '已安全保存' : '输入访问令牌';
    element('connection-state').textContent = hasToken ? '已配置' : '未配置';
    element('connection-state').classList.toggle('configured', hasToken);
    feedback('connection-error', error || '');
    syncControls();
    if (hasToken && !error) selectTab(element('appearance-tab'));
  } catch (error) { feedback('connection-error', String(error)); }
  finally {
    element('connection-fields').disabled = false;
    element('appearance-fields').disabled = false;
  }
}
element('token-visibility').addEventListener('click', () => {
  const show = element('token').type === 'password';
  element('token').type = show ? 'text' : 'password';
  const button = element('token-visibility');
  const label = show ? '隐藏令牌' : '显示令牌';
  button.setAttribute('aria-label', label);
  button.setAttribute('aria-pressed', String(show));
  button.title = label;
  button.firstElementChild.className = `icon ${show ? 'eye-off' : 'eye'}`;
});
element('connection').addEventListener('input', () => feedback('connection-error'));
element('connection').addEventListener('submit', async (event) => {
  event.preventDefault();
  element('connection-fields').disabled = true;
  element('connect-label').textContent = '正在连接';
  feedback('connection-error', '正在验证连接…', 'pending');
  try {
    await invoke('float_connect', { ip: element('ip').value.trim(), port: Number(element('port').value), token: element('token').value.trim() });
    element('token').value = '';
    element('token').placeholder = '已安全保存';
    element('connection-state').textContent = '已配置';
    element('connection-state').classList.add('configured');
    feedback('connection-error', '已连接', 'success');
  } catch (error) { feedback('connection-error', String(error)); }
  finally { element('connection-fields').disabled = false; element('connect-label').textContent = '测试并保存连接'; }
});
function appearanceChanged() { syncControls(); feedback('appearance-error'); void updatePreview(); }
element('appearance').addEventListener('input', appearanceChanged);
for (const [id, delta] of [['font-down', -1], ['font-up', 1]]) {
  element(id).addEventListener('click', () => {
    element('fontSize').value = Math.max(8, Math.min(32, Number(element('fontSize').value || 12) + delta));
    appearanceChanged();
  });
}
document.querySelectorAll('[data-color]').forEach(button => button.addEventListener('click', () => {
  element('background').value = button.dataset.color;
  appearanceChanged();
}));
element('appearance').addEventListener('submit', async (event) => {
  event.preventDefault();
  element('appearance-fields').disabled = true;
  feedback('appearance-error', '正在保存…', 'pending');
  try { await invoke('float_appearance', appearance()); feedback('appearance-error', '已保存', 'success'); }
  catch (error) { feedback('appearance-error', String(error)); }
  finally { element('appearance-fields').disabled = false; }
});
new ResizeObserver(() => { void updatePreview(); }).observe(element('preview'));
async function changeLayout(action) {
  try {
    await invoke('float_layout', { action });
    const { config } = await invoke('float_settings');
    element('layout').value = config.layout;
    element('swap').disabled = config.layout === 'single';
    feedback('appearance-error');
    void updatePreview();
  } catch (error) { feedback('appearance-error', String(error)); }
}
element('layout').addEventListener('change', () => { void changeLayout(element('layout').value); });
element('swap').addEventListener('click', () => { void changeLayout('swap'); });
await document.fonts.load('12px Cascadia');
await load();
