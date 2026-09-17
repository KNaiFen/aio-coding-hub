const { invoke } = window.__TAURI__.core;
const element = (id) => document.getElementById(id);
async function load() {
  try {
    const {config, hasToken, error} = await invoke('float_settings');
    for (const key of ['ip','port','fontSize','background']) element(key).value = config[key];
    element('opacity').value = Math.round(config.opacity * 100);
    element('opacity-value').value = `${element('opacity').value}%`;
    for (const key of ['alwaysOnTop','clickThrough']) element(key).checked = config[key];
    element('token').placeholder = hasToken ? '已保存；留空使用此地址的已存令牌' : '';
    element('connection-error').textContent = error || '';
  } catch(error) { element('connection-error').textContent = String(error); }
}
element('opacity').addEventListener('input', () => { element('opacity-value').value = `${element('opacity').value}%`; });
element('connection').addEventListener('submit', async (event) => {
  event.preventDefault();
  element('connect').disabled = true;
  element('connection-error').textContent = '正在连接…';
  try {
    await invoke('float_connect', { ip: element('ip').value.trim(), port: Number(element('port').value), token: element('token').value.trim() });
    element('token').value = '';
    element('connection-error').textContent = '已连接';
  } catch(error) { element('connection-error').textContent = String(error); } finally { element('connect').disabled = false; }
});
element('appearance').addEventListener('submit', async (event) => {
  event.preventDefault();
  const button = event.currentTarget.querySelector('button'); button.disabled = true;
  try {
    await invoke('float_appearance', { fontSize: Number(element('fontSize').value), background: element('background').value, opacity: Number(element('opacity').value)/100, alwaysOnTop: element('alwaysOnTop').checked, clickThrough: element('clickThrough').checked });
    element('appearance-error').textContent = '已保存';
  } catch(error) { element('appearance-error').textContent = String(error); } finally { button.disabled = false; }
});
await load();
