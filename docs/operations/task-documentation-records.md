# 任务记录

任务的 Markdown 记录保存用户决定、范围、AC、实施结果和长期知识；机器状态只由任务系统生成。

## 实施前

- `requirements.md`：问题、目标、范围、非目标、用户锁定决定和 PENDING 审阅。
- `plan.md`：方案版本、技术选择、执行路线、风险和验证计划。
- `authorization.json`：由任务系统生成，绑定 task、repository、base、plan digest 和允许动作。

材料性变化必须重新获批，不能用聊天内容或旧 receipt 继续执行。

## 实施中

`offer.json`、`task.json`、claim receipt 和 runtime attachment 由任务系统生成。执行者只维护 `delivery.md` 的实际结果，不缓存实时 head、CI 或 merge 状态。

## 收尾

独立验收者绑定固定 head 给出结论；trusted main 在通过后完成 merge、acceptance、归档和清理。失败、阻塞或放弃必须记录真实状态，不伪造合并或验证。

本地验证统一使用 `scripts/gkd-verify --base-sha <full-lowercase-sha>`；依赖和重型检查由 GitHub Actions 负责。
