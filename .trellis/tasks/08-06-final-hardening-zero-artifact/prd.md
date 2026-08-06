# 最终修复与本地零产物

## Goal

完成八项最终治理修复，并以本地零产物、云端完整验证的方式逐项合并。

## Requirements

- 固定实施顺序：AUD-054、AUD-055、AUD-056、AUD-016、AUD-008、AUD-002、AUD-035、AUD-033。
- 每项使用独立短分支和 PR；合并后才开始下一项，并在下一候选分支补记前一项证据。
- 本地不得安装依赖、启动开发服务、运行 pnpm/Cargo/Tauri、格式化、类型检查、Lint、测试或构建。
- 本地只允许运行无依赖 Node 源码合同检查、源文件解析检查和 `git diff --check`。
- Rust、bindings、Clippy、测试、前端质量门和原生制品均由 GitHub Actions 负责。
- 每个 PR 前与合并前重新 fetch `origin/main`，核对主线和开放 PR 的功能、接口、实现和最终效果。
- 不纳入旧 worktree 的其他未跟踪任务、缓存或产物，不操作 `upstream`。
- 第八项合并后另建纯文档 PR，收口报告、PENDING 和 Trellis 证据。

## Acceptance Criteria

- [ ] 八个子任务均通过精确分支的 `ci.yml` 全量 `workflow_dispatch` 和 `ci-gate`。
- [ ] 八个实现 PR 均已合并且主线树与候选一致。
- [ ] AUD-054 合并后只清理核验过的仓库级 Node/Rust 产物，不删除全局缓存或其他项目文件。
- [ ] 八项在审计报告和 `PENDING.md` 中都有 PR、提交和 CI 证据。
- [ ] 最终纯文档 PR 合并后，八项从活跃待办迁入完成归档。
- [ ] 本批没有未记录的根本冲突、被静默省略的范围或本地生成产物。

## Notes

- 本任务只负责任务映射、顺序和最终集成审查；产品实现由八个子任务分别负责。
