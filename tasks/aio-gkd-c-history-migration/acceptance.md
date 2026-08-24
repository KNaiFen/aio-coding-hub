# AIO GKD Historical Task Adapter Acceptance

## 结果

- 结果：完成。
- 功能 PR：[KNaiFen/aio-coding-hub#179](https://github.com/KNaiFen/aio-coding-hub/pull/179)。
- 固定交付 head：`0d91548c1990dd059ba6df0f6465e2c275714f36`。
- canonical task：`delivered`，revision `14`；epoch `2`，包含两次独立拒绝返工记录。
- 实际 GitHub squash merge commit：`378fb51569a904573c8a95a9e0b5b51e6820de88`，2026-08-24。

## 独立验收

- 最终 acceptor 审查固定 head，无 finding；review digest：`308d3b7cf90b64e109e237f712ad529ebacd04db3a74b454eb7e897dd5572560`。
- 独立 `scripts/gkd-verify --base-sha 3f856c88749f4875889164fa72caeebc22143d98` 返回 `local_ready`；history smoke 返回 `active_count=1`、`archived_count=107`，变更 Node 文件语法、diff 和 cloud-only contract 均通过。
- 最终 acceptor 使用已安装 GKD `v0.1.5` 的 `gkd-ci-monitor` 恰好调用一次，完整 repository `github.com/KNaiFen/aio-coding-hub`、相对 policy `.gkd/policy.json`、PR `179`、fixed head、3600 秒上限和 30 秒轮询均绑定；终态为 `success`，`ci-gate` 与 `pr-title` 均成功，policy digest 为 `5fd82f880eb4f558142947651bbb6a35109c3bfbf151a11425ce3b19bc1c4317`。
- canonical `gkd-task accept --actor-role main --merge` 返回 `accepted`、`merged=true`；没有使用直接 `gh merge` 或 auto-merge。

## 验收结论

- `.gkd/history-adapter.json`、tracked-only checker、active/archive fail-closed 规则、真实 Git 删除触发 selftest 和非 manifest 不触发 selftest 均满足 requirements。
- 归档历史不被重写；archive 的旧 `worktree_path` 只作为历史事实被忽略，不作路径解析、文件系统访问或输出。
- base `3f856c88749f4875889164fa72caeebc22143d98` 到 fixed head 的 `.trellis/tasks/**` 路径与内容无差异；原始 AIO checkout 未跟踪的 `.trellis/tasks/08-17-gkd-workflow-remediation/` 未触碰。
- 两次独立 HIGH finding 已通过 canonical rework 记录并修复：manifest-only 变更触发 history smoke；删除 active/archive `task.json` 也触发 history smoke。
- `.gkd/policy.json`、bundle pin、review/resource/adapter policy、workflows、GitHub settings、Secrets、runner、产品代码、发布和部署均未修改。

## 交付与收尾

- implementation head：`b913fccbc877e5ac482869c745c98959f481de83`。
- delivery document commit：`8697c74e8b365e93e839ceade095197b9035591e`。
- delivery document digest：`c601a51c389e000ccd74f3df1cfbc4fe81b47e4b02e52d568a9e7a3309578c15`。
- candidate output bundle digest：`7121abfa2d7bb8eacd29c825a20409990f74eb9a4fe15c64109f22d0f71c90b0`。
- 首次 acceptance 调用曾返回 `FILESYSTEM_ERROR` 且未合并；固定 head、review 和独立 monitor 事实未变，trusted main 随后仅通过同一 canonical acceptance path 完成成功合并；没有补造 receipt。
- 本文件是 records-only closeout；提交后由 trusted main 运行一次 3600 秒 fixed-head monitor 并在成功后合并，随后删除 closeout worktree/branch。

## 后续边界

- 里程碑 C 已完成；下一项按 adoption plan 进入里程碑 D CI/release 接入。
- C 不声称已实现 D 的资源优化、release candidate、checksum 或部署接入；这些只能在新的 v0.1.5 隔离 runtime、独立 task route、delivery、acceptance 和 closeout 中继续。
