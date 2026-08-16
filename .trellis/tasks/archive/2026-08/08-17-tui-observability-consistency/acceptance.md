# 验收与收尾：统一 TUI 观测语义与窄屏布局

> 仅 main 在合并或确定其他终态后填写。活动期的实时 head/CI 以 GitHub 为准；执行 session 不需要读取本文件。

## 最终结果

- 结果：完成
- 功能 PR：https://github.com/KNaiFen/aio-coding-hub/pull/157
- 被验收 head：`03d9a3cf00bc1b030ead056a4056ac62c3094481`
- 验收证据：main 对固定 head 的需求、实现、测试、规格和实时 PR diff 完成独立审查；合并后主线树与被验收 head 完全一致。
- 必需 CI：https://github.com/KNaiFen/aio-coding-hub/actions/runs/31967112361 （`contracts`、`rust`、`ci-gate`）；https://github.com/KNaiFen/aio-coding-hub/actions/runs/31967112339 （`pr-title`）；https://github.com/KNaiFen/aio-coding-hub/actions/runs/31967112370 （CodeQL）
- merge commit：`20c19d1e05b95a0939c3d925b81e7d8b704b7ec8`
- 日期：2026-08-17 04:00 CST

## 验收结论

- AC：AC1-AC12 全部通过。固定本地 runner 为 `local_ready`；最终 head 的合同、Rust 格式、Clippy/check/tests、CodeQL、`ci-gate` 和 `pr-title` 全部通过。窄屏行为由 formatter 与 `TestBackend` 回归覆盖，任务合同不要求额外运行时人工验收。
- 接受的偏移或风险：无。Observer hop 的 `ok` 为必填布尔值，活动且无状态/错误的 `ok=false` hop 按交付说明显示 `进行中`，终态显示 `失败`，与当前协议一致。
- 历史整改：无 findings 或返工轮次。
- 结论证据：`format.rs` 统一后缀保留、`provider_cross` 模型展示、route presentation、缓存与详情指标；`ui.rs` 统一语义色调并将每个可用性 bucket 拆为时间/结果两行。回归矩阵覆盖 0/1/24/31/32/80 列、skipped/sent/retry/switch、四种 bucket 状态和详情滚动。

## 长期记录

- 知识库与现行合同：功能 PR 已同步 `.trellis/spec/aio-coding-hub/cross-layer/local-observer-tui-contract.md` 与 `configured-model-routing-contract.md`；无需额外知识库修改。
- PENDING：无；`PENDING.md` 没有活动 `pending`/`planned` 条目。
- 遗留风险：无已知阻塞风险；后续真实终端体验反馈按新问题处理。

## 归档与清理

- 归档路径：`.trellis/tasks/archive/2026-08/08-17-tui-observability-consistency`
- `archive --no-commit`：通过；任务状态已更新为 `completed`，目录和 JSONL 引用已迁移。
- `validate --all`：通过，共验证 139 份已有 manifest；该结果只代表 JSONL 与引用路径校验。
- records-only PR：https://github.com/KNaiFen/aio-coding-hub/pull/159；归档提交 `59ee043c527221ad4bb394c5c4e4d68fe5505f0a`。
- worktree：`/Users/knaifen/Documents/Codex/aio-coding-hub/08-17-tui-observability-consistency`；待归档记录合并且确认 session 不再占用后清理。
- 本地/远端分支：`task/tui-observability-consistency`；待 records-only PR 合并后按实时状态清理。

阻塞任务保持活动。archive 非事务性；失败时记录实际目录、状态和恢复动作，不把 `status=completed` 当成功证据。
