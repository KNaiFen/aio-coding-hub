# 集成与发布设计

## 任务边界

三个子任务不共享产品状态：TUI 子任务只格式化只读投影，余额子任务只修正 React 渲染状态，Tray 子任务只同步 CSS 与原生窗口几何。父任务只拥有集成审查、版本提升、发布和证据归档。

## Git 与 CI

- 功能分支为 `codex/pending-012-015-release`，每个子任务形成独立提交。
- PR CI 负责 Rust 格式、Cargo.lock、生成绑定、Clippy、Rust 测试、审计和原生编译；只应用与本批次直接相关的 cloud-native-fixes 补丁。
- 功能合并后从最新 `main` 创建 `codex/release-0.60.45`，同步五个版本清单。Cargo.lock 版本漂移继续由 CI 生成。
- 版本 PR 的 main 合并 SHA 必须完成带 release-candidate 的成功 CI，随后才创建注释标签并运行 release workflow。
- 发布后另建归档 PR，避免在 Release 标签中提前声称任务已完成。

## 失败处理

任何子任务或候选制品失败都阻止发布。与任务无关的现存缺陷不混入本批次；标签自动工作流若仅命中已知本地标签 clobber 问题，则从 `main` 对同一已解析标签执行 `workflow_dispatch`，不重写标签。
