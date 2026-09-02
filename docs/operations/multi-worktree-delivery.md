# GKD 任务交付

本页是项目任务协作入口。生命周期、机器状态和权限均由已安装的 canonical GKD bundle 提供。

## 事实源

| 事实 | 权威来源 |
|---|---|
| 需求、范围、AC、授权 | `tasks/<task>/requirements.md`、`plan.md`、`authorization.json` |
| route、offer、claim、writer、phase、delivery、rework | `gkd-task` 生成的 `task.json`、`offer.json` 和 receipt |
| 项目身份、base、required checks | `.gkd/policy.json` 与 adapter 文件 |
| PR、head、CI、merge | GitHub 实时状态 |

不得手写协调 JSON、复制候选 head 或把 Markdown 缓存当作机器状态。

## 生命周期

```text
requirements -> plan -> ready -> awaiting_claim -> implementing
  -> delivered -> accepted -> completed
                         \\-> rework -> awaiting_claim
```

- `gkd-task bootstrap` 在 clean main 上绑定项目 policy、origin 和 base SHA。
- `gkd-role route` 生成六门决策；`TrustedMainRuntimeBridge.prepare` 只接受已验证 bundle 和 policy。
- manual 或 explicit-auto 路线都先由 `gkd-task` 创建 offer；实际 session 领取后才进入 `implementing`。
- `$gkd-execute` 只处理已领取任务并生成 delivery；`gkd-role wait-transition` 管理健康等待和终态。
- `gkd_acceptor` 只审查暂停后的固定 head。拒绝时由可信 main 使用 `gkd-task rework` 生成新 offer。
- 验收通过后，只有可信 main 执行窄 merge、归档和清理。

## 角色边界

- main 负责需求、授权、route、bridge、验收协调和终态收尾。
- `gkd_executor` 是登记 worktree 的唯一 writer，不能验收或合并。
- `gkd_acceptor` 独立只读审查固定 head，并检查实时 CI、policy 和 diff。
- `$gkd-local-verify` 只运行 `scripts/gkd-verify --base-sha <full-lowercase-sha>`；重型检查由 GitHub Actions 承担。
