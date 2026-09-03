# 进度

## 已完成

- 在 `src-tauri/crates/aio-tui/src/format.rs` 增加单次成功请求的总耗时估算函数。
- TUI 请求详情优先显示精确最终尝试速度；精确值缺失时显示 `≈<value> t/s`。
- 估算值不改变请求卡片、日志字段或任何聚合统计。
- 增加精确值优先、估算值显示和多次尝试禁止估算的测试覆盖。

## 验证

- 已运行：`git diff --check`，通过。
- 未运行：`scripts/gkd-verify --base-sha <full-lowercase-sha>`；当前 worktree 和主 checkout 均不存在该脚本。
- 未运行：Rust 测试、lint、类型检查、构建和开发服务器；仓库规则明确禁止本地运行。

## 剩余风险

- 未在完整 TUI 构建环境中执行 Rust 测试，需由云端 CI 验证编译和测试。
- 估算使用总请求耗时，可能包含下游等待，因此仅展示在详情且带 `≈`，不进入正式统计。
