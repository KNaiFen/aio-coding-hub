# 集成设计

## 任务边界

本批次包含三个独立交付面：

1. 独立 TUI 文本渲染，仅修改 `aio-tui` 的摘要和时间格式。
2. 待办文档分层，将活跃列表与历史归档分开。
3. macOS Tray mini 的观测投影、前端渲染和原生窗口外观。

三个面不共享运行时状态，也不修改网关转发语义。统一放入一个补丁版本是为了减少重复发布和资产验证。

## 子任务映射

- `08-02-tui-summary-local-time`：PRD-only 轻量子任务。
- `08-02-pending-completed-archive`：PRD-only 文档子任务。
- `08-02-macos-tray-mini-polish`：跨 Rust 投影、生成绑定、TypeScript 边界、React 和 macOS 窗口的复杂子任务，拥有独立 design/implement/research。

## 集成与发布

- 所有变更进入 `codex/tui-polish-release` 短期分支。
- 本地先完成前端测试、TypeScript、lint、Vite build 和静态 diff 检查。
- PR 云端 CI 负责 Rust 格式、Cargo.lock、生成绑定、Rust 测试、Clippy、audit 和原生集成。
- 若 CI 产出有界漂移补丁，只应用与本任务直接相关的格式、锁文件或绑定更新并重新验证。
- 合并后提交 `0.60.44` 版本提升 PR；精确 `main` CI 成功后发布 `aio-coding-hub-v0.60.44`，不在标签流程重新构建。

## 回滚

- 任一子任务出现阻断性回归时不发布版本，优先回滚该子任务的独立提交。
- 原生圆角效果若在 macOS CI 不可用，保留紧凑布局和统计功能，回滚透明/Popover 窗口配置后重新评估；不得以破坏悬停生命周期换取视觉效果。
