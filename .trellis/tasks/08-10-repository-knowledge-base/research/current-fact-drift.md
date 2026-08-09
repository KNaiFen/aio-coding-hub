# 现行文档事实漂移

## 已确认错误

1. `hostCompatibility.platforms` 已由 `src-tauri/src/domain/plugins.rs::validate_host_compatibility` 对 `std::env::consts::OS` 强制校验，不匹配返回 `PLUGIN_INCOMPATIBLE_PLATFORM`。多份文档仍称只展示、不阻断。
2. Extension Host 已提供 capability-gated `diagnostics.getRuntimeReports`。SDK 暴露 `PluginApi.diagnostics`；宿主只允许读取当前插件报告，limit 收敛到 `1..100`。运行时 README 仍否认 plugin-callable diagnostics API。
3. 应用当前版本为 `0.60.50`，而多份现行插件文档把不存在的 `0.62.x` 写成当前发布线；文档检查脚本还锁定这些短语。
4. `CHANGELOG.md` 顶部停在 `0.60.32`，Git 标签已经连续到 `0.60.50`。
5. `protocol_bridge/README.md` 指导本地运行 Cargo，与根 `AGENTS.md` 的零产物边界冲突。
6. `.trellis/workflow.md` 最后指向两个从未存在的契约/脚本路径；workspace 根索引又错误声称没有开发者记录。

## 历史而非现行错误

- `docs/plugins/architecture/current-plugin-system-audit-2026-07-02.md` 的 completion CI、平台和失败策略结论反映审计时点，后续已经变化；应改名迁入历史区，不逐句重写。
- `docs/plugin-system-development-plan.md` 已标记 superseded，其中旧路径、WASM/process 路线、本地测试矩阵和平台发布假设应保留为历史，不再放在现行文档入口。
- `omx_wiki` 两份正文是带日期的问题分析/合并记录；旧命令和路径仅作为当时证据。
