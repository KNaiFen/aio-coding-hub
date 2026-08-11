# 返工意见：08-10 GitHub Actions Governance 收尾记录

## Round 1 - 2026-08-12

- 审查候选：PR [#115](https://github.com/KNaiFen/aio-coding-hub/pull/115) @
  `708174e7d7a1b1431d40a1db2d2b6119a4d26c60`。
- CI 证据：该 head 的 `ci-gate`、`pr-title`、CodeQL JS/TS 和 Rust 均成功；PR
  当前为 Draft、OPEN、CLEAN。CI 不是本轮阻断项。
- 审查范围：records-only closeout 的四份任务记录、任务目录解析和与 PR #114 的
  交叉引用。
- 结论：不通过；须先修正以下记录完整性问题。除本文件明确要求外，不得扩大范围。

## F-001：后续任务交叉引用指向不存在的目录

**严重性：必须修复。**

`delivery.md`、`execution.md` 和 `task.json` 将后续修复写为
`08-11-upstream-sync-pr-resolution`，但实际独立任务由 Trellis 创建为
`08-12-upstream-sync-pr-resolution`，且它仅存在于 PR #114 的分支；#115 的
`HEAD` 和 `main` 均没有该目录。因此当前相对链接
`../08-11-upstream-sync-pr-resolution/` 在 #115 合并后必然失效，并会让未来读者
误以为旧任务包保存了 follow-up 的正式材料。

### 所需改动

1. 在 #115 的所有 08-10 正式记录中，删除指向
   `../08-11-upstream-sync-pr-resolution/` 的相对链接和错误任务 ID。
2. 若需保留任务身份，使用纯文本
   `08-12-upstream-sync-pr-resolution`，并只保留持久可访问的
   [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114) 链接。不要链接一个
   尚未合并到当前分支或 `main` 的任务目录。
3. 保持 #108 的 merge commit、部分完成结论和 PR #114 的“仅交叉引用、不继承其
   实现/CI/验收事实”边界不变。
4. 不得改动 `.github/`、产品代码、同步脚本、测试合同、`upgrade-tui.command`、
   `SESSION_REMEDIATION_PLAN.md` 或 08-11 worktree 的任何文件。

### 复验方式

```bash
git diff --check origin/main...HEAD
git diff --name-only origin/main...HEAD
git grep -n '08-11-upstream-sync-pr-resolution' HEAD -- \
  .trellis/tasks/08-10-github-actions-governance/delivery.md \
  .trellis/tasks/08-10-github-actions-governance/execution.md \
  .trellis/tasks/08-10-github-actions-governance/task.json \
  .trellis/tasks/README.md
git grep -n '08-12-upstream-sync-pr-resolution' HEAD -- \
  .trellis/tasks/08-10-github-actions-governance/delivery.md \
  .trellis/tasks/08-10-github-actions-governance/execution.md \
  .trellis/tasks/08-10-github-actions-governance/task.json \
  .trellis/tasks/README.md
```

预期：改动仍只限 08-10 的记录和任务索引；第一条 grep 无输出；第二条只有纯文本
任务 ID，绝不构成相对目录链接。推送后等待 PR #115 **最新完整 head** 的 required
`ci-gate` 与 `pr-title` 成功，再更新执行侧交付状态并暂停。

### 执行回应 - 2026-08-12

- 已将 `delivery.md`、`execution.md` 和 `task.json` 中错误的 follow-up 任务 ID 改为纯文本 `08-12-upstream-sync-pr-resolution`，并删除相对目录链接。
- 后续修复仅保留 [PR #114](https://github.com/KNaiFen/aio-coding-hub/pull/114) 链接；#108 merge commit、部分完成结论及“不继承 #114 实现、CI、验收事实”的边界不变。
- 本次提交仅处理 F-001，不改写本文件的 main 审查结论，也不改动 `delivery.md` 的 `main 验收记录` 或 `main 收尾`。

## 执行边界

执行 session 仅修复 F-001、提交、推送、等待 CI、更新其执行侧说明后暂停。不得
填写 `main 验收记录`、`main 收尾`，不得合并、归档或清理。main 将在新的冻结 head
上重新验收。
