# 设计：轻量协调状态与按需工作流

## 事实分层

| 层级 | 权威内容 | 生成/维护方式 |
|---|---|---|
| Git/GitHub | commit、branch、PR head、CI、merge | 实时读取，不复制为长期缓存 |
| `task.json` | route、phase、writer、登记 worktree/base/planning commit、blocker | `task.py` 专用 writer |
| 任务 Markdown | 用户决定、范围、AC、设计、实际实现、风险 | 人类/模型按职责维护 |
| session runtime | 当前窗口选择哪个任务 | 可丢弃缓存，可从显式 task 路径恢复 |

## Coordination v1

在现有 `task.json` 增加版本化对象，不复制已有 `branch`、`base_branch`、`worktree_path`、`commit`、`pr_url`：

```json
{
  "coordination": {
    "version": 1,
    "route": "main",
    "phase": "planning",
    "writer": "main",
    "base_sha": null,
    "planning_commit": null,
    "block": null,
    "updated_at": "2026-08-16T00:00:00Z"
  }
}
```

- `route`: `main` 或 `delegated`。
- `phase`: `planning`、`ready`、`implementing`、`blocked`、`delivered`、`completed`。
- `writer`: 协调身份字符串，不承担认证功能。
- `block`: `null` 或 `{reason, resume_condition, owner, blocked_at, previous_phase, previous_writer}`；阻塞时 writer 转给 owner，恢复时显式交给新 writer。
- 旧任务没有 `coordination` 时只读兼容，不做批量迁移；只有新命令修改活动任务时才升级。

所有写入保留未知字段、使用原子替换并在失败时返回非零。`doctor` 只严格校验 v1 delegated 任务，不追溯拒绝历史短 SHA 或相对路径。

## CLI 边界

- `status [task] [--json]`：稳定读取持久状态；session pointer 只用于省略 task 参数时的选择。
- `doctor [task]`：检查 manifest、canonical cwd、branch、worktree、完整 SHA、planning commit 和 merge-base；不查询 GitHub。
- `delegate <task> ...`：登记已由 main 创建的 worktree/branch/base/planning commit/writer，校验后写入。
- `handoff [task]`：先 doctor，再从 canonical state 生成固定交接清单和可粘贴 Prompt；不写第二份状态文件。
- `deliver`：只允许干净的 delegated implementing worktree 在 `delivery.md` 已提交后转为 `delivered`，并把 writer 固定交给 `main`。
- `block/resume`：显式记录或清除阻塞，拒绝非法转换。

`accept` 只能从干净且已同步的可信 main checkout 运行。它读取显式候选 worktree 中的活动任务 manifest，但不导入或执行候选分支代码；两次核对本地与 GitHub 实况后，通过带固定 head `sha` 的 REST merge endpoint 同步 squash merge。合并请求结果不确定时，只有 GitHub 确认同一 head 已合并才幂等成功。

worktree 创建和删除仍由生命周期所有者显式执行 Git 命令，本轮不封装自动清理，避免把高风险操作藏入一个新命令。

## 文档与 skills

```text
AGENTS.md
  -> role skill
     -> 当前阶段专题
        -> 当前任务 Markdown / task.py
```

三个 skills 是独立新窗口和 main subagent 的角色入口，不是安全边界。硬约束仍由 Git、sandbox、CLI 和可机械判断的脚本承担。

多 worktree 文档保持一层直链：主入口直接链接 planning、execution、acceptance、cleanup 四份专题和 execution、delivery、findings、acceptance 四个模板。模板只存实例数据；通用说明回到对应专题。

## 兼容性与迁移

- 保留现有 `status=planning/in_progress/completed` 顶层语义，breadcrumb 继续兼容。
- `start` 在 v1 存在时同步 `ready -> implementing`；返工以 `--writer` 同步 `delivered -> implementing`；archive 同步 `completed`。
- 旧 `implement.jsonl/check.jsonl` 暂不迁移；workflow 不再把 seed-only validate 描述为 ready gate。
- 归档历史保持只读；新模板只影响新任务和后续轮次。
