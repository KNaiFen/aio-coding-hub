# 发布 AIO Coding Hub 0.60.58

## 目标

从已通过 PR 门禁并合入 `main` 的提交 `e7c01882` 发布补丁版本 `0.60.58`。

## 范围

- 将 `package.json`、Tauri/Cargo manifest、`Cargo.lock` 的三个 workspace 包与 `src-tauri/tauri.conf.json` 统一更新为 `0.60.58`。
- 在 `CHANGELOG.md` 记录估算 Token 速率标记位置调整和遗留工作流状态清理。
- 通过版本 PR 的自动检查后，复用该 PR 合并提交对应的成功 main CI 候选制品；推送指向实际版本 merge SHA 的 annotated tag `aio-coding-hub-v0.60.58`，由 release workflow 发布正式 Release。

## 非目标

- 不修改产品逻辑、依赖、锁定版本以外的 Cargo 内容、CI workflow、签名配置或候选制品命名。
- 不安装依赖，不在本地运行 package manager、测试、lint、类型检查、Cargo、Tauri、签名、打包或构建。
- 不覆盖已有 Release 资产，不推送远端 `main`。

## 成功标准

1. 八个版本源全部为 `0.60.58`，`scripts/support-matrix.mjs validate-release-version --tag aio-coding-hub-v0.60.58` 通过。
2. 版本 PR 的 `ci-gate`、`pr-title` 和按范围选择的自动检查成功，并以 squash merge 进入 `main`。
3. 绑定该实际 main merge SHA 的唯一成功候选包含完整发布资产，校验和和 `latest.json` 有效。
4. annotated tag 解引用后指向该 main merge SHA；release workflow 成功，正式 Release 非 draft、非 prerelease，且资产清单完整。

## 验证与授权

- 本地仅运行 `scripts/support-matrix.mjs validate-release-version --tag aio-coding-hub-v0.60.58`、相关无依赖 Node selftest、`git diff --check` 和 Git/远端只读核对。
- GitHub Actions 承担依赖安装、前端/Rust 检查、候选制品、签名与发布验证。
- 用户已授权创建 PR、合并通过门禁的 PR、推送 release tag 与发布正式 Release。

## 风险

- 版本源、PR head、main merge SHA、候选制品、tag 或资产校验任一不一致即停止发布。
- 现有 Release 同名或资产不等价时，release workflow 必须保持既有 Release 不变并失败。
