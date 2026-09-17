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
window hides it; `q` exits as in the TUI. A second launch restores the first window.

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

## Build and delivery

All dependency installation, generated bindings, tests and native builds run in
GitHub Actions. `float-build.yml` checks Windows x64 and macOS ARM64, publishes
an EXE or application ZIP and SHA-256 checksums as workflow artifacts. The workflow
also runs on version tags and can be dispatched manually. Main CI builds both
Float packages alongside the signed AIO release candidate. The release workflow
publishes these exact packages with the shared SHA-256 manifest. Float packages
are not added to the desktop updater manifest: update AIO through its existing
updater, and replace Float with the new EXE or app from the release downloads.
Companion development builds use `dev-build` for the same source revision.

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
再次启动应用恢复已有窗口；`q` 退出应用。显示器变化后自动恢复到可见区域。

局域网开关、改端口和重置令牌不会影响原 TUI 的动态端口、描述文件或本机令牌。
新旧观察端共享快照缓存和有界数据库查询，分别限制请求与供应商测试入口。
修改端口失败保留原服务与配置；启动时局域网绑定失败不影响本机 TUI。

### 发布检查

`float-build.yml` 还提供 Playwright 视觉检查和 `float-visual-*` 截图工件，
覆盖窄窗口、高 DPI、透明度像素、字体缩放、键盘/滚轮及设置。
发布版本须同步根包、主程序、协议、TUI、Float 的 Cargo/锁文件与 Tauri 配置；
版本一致性校验包含 Float。正式版由主 CI 同时构建 AIO 与 Float，发布流程
复用同一提交的候选包，统一提供校验和。AIO 可使用现有更新入口；Float 无自动
更新，退出后用发布页的新 EXE 或 app 替换。开发验收包仍可使用同一提交的
`dev-build` 工作流，分别选择 `windows-x64` 与 `macos-arm64`。

原生验收与云端验证分别记录在 [验收记录](aio-float-acceptance.md)。
