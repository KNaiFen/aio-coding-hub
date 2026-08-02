# 实施计划

1. 扩展可用性时间线允许 18 桶，并增加不改变 12/36 桶行为的 Rust 回归测试。
2. 将 Tray provider 快照改为 18 状态并携带总成功/失败数，补充无数据和投影测试。
3. 更新 TypeScript 服务类型与正规化测试；等待 CI 生成并应用 Specta bindings 漂移补丁。
4. 重构 `TrayProviderMiniApp` 行布局：紧凑名称/状态、18 个扁长格、固定统计列、长文本截断。
5. 把原生窗口尺寸改为 440/42/36，并启用透明 Popover effect、14px radius 和透明渲染根。
6. 更新 React 单测、入口测试和原生窗口高度断言。
7. 运行前端定向测试、完整前端测试、typecheck、lint 和 build；云端运行全部 Rust/native 验证。
8. 使用浏览器预览 mini renderer 的浅色、深色、长名称、10+ 行、无数据和多标记状态；原生透明与阴影由 macOS CI/开发制品最终确认。
