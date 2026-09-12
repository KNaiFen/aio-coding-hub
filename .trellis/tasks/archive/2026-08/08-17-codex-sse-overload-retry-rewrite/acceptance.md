# 验收与收尾：Codex Responses 过载错误码改写

> 仅 main 在合并或确定其他终态后填写。活动期的实时 head/CI 以 GitHub 为准；执行 session 不需要读取本文件。

## 最终结果

- 结果：完成
- 功能 PR：https://github.com/KNaiFen/aio-coding-hub/pull/164
- 被验收 head：`15e9f322bd610abd3fef831419bee137ca7e9353`
- 验收证据：PR #164 的固定 head、实时合并状态和检查结果；本会话独立验收无阻塞 finding。
- 必需 CI：`ci-gate`、`pr-title`、frontend、rust、contracts、JavaScript/TypeScript CodeQL 与 Rust CodeQL 均成功（见 PR #164）。
- merge commit：`fabc7babee7230eede254127b09fa62e454f6f78`
- 日期：2026-08-17 16:23:55 CST

## 验收结论

- AC：全部通过。开关默认关闭，位于 CLI 管理 > Codex；仅对原生 Codex Responses SSE 的 `response.failed` 事件、`/response/error/code` 中精确的 `server_is_overloaded` 与 `slow_down` 改写为 `server_error`。原始用量与请求日志保留上游码，Content-Length、分帧、EOF 与超限 fail-open 均有覆盖。
- 接受的偏移或风险：目标 JSON `data` 会规范化为单行；单帧超过 1 MiB 后该流永久进入原样旁路。两者均为冻结设计，以边界正确性、内存上限和无损退化优先。
- 历史整改：无正式 findings 轮次。
- 结论证据：`CodexResponsesOverloadErrorRewriter`、`spawn_usage_sse_relay_body`、原生 Responses 路径判定以及 settings schema 61 的自动化覆盖；候选 tree 与 squash merge tree 一致。

## 长期记录

- 知识库与现行合同：功能、设置 schema 和生成绑定均已随功能 PR 合并；无需额外长期文档。
- PENDING：无关联条目，`PENDING.md` 无未解决事项。
- 遗留风险：未运行真实桌面 UI 和第三方中转人工流量；本地合同禁止该类运行时验证，GitHub frontend/Rust/contract/CodeQL 自动化均已通过。

## 归档与清理

- 归档路径：`.trellis/tasks/archive/2026-08/08-17-codex-sse-overload-retry-rewrite`。
- `archive --no-commit`：成功，任务状态已写为 `completed` 并移入 2026-08 归档目录。
- `validate --all`：成功，共校验 139 个 manifests；该结果仅证明已有 JSONL 与引用路径有效。
- records-only PR：https://github.com/KNaiFen/aio-coding-hub/pull/165；最终 head `f2e817a73ec46b41725b8aa5301436fdd8b4a25c` 已通过必需检查，并以 `f558c5db6ec8adb508c22766c5f7a5a59f332baf` 合入。
- worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-17-codex-sse-overload-retry-rewrite` 已在 records-only PR 合并后删除；删除前确认干净、执行与验收 session 已结束且内容已归档。
- 本地/远端分支：本地任务分支、规划恢复分支和 records 分支均已删除；任务与 records 远端分支由 GitHub 自动删除，规划恢复分支从未推送。
