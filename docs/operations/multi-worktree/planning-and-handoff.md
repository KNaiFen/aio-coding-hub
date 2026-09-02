# 规划与交接

本页供 main 使用，具体命令由 canonical GKD bundle 提供。

1. 读取 `PENDING.md`、代码、`.gkd/policy.json` 和适用 spec，确认用户决定、范围、非目标与 AC。
2. 在任务目录写入 requirements、plan 和 authorization；材料性变化必须重新获批。
3. 从已同步 `origin/main` 创建 clean task worktree，调用 `gkd-task bootstrap` 绑定 repository、base branch、base SHA 和 policy digest。
4. 通过 `gkd-role route` 获得 manual/explicit-auto 决策；需要自动路线时只能交给 `TrustedMainRuntimeBridge.prepare` 返回的 `gkd_executor`。
5. 由 `gkd-task` 生成唯一 offer、handoff envelope 和一次性 claim capability。offer 创建后 writer 为空，禁止直接把任务标为 `implementing`。
6. 将 bundle 生成的 envelope 交给实际 session；session 首个写操作必须是 `gkd-task claim`。

交接材料只包含任务路径、offer ID、worktree、base/planning digest、角色配置和停止条件。不要手填 JSON、路径或 writer。
