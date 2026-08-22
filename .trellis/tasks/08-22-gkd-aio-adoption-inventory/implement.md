# 实施：AIO GKD 接入遗留盘点与迁移映射

1. 从固定 `origin/main` base 审阅 AIO 的角色入口、任务协调、验收、本地验证、CI/release 和历史任务材料。
2. 把每项归入 bundle 替代、AIO policy/adapter、证据后删除或历史只读，并记录文件级映射。
3. 在 `prd.md` 记录用户授权、已审阅的 PENDING、范围、非目标、停止条件和 AC。
4. 在 `design.md` 固定 bundle/adapter 分层和五个后续里程碑的安全门。
5. 仅提交本任务材料，使用登记 base 运行仓库允许的本地验证合同，并在固定 PR head 上接受独立验收。

## 后续里程碑完成门

| Milestone | Required proof before proceeding |
|---|---|
| B: bundle pin and adapters | Released asset SHA-256、bundle digest、repo/policy/origin agreement and adapter smoke |
| C: state/history migration | Active and archived fixtures are idempotent; missing/deleted worktrees fail or remain historical as designed |
| D: CI and release integration | Air-safe local micro contract, fixed-head CI result, project required checks and release-policy tests |
| E: canary and deletion | Manual canary evidence, complete automatic-route gates under the recorded authorization, independent acceptance and a clean legacy-removal diff |

## Stop Conditions

- Pinned release asset, output bundle digest, AIO origin or required-check policy differs from recorded facts.
- A step would require a production install, paid runner, Secret, unapproved GitHub setting, tag/Release or GKD canonical source change.
- A state migration cannot use a supported command or would require making historical worktrees live again.
