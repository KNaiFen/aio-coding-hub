# GitHub Actions 治理与远端配置

本文记录当前 GitHub Actions 的触发边界、最小权限和仓库设置。工作流与机器检查器是最终事实源；远端设置变更应在对应工作流已经合并并成功运行后执行。

## 工作流职责

| 工作流 | 触发 | 门禁角色 |
| --- | --- | --- |
| `ci.yml` | `dev`/`main` push、面向两分支的 PR、`main` 手动运行 | 自动运行报告 required `ci-gate`；手动运行报告 `manual-ci-gate` |
| `pr-title.yml` | PR opened、edited、reopened、synchronize | 独立 required `pr-title`，不 checkout PR 代码 |
| `performance.yml` | 仅手动，且只允许 `main` | 非 required 的 Provider trend release benchmark |
| `codeql.yml` | PR、push、每周计划、手动 | 非 required 的 JS/TS 与 Rust 代码扫描；两者均使用 `build-mode: none` |
| `dev-build.yml` | 手动 | 按需生成未签名桌面集成制品 |
| `release.yml` | 发布标签或手动指定既有标签 | 从成功的 main CI 候选制品发布，不重新构建 |
| `sync-upstream.yml` | 每日计划或手动 | 使用 GitHub App 创建或更新人工审核 PR，不 push 或 merge |

普通 PR 只等待自动触发的 `ci-gate` 与 `pr-title`。不要对同一 PR commit 再启动 `ci` 的 `workflow_dispatch`；手动 CI 只用于 `main` 恢复或候选构建。

PR 的 `change-scope` 按 `.github/ci-scope.json` 分别输出 frontend、Rust 与 shared 选择：纯前端源码/样式只运行 frontend，纯 Rust/Cargo 路径只运行 Rust；生成绑定、根依赖、CI/工具脚本、未知路径及前后端混合改动运行两端。纯文档 PR 和 `dev`/`main` push 保留文档分类：`.gkd/` Markdown 与既有根级 plan/progress/review 只运行过程门禁，README、AGENTS 和 active spec 还运行文档合同。包含代码或未知文件的主干 push、main 手动运行仍运行两端。`ci-gate` 验证选中 job 成功、未选 job 为 `skipped`；文档优化不会跳过整个 required workflow。CodeQL 继续按独立触发规则运行。

`manual-dispatch-guard` 在自动事件中按设计跳过。依赖它的条件 job 必须用 `always()` 解除 skipped 祖先的传播，并显式检查各直接依赖的 `result == 'success'`；否则 PR 与 push 的重任务会在分类成功后仍被 GitHub 跳过。CodeQL 的 Rust 分支不安装系统依赖或 Rust 工具链，也不调用 Autobuild，因为当前 Rust extractor 只支持 no-build 模式。

## 提交与发版

1. 生命周期、路线和授权遵循 `$gkd-main`，[AGENTS.md](../../AGENTS.md) 补充项目约束。main 先维护并取得 `.gkd/plan.md` 的执行批准，再按 GKD 选择 direct-main 或 delegated。实施均从已更新的 `origin/main` 建立任务分支；仅 delegated 在独立 worktree 生成 `.gkd/execution.md` 并记录 `.gkd/progress.md`。main 按 GKD 维护 `.gkd/review.md`；任务记录不充当构建或发布输入。
2. 每个完成的任务提交简短中文 Conventional Commit。提交前执行获批方案中的零依赖、无产物检查；合并前通过任务分支 PR 等待自动分类器选中的 job 和 required `ci-gate`、`pr-title`，本地结果不替代云端质量门。PR squash 后以远端实际 merge SHA 为集成事实，同步远端结果，不再把原任务分支重复合入本地 main。本地 main 有独有提交时先比较文件差异、保留现场，不能按 ahead 数量推断遗漏功能或自动重置历史。
3. 版本 PR 合并后，等待该实际 main merge SHA 的成功 CI 与签名候选。候选完成前不推送发布标签；有候选时不额外手动重建，以免产生同 SHA 多候选歧义。main 后续有新提交也不能将标签移到新 HEAD。
4. 对该 SHA 推送 `aio-coding-hub-vMAJOR.MINOR.PATCH` 标签后，release 只晋升对应成功 main CI 的精确候选，不重新构建；同名 Release 的资产相同则无操作，不同则失败。手动 CI/发布仅用于明确恢复场景，候选缺失或过期时先确认目标提交与恢复方式，不用当前 main 替代旧发布来源。

独立验收、CI 监控、归档和现场清理由 GKD 路由；归档只保存 `.gkd/archive/` 下的脱敏 Markdown 事实。提交、推送、合并和发版按已有任务授权执行，材料性变化交回 main 确认；历史根级记录与 Trellis 归档不作为新任务入口。

## Upstream Sync GitHub App

在仓库所有者账户下创建专用 GitHub App，并按以下边界配置：

1. Repository permissions 只授予 `Contents: Read-only` 与 `Pull requests: Read and write`。
2. App 只安装到 `KNaiFen/aio-coding-hub`，不要安装到所有仓库，不授予组织或账户级权限。
3. 在仓库 Actions variables 中创建 `SYNC_UPSTREAM_APP_ID`，值为十进制 App ID。
4. 为 App 生成私钥，并将完整 PEM 保存为仓库 Actions secret `SYNC_UPSTREAM_APP_PRIVATE_KEY`。私钥不得写入仓库、日志或任务文档。
5. 保持仓库默认 `GITHUB_TOKEN` 为只读，并保持“允许 Actions 创建或批准 PR”关闭。

工作流先验证变量和私钥非空，再生成仅限本仓库的短期 installation token。checkout 与 `gh` 都只使用该输出；没有 `github.token` 或 PAT 回退，job 结束时 token 自动撤销。

同步策略检查器只接受当前批准的六个有序步骤，并把 App token 的消费者限定为 checkout 与 `GH_TOKEN`。已有同步 PR 时，只能执行一次以当前仓库、目标 head 分支、目标 base 与 open 状态限定的 `gh pr list` 查询，并将结果上限显式设为 1000，在同一 JSON 响应中按上游仓库 owner 精确选择编号；不能把 `owner:branch` 直接传给 `--head`，否则 GitHub CLI 不会匹配跨仓 PR。新建 PR 时必须捕获 `gh pr create` 的 stdout，严格接受当前仓库的 `https://github.com/${GITHUB_REPOSITORY}/pull/<正整数>` URL 并直接取出编号，不能再 list、猜测或重试。URL、编号异常、空 merge state 或命令失败均 fail-closed，且不得 push、merge 或自动批准。凭据预检、fetch 和 PR 创建脚本是完整正文合同；调整这些步骤时必须同时更新 policy selftest，不能用附加 action 或间接 Shell 调用扩展写权限。

配置后手动运行一次 `Sync Upstream`：无漂移应成功 no-op；存在漂移时应创建或更新跨仓 PR 并要求人工审核。PR 已成功创建或更新后，`DIRTY` 以 warning 提醒处理冲突，`UNKNOWN` 以 warning 表示 GitHub 尚未算出可合并性，两者都成功结束并在摘要中提供 PR 链接；成功只代表同步 PR 准备完成，不代表可合并。空状态仍失败，认证、API 或创建/更新失败不被吞掉。缺失凭据会在 checkout 前失败；App 未安装或权限不足会在 token 生成阶段失败。轮换私钥时先更新 secret，再撤销旧私钥并复验工作流。

## 合并后仓库设置

按以下顺序修改远端设置，每一步完成后确认工作流仍能启动：

1. 开启 Actions 的“Require actions to be pinned to a full-length commit SHA”。仓库内 `scripts/check-github-actions-pin-policy.mjs` 同时检查所有工作流、本地 composite action 和 job timeout。
2. 启用 Dependency graph、Dependabot alerts 与 Dependabot security updates；再启用 automated security fixes。`.github/dependabot.yml` 每周检查根 npm workspace、`/src-tauri` Cargo workspace 和 GitHub Actions。
3. 等 `pr-title` 在实际 PR 上至少成功一次后，将 main Ruleset 的 required contexts 设置为 `ci-gate` 和 `pr-title`。不要加入 `manual-ci-gate`。
4. 保持 CodeQL 初期为非 required；确认 JS/TS 与 Rust 两个 matrix 分支稳定后，再单独评估是否提升为合并门禁。

不要同时启用 selected-actions allowlist。先观察 Dependabot 与新 Action runtime 的运行结果，再以独立变更建立精确 allowlist，避免阻断发布链路。

为保持检查器无依赖且避免 YAML 等价语法绕过，工作流和本地 composite action 使用两空格 block YAML，mapping key 与冒号之间不得留空格。pin policy 会 fail-closed 拒绝 anchor、alias、merge key、复杂或引用的 block mapping key，以及无法可靠审计的 flow-style `jobs`、`runs` 或 `steps`；单行 flow-style step mapping 的 `uses` 仍会被解析并校验。

## Action 维护与回滚

低风险 runtime Action 可以按家族统一升级，但必须保持完整 SHA 与精确版本注释。artifact upload/download 必须成对升级；`github-script`、GitHub Release 和 Tauri Action 必须分别验证并可独立回滚，因为它们影响候选制品布局或不可逆发布行为。

任何升级失败时按 Action 家族恢复上一组固定 SHA。发布或候选制品链路出现清单、digest、文件名或 updater metadata 差异时停止发布，不覆盖既有 Release 资产。
