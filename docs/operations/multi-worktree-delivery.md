# 任务交付

本页是项目任务协作入口，描述任务生命周期、事实源和角色边界。

## 事实源

| 事实 | 权威来源 |
|---|---|
| 需求、范围、AC、授权 | `tasks/<task>/requirements.md`、`plan.md`、`authorization.json` |
| route、offer、claim、writer、phase、delivery、rework | 任务系统生成的 `task.json`、`offer.json` 和 receipt |
| 项目身份、base、required checks | `.gkd/policy.json` 与 adapter 文件 |
| PR、head、CI、merge | GitHub 实时状态 |

不得手写协调 JSON、复制候选 head 或把 Markdown 缓存当作机器状态。

## 生命周期

```text
requirements -> plan -> ready -> awaiting_claim -> implementing
  -> delivered -> accepted -> completed
                         \\-> rework -> awaiting_claim
```

- 在 clean main 上绑定项目 policy、origin 和 base SHA。
- 完成路由决策后再创建 offer；实际 session 领取后才进入 `implementing`。
- 执行者只处理已领取任务并生成 delivery；健康等待和终态按状态机处理。
- 独立验收者只审查暂停后的固定 head。拒绝时由可信 main 创建全新 rework offer。
- 验收通过后，只有可信 main 执行窄 merge、归档和清理。

## 角色边界

- main 负责需求、授权、路由、验收协调和终态收尾。
- executor 是登记 worktree 的唯一 writer，不能验收或合并。
- 独立验收者只读审查固定 head，并检查实时 CI、policy 和 diff。
- 本地只运行 `scripts/gkd-verify --base-sha <full-lowercase-sha>`；重型检查由 GitHub Actions 承担。
