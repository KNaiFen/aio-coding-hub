# macOS Popover 设计与 Tauri 能力调研

## 官方资料

- Apple Human Interface Guidelines, Menus and actions / Popovers：Popover 是与触发来源关联的短暂界面，适合承载少量聚焦信息，关闭和交互不应打断主任务。
  - https://developer.apple.com/design/human-interface-guidelines/menus-and-actions
- Apple AppKit `NSPopover`：macOS 原生用于相对界面元素显示短暂内容的容器。
  - https://developer.apple.com/documentation/appkit/nspopover
- Tauri 2 Window customization：透明窗口可通过 `transparent` 配置/构建器启用；macOS 需要 `macos-private-api` 能力，且该能力不适合 Mac App Store 分发。
  - https://v2.tauri.app/learn/window-customization/
  - https://v2.tauri.app/reference/config/
- Tauri 2 `EffectsBuilder`：`Effect::Popover`、`EffectState::Active` 和 macOS `radius` 可组合为原生 Popover 材质与圆角效果，但窗口必须透明。
  - https://docs.rs/tauri/latest/tauri/window/struct.EffectsBuilder.html

## 对本项目的结论

- AIO 当前通过 GitHub Release 分发，不以 Mac App Store 为发布目标，可以为这个 macOS-only 辅助窗口启用透明窗口能力。
- 只给 React `<main>` 添加 `border-radius` 仍会留下方形原生窗口角区，无法满足用户要求；必须同时让原生窗口和 WebView 背景透明。
- 使用系统 Popover material 和系统 shadow，比自绘大阴影、渐变或玻璃卡片更符合当前“克制、紧凑、可靠”的产品注册表。
- 圆角采用 14px；它落在产品面板 12–16px 的合理范围，不会出现过度圆润。
- 信息密度通过固定列宽、36px 行高和 6px 高状态格提升；不增加动画或额外控件。
