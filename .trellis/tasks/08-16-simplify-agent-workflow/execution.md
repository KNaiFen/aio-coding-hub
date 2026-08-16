# 施工入口：精简多 worktree 多 session 工作流

## 权威材料

按顺序读取同目录的 `prd.md`、`design.md`、`implement.md`。本文件只补充本次施工定位，不复制其中内容。

## 开工状态

- 实施授权：已确认（2026-08-16）
- 当前唯一写者：当前 Codex session
- PR base：`main`
- 任务分支、worktree、完整 base SHA、规划提交：由规划提交后运行 `task.py delegate` 写入 `task.json`；执行时以 `task.py status` 为准。
- PENDING：当前无未解决条目。
- 材料性未决问题：无。

## 允许修改

- `AGENTS.md`、`.gitignore`
- 用户级 `~/.codex/skills/gkd-*`
- `.trellis/config.yaml`、`.trellis/workflow.md`、`.trellis/scripts/**`、本任务目录
- `docs/README.md`、`docs/operations/**`、`.trellis/tasks/README.md`
- 为本任务必要的无依赖合同或 CI 调用点

不得修改产品功能、依赖、生成物、发布版本、数据库或远端 `main`。

## Preflight

在首次写入任务 worktree 前运行：

```bash
python3 .trellis/scripts/task.py status .trellis/tasks/08-16-simplify-agent-workflow
python3 .trellis/scripts/task.py doctor .trellis/tasks/08-16-simplify-agent-workflow
```

若新命令尚未实现，先按 `task.json` 登记值人工核对 `pwd -P`、branch、完整 base SHA、规划提交和 merge-base；实现后必须回到上述 canonical preflight。

## 完成信号

- `implement.md` 九项全部完成并有真实验证结果。
- 文档与脚本能力一致，旧命令和死状态不再出现在现行指导中。
- `delivery.md` 只写实际实现、验证、偏移和风险，不手写可由实时 Git/CLI 得出的候选状态。
- 变更提交到任务分支并形成面向 `main` 的 PR 后暂停，由 main 做最终固定 head 验收与合并。

## 停止条件

触及 `prd.md` 明确排除项、出现来源不明修改、需要改变用户锁定决定，或 canonical preflight 失败时停止并报告。
