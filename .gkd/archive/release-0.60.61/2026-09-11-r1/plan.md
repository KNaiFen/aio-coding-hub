# AIO Coding Hub 0.60.61 发版计划

- Revision: r1，2026-09-11；用户明确要求“发版”，授权本轮准备版本、PR/合并、候选构建、正式标签/Release、必要验证和任务归档清理。
- 路线: direct-main；仅版本元数据和更新日志，不改变业务实现。因主树有独有历史和其他任务脏文件，使用隔离 worktree `../release-0.60.61`，分支 `chore/release-0.60.61`。
- 基线: 远端 main `7680834aa433a5191ae31e63e23ccb587ad8450f`，已包含 PR #196 的供应商四周期独立重设功能。最新正式版与全部版本清单均为 0.60.60，按项目既有 patch 发版惯例更新为 0.60.61。

## 范围与验收

1. 将 package.json、src-tauri/tauri.conf.json、src-tauri/Cargo.toml、src-tauri/crates/aio-observer-protocol/Cargo.toml、src-tauri/crates/aio-tui/Cargo.toml 和 Cargo.lock 对应三个 workspace package 的版本同步为 0.60.61。只改根包自身版本，不更改外部依赖。
2. CHANGELOG.md 增加简短 Highlights：配置页分别重设 5h/日/周/月、周期从现在持续递推且即时生效、其他周期和历史累计保留。保留原发布历史。
3. main 局部复查并完成版本一致性与差异检查后，由一个 gkd_closeout 归档本任务 plan/review/summary，与版本变更合批推送同一个发版 PR。不得混入主树独有历史或其他任务脏文件。
4. 最终 PR head 的 contracts/frontend/rust/observer-macos/ci-gate/pr-title 全通过后，以完整 SHA 正常合并 main。main 自动 CI 检测版本变化并构建既有桌面和 TUI 候选；等待该实际 merge SHA 的必要检查、所有候选构建及 assemble-release-candidate 成功。
5. 核对该 main SHA 唯一成功且未过期的 release-candidate，创建并推送注解标签 `aio-coding-hub-v0.60.61` 指向该 SHA。按既有 release.yml 发布不可变候选产物，不本地重建或重传资产。若标签/Release 已存在，按远端事实和现有合同判断，不改写已有对象。
6. 等待标签触发的 release 工作流成功并核对正式 Release、12 项既有资产及 updater 版本/下载链接，终点为 0.60.61 正式版可下载且包含 #196。不安装应用、不操作用户当前运行实例，不更新外部 Homebrew tap。

## 环境与验证

- 复用仍适用的本机资源事实：Darwin arm64、16 GiB RAM、约 245 GB 磁盘。安装、包脚本、测试/覆盖率、编译、Rust 格式/绑定生成与签名打包全部使用标准 GitHub Actions；不启动本地服务，不绕过 guard，不下载大候选包。
- 本地只读检查：`git diff --check`、`git diff --cached --check`、Git/gh 查询、`node scripts/support-matrix.mjs validate-release-version --tag aio-coding-hub-v0.60.61`。最后一个是已读源码的零依赖短时清单检查，不生成文件。
- 功能已有独立审查与 PR #196/main CI 成功证据；本任务不重复功能实现审查或增添测试。版本改动按现有自动 CI 重新验证，候选验证由 main/release workflow 实际完成。
- 纯等待复用 gkd-ci-monitor 的 monitor-worker 材料，在同一个收尾角色中处理；240 秒轮询、21600 秒预算。记录 PR head、测试 merge SHA、实际 main merge SHA、run/attempt、唯一候选和最终 Release 事实。

## 归档与清理

- 归档为 `.gkd/archive/release-0.60.61/2026-09-11-r1/`，只保存实际 plan/review/summary，不造 execution/progress 空文件。main 不预先归档。
- 收尾获准中文提交、推送分支、创建/更新一个发版 PR、正常合并、创建并推送本次注解标签、由 workflow 正式发布；必要参数错误可修正。业务代码、依赖、CI 工作流或保护规则不得借发版改动。
- 发布和必要验证成功后，移除本任务 worktree、本地/远端 `chore/release-0.60.61` 和已归档独占活动 plan/review；保留主树 main、旧归档、其他任务差异。主树有既有分叉，不 reset/rebase/stash，也不为刷新主树覆盖未知内容。
- 合并/发布/清理后续事实保存在清理后仍可访问的本任务本地归档 summary；已版本化初始材料保持观察时点，不为回填自身结果递归提交。受阻保留材料，如实返回未完成项。

## 消融审查

仅同步既有版本入口及简短更新日志，复用原生候选提升流程；不新增发版脚本、依赖、功能或工作流。
