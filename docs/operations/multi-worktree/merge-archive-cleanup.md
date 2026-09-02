# 合并与清理

本页供 trusted main 使用。

1. 核对独立验收者对固定 head 的通过结论、实时 `ci-gate`/`pr-title` 和 `.gkd` policy/bundle digest。
2. 只在 clean、已同步的可信 main checkout 通过窄 merge 接口合并；不使用 deferred auto-merge、管理员绕过或未绑定 head 的命令。
3. 合并后完成 closeout，写入 acceptance、merge SHA、知识库和 PENDING 去向，再归档任务。
4. 仅删除已合并、clean、无人使用且归属明确的 task worktree 和分支；runtime attachment 与 session receipt 由任务系统清理。

合并、归档或清理失败时保留机器状态并报告，不重建 offer 或覆盖 receipt。
