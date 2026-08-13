# 验收返工：跨供应商模型路由

## Round 1

- 验收候选 head：`bbe3e8bb96ef09cdff6b791b7ee4d1d9c29b9f4d`
- PR：[KNaiFen/aio-coding-hub#137](https://github.com/KNaiFen/aio-coding-hub/pull/137)（Draft）
- 失败 CI：[run 31736969410](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31736969410)
- 验收结论：未进入产品 diff 验收。AC-10 的同一 head 必需 CI 全绿与 Ready 状态未满足，先按本 finding 恢复交付门。
- 返工责任：执行 session。main 已通过独立 PR #140 修复仓库基线；本轮只要求同步基线、复验和更新交付记录，不要求重做产品实现。

### F-001 [P1] 同步已修复的 main 基线并重新取得完整交付证据

**证据**

- 候选 head 的 frontend job `94570993792` 在依赖审计阶段报告 `nanoid / GHSA-2v37-7h3g-55p8` 后失败，lint、typecheck、frontend tests 和 build 未执行；`ci-gate` job `94577125786` 因 `FRONTEND_RESULT=failure` 失败。
- PR #137 未修改 `package.json`、`pnpm-workspace.yaml` 或 `pnpm-lock.yaml`，该失败不是跨供应商路由 diff 引入的回归。
- main 已通过 [PR #140](https://github.com/KNaiFen/aio-coding-hub/pull/140) 把 `nanoid` 从受影响的 `3.3.17` 升到首个安全 3.x 版本 `3.3.18`；最终 head `abe221420d597a4afed873f85e75686ddb72fce7` 的 [CI run 31752575312](https://github.com/KNaiFen/aio-coding-hub/actions/runs/31752575312) 已通过 frozen install、pnpm audit、frontend、Rust、docs/support contract、CodeQL、`pr-title` 和 `ci-gate`。实际 main merge commit 为 `a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885`。
- 只读 `git merge-tree` 预检显示 `origin/main` 可普通 merge 到当前任务分支，不存在内容冲突。`.trellis/tasks/README.md` 是双方都修改但可自动合并的文件；`pnpm-workspace.yaml`、`pnpm-lock.yaml` 和 `docs/history/change-records/2026-08.md` 是 main 单边变更，必须完整保留。

**影响**

- AC-03、AC-04、AC-08、AC-09 缺少最新候选 head 的 frontend 云端测试/构建证据。
- AC-10 未满足；PR 不能标记 Ready，main 不能开始固定 head 的产品验收或合并。

**期望结果**

1. 在登记 worktree 和分支中重新做 preflight，确认路径、分支、`task.json.status=in_progress`、规划提交、工作树干净且本地/远端 head 一致。
2. `git fetch origin` 后使用普通 merge 同步 `origin/main`，不要 rebase、不要 force-push、不要 cherry-pick PR #140：

   ```bash
   git merge --no-edit origin/main
   ```

3. 保留 `origin/main` 的全部非冲突变化，尤其是 `nanoid 3.3.18` 的 workspace/lockfile 修复和 TUI 任务归档事实。若实际出现与只读预检不一致的内容冲突，或必须改变产品行为/现行合同才能继续，立即停止并报告 main。
4. 不修改跨供应商产品实现、测试逻辑、`src-tauri/crates/aio-tui/src/format.rs`、依赖版本或审计 allowlist；除任务 `delivery.md`/`execution.md`/`findings.md` 等交付记录外，本轮产品树变化只应来自合并 `origin/main`。
5. 运行 `execution.md` 允许的本地检查，推送新的完整 head，等待该 head 的自动 `change-scope`、frontend、Rust、docs/support contract、`pr-title`、CodeQL 和 `ci-gate` 终态；不要手动启动额外 `ci` run。
6. 若 CI 发现属于跨供应商路由 diff 的新失败，按原任务范围修复；若出现新的基线/基础设施失败且没有任务内安全修法，保留日志并停止报告 main。
7. 基于实际新 head 更新 `delivery.md`：删除已解除的 nanoid 阻塞表述，记录 merge commit、完整 PR head/base SHA、同一 head 的 `ci-gate` 链接、frontend/Rust/本地检查结果、剩余人工验证风险和 Round 1 返工结果。检查绿色后把 PR 标记 Ready for review，停止写入并通知 main 验收。

**复验标准**

- `git merge-base --is-ancestor a0f6823be399b2a2a31fb27b9fdfb0b6cb60e885 HEAD` 成功。
- worktree 干净，本地 `HEAD`、远端任务分支和 PR `headRefOid` 完全一致。
- 最新完整 head 的 required `ci-gate`、`pr-title` 及其选中的 frontend、Rust、docs/support contract 和 CodeQL 全部成功；没有沿用 `bbe3e8bb...` 的过期证据。
- `delivery.md` 与实时 PR/CI 一致，PR 已从 Draft 切换为 Ready for review；执行 session 已停止写入。
- main 将在新的固定 head 上重新进行产品 diff、AC、回归风险、测试与文档验收。此次基线修复绿色不预先代表 PR #137 的产品验收通过。
