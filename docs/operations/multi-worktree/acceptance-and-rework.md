# 验收与返工

本页供 main 与 `gkd_acceptor` 使用。

## 固定 head 验收

1. 确认 executor 已停止写入，读取 bundle delivery receipt 和任务 AC。
2. 从可信、clean、已同步的 main checkout 读取候选 worktree、PR、完整 head/base、实时 required checks 和项目 policy。
3. `gkd_acceptor` 只读审查固定 head 的 diff 与交付证据，不执行候选脚本，不接受 head 漂移或 policy/bundle 不一致。
4. 结论必须绑定同一完整 head；新提交立即使结论失效。

## 返工

- 有阻塞 finding 时保持任务为 `delivered`/`rework`，不得合并。
- trusted main 调用 `gkd-task rework`，传入 delivered head、PR snapshot、review、candidate worktree 和 runtime root。
- rework 会撤销旧 offer 并生成全新 offer；禁止复用旧 claim、activation、receipt 或手写状态。

## 通过

无阻塞 finding、固定 head、required checks 全绿且 policy 未漂移后，交由 trusted main 执行窄 merge。executor 永远不能 merge。
