# 最终修复与本地零产物

## Goal

完成八项最终治理修复，并以本地零产物、云端完整验证的方式完成主线交付。AUD-054、AUD-056、AUD-016、AUD-008 已进入主线；AUD-055、AUD-002、AUD-035、AUD-033 由一个统一 PR 完成验证与合并。

## Requirements

- 保留既定实施顺序与历史证据：AUD-054、AUD-055、AUD-056、AUD-016、AUD-008、AUD-002、AUD-035、AUD-033。
- 已合并的 AUD-054、AUD-056、AUD-016、AUD-008 以主线事实为准；尚未合并的 AUD-055、AUD-002、AUD-035、AUD-033 统一由 `codex/final-hardening-unified` 面向 `main` 的唯一 PR 交付。
- 本地不得安装依赖、启动开发服务、运行 pnpm/Cargo/Tauri、格式化、类型检查、Lint、测试或构建。
- 本地只允许运行无依赖 Node 源码合同检查、源文件解析检查和 `git diff --check`。
- Rust、bindings、Clippy、测试、前端质量门和原生制品均由 GitHub Actions 负责。
- 统一 PR 创建前与合并前重新 fetch `origin/main`，核对主线和开放 PR 的功能、接口、实现和最终效果。
- 不纳入旧 worktree 的其他未跟踪任务、缓存或产物，不操作 `upstream`。
- 报告、PENDING 和 Trellis 的当前拓扑随统一 PR 一起更新；不再创建额外纯文档 PR。

## Acceptance Criteria

- [ ] 已合并四项保留各自精确 SHA 的 Actions 与主线树证据；剩余四项通过统一 PR 精确 head 的 `ci.yml` 全量 `workflow_dispatch` 和 `ci-gate`。
- [ ] 统一 PR 已合并，主线树与统一候选一致，旧 PR #94、#93、#92、#87 已以统一 PR 链接说明替代关系并关闭。
- [ ] AUD-054 合并后只清理核验过的仓库级 Node/Rust 产物，不删除全局缓存或其他项目文件。
- [ ] 八项在审计报告和 `PENDING.md` 中都有 PR、提交和 CI 证据。
- [ ] 统一 PR 合并并完成主线核验后，仍在活跃列表的条目按仓库规则迁入完成归档。
- [ ] 本批没有未记录的根本冲突、被静默省略的范围或本地生成产物。

## Notes

- 本任务负责八项任务映射和最终集成审查；剩余四个子任务共享同一交付分支与 PR，但继续保留各自需求、实现与验证证据。
