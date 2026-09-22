# AIO Float

AIO Float is the independent Windows x64 and macOS Apple Silicon floating
dashboard. It shares the AIO TUI renderer, palette, navigation and provider tests.
The display computer does not need AIO Coding Hub. Windows needs WebView2.

## Connect

1. Upgrade AIO Coding Hub on the computer providing the data.
2. Open Settings, enable LAN observation, and note its LAN IP and port (13799 by
   default). Copy the observation token.
3. Open AIO Float and enter that IP, port and token. Test and save the connection.
4. If the host firewall blocks the connection, allow the chosen TCP port for
   the private network. Neither application edits firewall rules automatically.

This connection is HTTP for a trusted LAN. It is not an Internet-facing service.
Tokens are independent of gateway credentials and the local TUI descriptor.
Disabling LAN observation, changing its port or rotating its token does not
restart or reconfigure the local TUI listener. AIO must remain running.

## Window

Drag the top summary to move the window; drag edges or corners to resize it.
Right-click opens settings, topmost, click-through, hide and exit. The Windows
tray/macOS menu bar always offers Show / Disable click-through. Closing the main
window hides it. A subtle one-pixel border separates the dashboard from the desktop.
`q` / `Q` have no action in Float; use the context/tray menu to exit (Ctrl-C also
remains available). The terminal TUI retains its `q` shortcut. A second launch
restores the first window.

On Windows, both the dashboard and settings stay out of the taskbar, including
the first-launch settings window. Use the tray to show the dashboard, open
settings or exit.

Ctrl/Cmd plus, minus and zero change/reset font size; Ctrl/Cmd-wheel zooms.
Ordinary wheel movement uses the shared TUI navigation. Font size is 8-32,
line height is 1.2 and cell width is 0.6. Cascadia Mono and its license are embedded;
Chinese characters use system fallback fonts. Background opacity leaves text
opaque. Zero opacity retains the smallest nonzero background alpha so unlocked
transparent regions remain draggable on Windows.

The application remembers geometry, appearance, CLI scope and connection address.
Tokens are stored in Windows Credential Manager or macOS Keychain, not the JSON
settings file. Hidden windows pause polling. A disconnected dashboard retains its
last snapshot; changing server clears it and rejects late responses from the old
connection. Authentication failures require an updated token.

## 组合布局与窗口锁定

- `m` 循环切换单视图、左右双列、上下单列；组合布局用一次观察轮询同时显示请求和供应商。
- `s` 互换左右或上下位置，不改变分界。两种组合布局分别记住顺序和比例，调整窗口或字号后保持比例。
- 分屏区域直接显示内容，不再占用一行显示“请求／供应商”标题。条目按实际内容行数判断是否放得下；分隔线仅在两条完整内容之间占用一行，最后一条及区域末尾不预留空行或额外画线。
- Windows 使用 `Ctrl+←/→` 每次移动一字符列，`Ctrl+↑/↓` 每次移动一行；macOS 使用 `Command+Option+Shift+对应方向键`。
- 不匹配布局的分界方向无动作。两区各保留至少一行内容；空间不足时提示调整窗口或字号。
- 点击区域切换键盘焦点；`←/→` 分别聚焦请求/供应商，与位置无关。滚轮操作鼠标所在区域，不改变键盘焦点。
- 两区分别保存选择、详情和详情滚动；`Enter` 只打开所在区域详情，`Esc` 返回该区域列表。聚焦供应商详情时 `t` 测试，`Tab` 同步切换两区 CLI 并清空旧选择与详情。
- `l` 锁定/解锁主窗口位置与尺寸。锁定仍可点击、滚动、调字号、调分界及操作内容，与鼠标穿透独立；打开设置不自动解锁。
- 右键、托盘及设置提供布局、互换、锁定入口。`m/s/l` 长按不重复触发，设置窗口不触发仪表盘快捷键。旧配置默认单视图、未锁定。

## Build and delivery

All dependency installation, generated bindings, tests and native builds run in
GitHub Actions. `float-build.yml` checks Windows x64 and macOS ARM64, publishes
an EXE or application ZIP and SHA-256 checksums as workflow artifacts. The workflow
is called by the path-classified CI and can be dispatched manually. Float and
shared TUI changes select this workflow without desktop frontend tests, desktop
Rust tests, binding generation or desktop packaging. Observer protocol, workspace
dependencies and mixed desktop changes still select the affected desktop checks.
CodeQL limits its source paths to Float/shared crates for a Float-only change,
and uses a separate analysis category to preserve desktop scan results.

Float versions are independent of the desktop/TUI version. `aio-float-vX.Y.Z`
tags invoke `float-release.yml`, validate a main-branch source, build only Float
for both platforms, and publish EXE/ZIP packages with SHA-256 checksums. These
releases do not become the repository's latest desktop release or replace the
desktop updater manifest. Replace Float with the new EXE or app to update it.
Full desktop releases may continue including the current Float package.

macOS uses the project's existing ad-hoc signing arrangement; this does not claim
Apple notarization. Native window acceptance must be recorded separately from
cloud compilation, especially on macOS when no local machine is available.

## 中文连接说明

1. 在提供数据的电脑上升级 AIO，打开设置中的“局域网观察”，开启服务并确认“运行中”。
2. 记录显示的局域网 IPv4 地址、端口，复制观察令牌。
3. 打开 AIO Float，在首次出现的设置窗口填写上述信息，点击“测试并保存连接”。

默认连接 `127.0.0.1:13799`。跨电脑使用时将 IP 改为提供数据电脑的局域网地址。
同机连接同样需要开启局域网观察服务；首版不自动发现本机描述文件。
提供数据的 AIO 必须保持运行。局域网端口范围为 `1024–65535`。

### 防火墙

- Windows：在“高级安全 Windows Defender 防火墙”中新增 TCP 端口入站规则，
  选择实际观察端口，仅勾选“专用”配置文件，并将远程地址限制为本地子网。
- macOS：若系统防火墙提示，允许 AIO Coding Hub 接收入站连接；
  使用其他防火墙时，允许来自本地子网的所选 TCP 端口。
- 两台电脑需要在可互通的局域网中，Wi-Fi 不应启用客户端隔离。
  两个应用均不自动更改防火墙或路由器设置。

### 恢复窗口

“固定”即整个窗口鼠标穿透。Windows 托盘或 macOS 菜单栏始终提供
“显示窗口 / 取消穿透”入口；打开设置也会解除穿透。关闭主窗口会隐藏，
再次启动应用恢复已有窗口。悬浮窗忽略 `q` / `Q`，退出使用右键或托盘菜单，
也保留 Ctrl-C；终端 TUI 的 `q` 不受影响。窗口有低调的 1 像素细边框，
显示器变化后自动恢复到可见区域。

Windows 下主悬浮窗和设置窗口均不占用任务栏，首次启动打开设置时也一样。
通过托盘显示窗口、打开设置或退出。

局域网开关、改端口和重置令牌不会影响原 TUI 的动态端口、描述文件或本机令牌。
新旧观察端共享快照缓存和有界数据库查询，分别限制请求与供应商测试入口。
修改端口失败保留原服务与配置；启动时局域网绑定失败不影响本机 TUI。

### 发布检查

`float-build.yml` 还提供 Playwright 视觉检查和 `float-visual-*` 截图工件，
覆盖窄窗口、高 DPI、透明度像素、字体缩放、键盘/滚轮及设置。
Float 独立维护 Cargo、锁文件中自身条目及 Tauri 配置的版本，以
`aio-float-vX.Y.Z` 标签单独发布两个平台包。仅修改锁文件中的 Float 版本时，
分类器按完整内容差异确认后走 Float 检查；其他依赖变更保留本体检查。
Float 发布不改变本体的最新版本与自动更新清单。退出后用新 EXE 或 app 替换。

仅 Float 或共享 TUI 改动会跳过本体前端、Rust、绑定生成、macOS 观察服务测试
和签名打包，保留静态合同、Float/TUI 测试、两平台构建与视觉检查。
协议、本体及公共依赖变更仍按影响范围检查。公共 CI 规则变更跑完整验证；
Float 专用构建、发布和扫描配置只触发 Float 检查。

原生验收与云端验证分别记录在 [验收记录](aio-float-acceptance.md)。
