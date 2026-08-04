# 修复 Release 标签重复 fetch

## Goal

让 Release 工作流在注释标签触发和手动触发两种入口下，都能可靠解析远端发布标签对应的不可变提交，避免标签事件因重复写入同名本地标签而在发布前失败。

## Requirements

- 标签 push 触发时，即使 checkout 已创建同名本地标签，发布源校验也必须成功。
- 手动 `workflow_dispatch` 入口必须继续支持通过 `tag` 参数发布既有标签。
- 发布源必须来自 `origin` 上指定的精确标签，不能依赖或覆盖 checkout 留下的同名本地标签。
- 保留现有标签格式、发布提交属于 `origin/main`、版本一致性和精确 CI 候选制品校验。
- 不重新构建发布制品，不修改标签或 Release 的现有版本语义。
- 修复必须包含能复现“本地同名标签已存在”场景的自动回归验证。

## Acceptance Criteria

- [ ] 注释标签已存在于本地且与远端同名时，发布源解析仍得到远端标签指向的提交。
- [ ] 本地没有该标签时，手动触发路径仍能解析同一远端提交。
- [ ] 无效、缺失或不属于 `origin/main` 的发布标签继续被拒绝。
- [ ] 工作流语法、回归验证和适用的仓库合同检查通过。
- [ ] PR 合并前复核最新 `origin/main` 的新增代码和业务影响，合并后 `main` CI 通过。

## Notes

- 真实失败证据：Release run `30876701787` 在 `Validate release source` 执行 `git fetch origin refs/tags/...:refs/tags/...` 时返回 `would clobber existing tag`。
- 候选构建与版本内容均已通过；本任务只修复发布编排中的标签解析缺陷。
