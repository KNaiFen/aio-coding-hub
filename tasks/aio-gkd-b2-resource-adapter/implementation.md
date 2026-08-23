# AIO GKD Resource Facts Adapter Implementation

## Internal Design

在 AIO 的既有 `.gkd` adapter 层新增 project-only `resource-facts.json`。其 schema 固定为 v1，复用现有 smoke 的 canonical JSON、SHA-256 和 policy loading 机制；它不导入 GKD runtime 或生产配置。

## Execution Details

executor 更新 pin 和 resource facts，扩展 strict validator/selftest 与边界文档，运行仓库批准的 local verification，提交推送 PR，并在同一 fixed head 修复任务范围内 CI 后写入 delivery evidence。
