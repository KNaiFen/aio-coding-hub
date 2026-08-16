# 验收与返工

本页只供 main 和 `$gkd-accept` 使用。普通探索 subagent 仍只读取证，执行 session 不需要在正常施工时读取。

## 固定候选

1. 确认执行 session 已暂停，`task.py status <task>` 为 `delivered`。
2. 读取 `prd.md`、必要设计、`execution.md`、`delivery.md` 和实时 PR diff。
3. 从 GitHub 读取完整 head/base、必需 CI、相关编译、合并状态和变更范围；不信任 Markdown 缓存。
4. 按 AC 检查成功路径、错误路径、兼容性、回归风险、测试、文档和偏移。
5. 可派普通只读 subagent 提供特定风险线索；`$gkd-accept` 或 main 抽查出处并给最终结论。

通过结论必须绑定 GitHub 上的具体 head，可使用 PR review/comment 留证。不要在候选分支内提交一个声称自己是“当前 head”的文件。任何新 push 都使旧结论失效。

验收输入必须显式给出活动任务的仓库相对路径、候选 worktree 绝对路径、PR 编号和完整 head SHA。没有阻塞 finding 时，切到干净且与最新 `origin/main` 一致的可信 main checkout 执行：

```bash
python3 .trellis/scripts/task.py accept .trellis/tasks/<task> \
  --worktree <absolute-candidate-worktree> \
  --pr <number> \
  --head <full-pr-head-sha>
```

命令自行 `fetch origin main`，拒绝过期或脏的 main、路径逃逸、归档或 symlink 任务、不同 Git 仓库、跨仓 PR、候选漂移和任何非绿色 required check。候选分支的 Python 或脚本不会被导入或执行；manifest 和必需任务文件直接从固定 `HEAD` Git tree 读取，required contexts 从实时 GitHub ruleset 读取。合并前再次 fetch 并重验本地 Git、PR、rules 和 checks，最后通过带精确 `sha` 的 GitHub REST endpoint 同步 squash merge。超时或非零后进行有限确认轮询，只有 GitHub 确认“该精确 head 已合并”才幂等成功；同样的命令可安全确认一次已完成但响应丢失的合并。不得使用 deferred auto-merge、管理员绕过或分支删除。

## 验收不通过

main 从[整改模板](../templates/findings.md)创建或更新 `findings.md`。每项 finding 使用稳定 ID，并包含严重度、对应要求、具体证据、影响、必须达到的结果、修改边界和复验方式。纯建议与阻塞问题分开。

默认由执行 session 返工。main 提交 findings、把任务恢复为执行者写入后，执行 session 只处理未解决 finding，更新 `delivery.md`，重新执行 `deliver` 并等待新 head CI。

历史轮次保留在 `findings.md`，但 `execution.md` 只链接当前未解决 ID；不要把旧轮次复制到每次执行入口。最终验收历史由 main 在终态 `acceptance.md` 汇总。

## main-direct-fix

只有以下条件全部满足时，main 才能在原任务 worktree、分支和 PR 上临时接管：

- 执行 session 已暂停，main 先持久化接管时间、冻结 head、工作树状态、未提交内容归属和 writer 转移。
- 问题单一、局部且修法确定，不需要设计、用户决定或扩大范围。
- 只修正任务记录中的事实、路径、链接、日期、拼写或格式，不改变产品行为、测试逻辑、API、兼容性、安全、迁移、架构、接口/数据流或 AC。
- 预期并由实时 `change-scope` 证明只进入 process/checked documentation，不选中 `frontend_ci`、`rust_ci` 或 `full_ci`，也不触发构建、生成、签名、打包、发布或性能任务。
- main 能在当前任务范围内独立证明结果，所有内容归属清楚。

在 `findings.md` 标记 `返工责任：main-direct-fix`，记录接管原因、范围、预期/实际 scope、选中与跳过 jobs、保持不变的行为和复验标准。推送后按新 head 重新验收。

只要涉及产品代码、测试逻辑、workflow/policy/selftest、依赖/锁文件、生成文件、公共 API、迁移、架构、跨模块、不明原因、用户再确认或长任务，就交回执行 session。若实际 scope 意外选中长任务，立即停止，持久化最后安全状态和恢复条件，再把 writer 交回执行 session。

## 终态

- 通过：确认无阻塞 finding 后，由 `$gkd-accept` 或 main 从可信 main checkout 调用 `task.py accept` 同步合并固定 head。
- 需要整改：保持任务活动，恢复为 `implementing`，不得合并。
- 阻塞：执行 `block`，保留任务和需要恢复的 worktree。
- 失败、放弃或无功能 PR 的部分完成：如实记录，不伪造功能 PR、merge 或验证。
