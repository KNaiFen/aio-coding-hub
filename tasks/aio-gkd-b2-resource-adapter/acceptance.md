# 验收与收尾：AIO GKD Resource Facts Adapter

## 最终结果

- 功能 PR：[KNaiFen/aio-coding-hub#172](https://github.com/KNaiFen/aio-coding-hub/pull/172)。
- 被审查与 CI 绑定的交付 head：`e7f3e1015e5f57d430ece6ac9808395947ea1f95`。
- 实际 GitHub squash merge commit：`6d75beac7d6807c7d16c213c6b96b480c2885582`。
- 独立本地验证：`scripts/gkd-verify --base-sha 492e5e2dffc788753223eec5a459b46b49765165` 返回 `local_ready`，adapter smoke 已执行，digest 为 `eac007446f5ce616aad866185b66da59a1fc5c74b32de21c0dffe117ed0443b6`。
- fixed-head CI：已安装 `gkd-ci-monitor` 对 PR #172 与该完整 head 返回 `success`；policy digest 为 `5fd82f880eb4f558142947651bbb6a35109c3bfbf151a11425ce3b19bc1c4317`，`ci-gate` 与 `pr-title` 均为 `success`。

## 验收结论

- 功能范围满足 B2 requirements：bundle pin 精确升级至已发布 GKD `v0.1.4`；`resource-facts.json` 仅绑定公开 policy/runner 来源，并将容量与账单保持为未验证 `unknown`；adapter smoke/selftest 拒绝非 canonical 数据、未知字段、runner 来源冒充、policy 漂移和把未知资源标为已验证的输入；文档明确该文件不是通用 GKD schema、实时扫描或账单事实。
- `.gkd/policy.json`、`.gkd/review-adapter.json`、workflow、runner、required checks、GitHub settings、产品代码与 Trellis 历史未改变。
- 本次不能声称 canonical GKD acceptance 成功。无写入 acceptance preflight 已通过，但正式 `gkd-task accept --merge` 的一次性 GitHub adapter 在 GitHub 已完成 squash merge 后把 REST `merged: true`、`state: closed` 错映射为 closed，退出为 `GITHUB_ADAPTER_FAILED`。task core 因此没有返回 accepted/merged receipt。
- 未为该异常补造 claim、delivery、activation、acceptance 或 receipt；实际 GitHub merge、独立验证和 fixed-head CI 是可复核事实，canonical task lifecycle 仍停留在 delivered。

## 长期记录

- 后续任何自动 merge 前，trusted acceptance adapter 必须将 GitHub REST 的 `merged: true` 归一化为 GKD contract 的 `state: merged`，并对 snapshot 输出使用 bundle canonical JSON（包括尾随换行）。
- 该修复需要在后续 GKD 流程整改任务中作为受审查、可复用的受信集成处理；本 records-only PR 不修改 GKD bundle、任务状态或已合并的功能范围。
- PENDING：在启动后续自动 merge 任务前，验证整改后的 acceptance adapter 可完整返回 `{\"status\":\"merged\",\"mergedHead\":<fixed head>}`，且只通过正式 acceptance 路径调用 merge。

## 归档与清理

- 归档路径：任务资料继续保留在 `tasks/aio-gkd-b2-resource-adapter/`；未运行 Trellis archive。
- records-only PR：本 PR。
- 候选 worktree、任务分支和一次性 runtime 仅在本 records-only PR 合并后清理；原始 AIO checkout 的未跟踪 `.trellis/` 材料不在本任务范围内。
