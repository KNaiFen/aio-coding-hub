# 验收与收尾：AIO GKD Project Adapter Policy

## 最终结果

- 功能 PR：[KNaiFen/aio-coding-hub#177](https://github.com/KNaiFen/aio-coding-hub/pull/177)。
- 最终固定交付 head：`58617c03142c63cd3f4017ef1fea6b2cb0e5f97f`。
- 实际 GitHub squash merge commit：`36a4b1855e767ce89286950b355cf249918eb732`。
- canonical task：`delivered`，revision `19`；接受结果为 `accepted/merged=true`。
- 独立本地验证：`scripts/gkd-verify --base-sha b35e34245a1667e647965be58ba44654ca0ba053` 返回 `local_ready`，包含 adapter selftest/smoke、cloud-only contract、diff 与变更 Node 语法检查。
- 独立 fixed-head CI：已发布并安装的 GKD `v0.1.5` `gkd-ci-monitor` 使用完整 repository `github.com/KNaiFen/aio-coding-hub`、相对 policy `.gkd/policy.json`、PR `177`、完整 head 与 3600 秒上限返回唯一 `success`；`ci-gate` 与 `pr-title` 均为 `success`，无 finding。
- acceptance review digest：`1d106c79f3e98ca81b2273dfcfc6fff602ae4f44ae229a23791caf237d83235d`。
- candidate output bundle digest：`02e687ff5f56c71404edcc3358dc641402a041f9974a779e3d1ad6137520287e`。

## 验收结论

- B4 requirements 已满足：新增 AIO 专有 `.gkd/adapter-policy.json`，严格绑定零产物本地验证、cloud-owned 类别、GitHub-hosted runner/cache、artifact 保留期及同 SHA release promotion 事实。
- validator 与 selftest 覆盖 canonical JSON、未知字段及 verification、runner/cache、artifact、tag、candidate SHA、checksum、main ancestry、immutability 的漂移拒绝。
- `.gkd/policy.json`、`.gkd/bundle-pin.json`、`.gkd/review-adapter.json`、`.gkd/resource-facts.json`、workflows、runner 配置、GitHub settings、Secrets、产品代码、Trellis 历史、生产目录、tag、Release 与 deployment 均未被 B4 修改。
- 首轮验收的 `POLICY_PATH_UNSUPPORTED`、后续 query 失败与 repository 参数错误均作为失败事实保留；未将其伪装为成功，也未重试同一次 monitor。canonical rework 退役旧 attempt 并创建新 offer/claim 后，最终独立 monitor 才成功。

## 归档与清理

- 任务资料继续保留在 `tasks/aio-gkd-b4-project-adapter-policy/`；不运行 Trellis archive，避免改写已合并任务历史。
- 本 records-only PR 合并后，trusted main 删除 B4 candidate worktree、任务分支、runtime、installed bundle staging 与一次性临时根。
- 原始 AIO checkout 的用户未跟踪 `.trellis/tasks/08-17-gkd-workflow-remediation/` 材料不在本任务范围内，未删除、覆盖或纳入提交。
