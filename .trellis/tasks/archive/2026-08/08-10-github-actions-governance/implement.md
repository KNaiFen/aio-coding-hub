# GitHub Actions 治理实施步骤

## 施工切片

1. **规划与入口**：填写 PRD、设计、执行入口，登记 base SHA、worktree、分支和唯一写者，提交规划检查点。
2. **CI 语义**：修改 `ci.yml` 的 manual guard、动态 gate、manual benchmark 输出和 timeout；新增 `pr-title.yml`、`performance.yml`。
3. **PR 分域验证（2026-08-11 确认）**：将二元 docs/full 分类扩展为 frontend、Rust、shared/unknown；PR 按域运行，主干 push 与 main 手动保持全量；同步 `ci-gate` 的预期 skipped 语义。
4. **合同与文档**：同步 `ci-change-scope`、cloud-only/quality-gates checker 及 selftest；更新 README、AGENTS、现行 CI scope 合同和 cloud-only 合同。
5. **同步安全**：修改 `sync-upstream.yml` 和 policy checker/selftest；准备 App 权限与 secret 操作说明，不把私钥写入仓库。
6. **安全自动化**：添加 Dependabot、CodeQL、pin checker；将 pin checker 接入 support-contract；为全部既有 jobs 添加 timeout。
7. **远端设置**：代码 PR 合并且验证后，按顺序启用 SHA pinning、Dependabot/alerts/security updates，再把 Ruleset required contexts 改为 `ci-gate` + `pr-title`。App 创建/安装/私钥生成由仓库 owner 在 GitHub 设置中完成。
8. **Action 维护**：低风险 runtime action 单独升级；artifact pair、github-script、release action和 Tauri action 分别验证并可独立回滚。

## 本地验证

- `node scripts/ci-change-scope.selftest.mjs`
- `node scripts/check-cloud-only-verification.selftest.mjs`
- `node scripts/check-ci-quality-gates.selftest.mjs`
- `node scripts/check-sync-upstream-policy.selftest.mjs`
- `node scripts/check-github-actions-pin-policy.selftest.mjs`
- 对应正式 checker、`git diff --check origin/main...HEAD`、YAML/actionlint 等静态检查。

## 云端验收矩阵

- docs-only PR、full PR、标题编辑、后继提交取消旧 run。
- frontend-only PR、Rust-only PR、shared/unknown PR 与 main/dev push 的 job 矩阵分别符合预期。
- 非 main 手动 dispatch 早失败；main 手动恢复和 `build_release_candidate=true` 成功。
- 相关 Rust 路径自动 benchmark 成功；手动恢复无 benchmark；performance 手动 workflow 成功。
- 缺失 App secret 早失败；有效 App token 创建/更新人工 PR且不 push/merge。
- CodeQL 两语言运行；Dependabot PR/告警出现；pin setting 开启后所有工作流仍可启动。
- 候选 artifact 下载/assemble/release no-op 验证成功。
