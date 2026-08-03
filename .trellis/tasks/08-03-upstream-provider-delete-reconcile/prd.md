# 收敛 Provider 删除缓存与历史身份

## Goal

选择性移植 45691b89 的删除缓存收敛和请求时 Provider 身份展示。

## Requirements

- 删除 Provider 前取消主列表、默认路由和该 CLI 下全部 sort-mode Provider 在途查询。
- 乐观移除所有相关缓存中的 Provider ID，并在成功后统一失效，禁止迟到响应复活旧行。
- 历史 attempt 显示请求时名称快照和稳定 `Name (#ID)`，不依赖当前 Provider 是否仍存在。
- 保留 fork 的账户用量、availability、模型目录、会话复用和删除级联语义。

## Acceptance Criteria

- [ ] 主列表、默认路由和多个 sort-mode 缓存删除后都不含目标 ID。
- [ ] 删除前启动的旧查询晚到后不能恢复目标 ID。
- [ ] 排序和默认路由提交 payload 不包含已删除 ID。
- [ ] 已删除或重命名 Provider 的历史 attempt 仍显示请求时身份。
- [ ] 变更可追溯到 `45691b89c495bb011fd89f97b6953dd6e5d988ae`。

## Notes

- 不移植该提交携带的上游任务文档或不适用于 fork 的测试 fixture。
