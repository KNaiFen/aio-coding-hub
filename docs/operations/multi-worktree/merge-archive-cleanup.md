# 合并、归档与清理

本页只供 main 在 PR 已由固定 head 验收命令合并，或用户确定其他终态后读取。

## 合并

1. `$gkd-accept` 或 main 只从干净、已同步的可信 main checkout 运行 `task.py accept .trellis/tasks/<task> --worktree <absolute-candidate-worktree> --pr <number> --head <sha>`，同步合并已经验收的固定 head。
2. 不启用 deferred auto-merge，不使用管理员绕过，也不在合并命令中删除分支；执行 session 永远不合并。
3. main `fetch origin` 并快进本地 `main`，确认验收 head 已进入 `main`，记录真实 merge commit。
4. 长期有效的产品、架构、API、迁移和运维文档优先随功能 PR 合并；合并后才发现的事实走短小 records-only PR。

## 终态记录

从[验收与收尾模板](../templates/acceptance.md)创建 `acceptance.md`，记录最终结果、被验收的完整 head、GitHub 检查、merge commit、接受的偏移/风险、知识库与 PENDING 去向，以及清理事实。

`delivery.md` 保持执行者交付事实，不追加验收轮次或收尾日志；`findings.md` 保留不通过轮次和复验结果。这样历史不会反复进入执行者的默认上下文。

没有功能 PR 时也如实记录“无”及原因。阻塞任务保持活动，不归档。失败、放弃或部分完成只能在用户/main 已决定终态、可保留成果和剩余范围已写清后归档。

## 归档

归档前确认终态记录已提交、当前任务无人写入、工作树内容归属明确，并处理长期知识与 `PENDING.md`。然后执行：

```bash
python3 .trellis/scripts/task.py archive --no-commit <task>
python3 .trellis/scripts/task.py validate --all
```

`validate --all` 只检查已有 JSONL 的 JSON 与引用路径，不证明交付、CI、Markdown 或 archive 资格。

archive 不是事务性的：命令会先把状态写为 `completed` 并移动目录，再重写/校验上下文；后续失败可能留下已经移动的目录。非零退出后先检查实际路径、`task.json` 和 `git status`，修复当前状态，不盲目重跑。

`task.json.status=completed` 只表示 Trellis 目录已归档，不等于功能成功。把归档和终态记录作为 records-only PR 提交并合并后，main 的归档副本才成为历史事实源。

## 清理

只删除已合并、干净、无人使用且内容归属明确的登记 worktree 和分支。先确认所有 session 已关闭、PR 已合并、任务资料已在 main 归档；存在来源不明文件、未提交内容或仍被 session 使用时停止并报告。

记录 worktree 路径、本地/远端分支和实际清理结果，不依赖目录扫描猜测目标。远端分支是否自动删除以 GitHub 实况为准。
