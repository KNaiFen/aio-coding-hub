# GKD 验收整改：{{任务名称}}

- 被验收 head：`{{完整 SHA}}`
- finding ID：`{{稳定 ID}}`
- 严重度：{{阻塞 / 建议}}
- 证据：{{固定 head 中的文件、行号或复现}}
- 必须达到：{{可判定结果}}
- 修改边界：{{允许触碰与明确禁止}}
- 复验方式：{{固定 head、CI、policy 和 AC}}

阻塞 finding 只能由 trusted main 调用 `gkd-task rework` 生成新 offer；禁止复用旧 claim、activation 或 receipt。
