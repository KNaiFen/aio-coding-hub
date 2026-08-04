# 悬浮窗现状调研

## 入口与几何

- 前端入口：`src/tray/TrayProviderMiniApp.tsx:207`；窗口模式分流：`src/main.tsx:43`。
- 原生窗口宽 404px，头 42px、行 24px、最多 10 行：`src-tauri/src/app/resident.rs:17,280`。
- 当前列为 `96px / 170px / 96px`，间隔各 8px：`src/tray/TrayProviderMiniApp.tsx:163`。
- 18 格状态条为 170px、格间 2px：`src/tray/TrayProviderMiniApp.tsx:17,104`。
- 成败区为 `44px | 1px | 44px`，每组含 12px 标签和 32px 数字：`src/tray/TrayProviderMiniApp.tsx:121`。
- 状态字为 18x18px 的“熔/冷/限”，状态容器当前已在供应商列内：`src/tray/TrayProviderMiniApp.tsx:33,74`。

## 背景现状

- React 面板已有半透明背景和 `backdrop-blur-xl`，但没有流体形变或动画：`src/tray/TrayProviderMiniApp.tsx:251`。
- macOS 原生窗口已有透明底、`Effect::Popover`、圆角和阴影：`src-tauri/src/app/resident.rs:440`。
- 该窗口的 `html/body/#root` 已清成透明：`src/styles/globals.css:314`。

结论：现有实现是原生 Popover 毛玻璃，不是明确的 Liquid Glass 视觉；浏览器预览也无法验证原生材质。
