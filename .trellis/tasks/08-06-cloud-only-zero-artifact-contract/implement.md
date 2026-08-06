# AUD-054 实施清单

- [x] 失败优先证明 AGENTS/README/package scripts/Trellis 仍暴露本地入口。
- [x] 新增零依赖 checker 与 self-test，并接入 `support-contract`。
- [x] 收窄根、plugin-sdk、create-aio-plugin 与聚合脚本的 Actions-only 合同。
- [x] 更新 AGENTS、README 中英文、活跃 Trellis agent 模板和规范。
- [x] 锁定 CI 的 Rust、bindings、Clippy、tests、audit 与前端质量门。
- [x] 保留 `ci.yml workflow_dispatch`、`dev-build workflow_dispatch` 和非 PR 桌面打包语义。
- [x] 运行允许的 checker/self-test、Node 语法、配置解析和 `git diff --check`。
- [x] 独立审查完整差异并确认没有历史任务/归档改写。
- [ ] 推送精确候选，触发全量 CI，等待 `ci-gate` 后合并。
- [ ] 合并后重新核验并删除精确仓库产物目录。
