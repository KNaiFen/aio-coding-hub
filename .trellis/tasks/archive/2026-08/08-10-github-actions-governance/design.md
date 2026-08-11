# GitHub Actions 治理设计

## 1. 触发与门禁

`ci.yml` 继续监听 `push`、`pull_request` 和 `workflow_dispatch`。自动事件的聚合 job 保持 job id `ci-gate`，并将 job `name` 依据事件显示为 `ci-gate` 或 `manual-ci-gate`；这样 Ruleset 只匹配自动门禁。手动入口先运行无 checkout 的 `manual-dispatch-guard`，只有 `refs/heads/main` 才允许下游 jobs。该 guard 在自动事件中按设计为 skipped，因此所有下游条件 job 必须显式使用 `always()` 并逐项检查直接依赖的成功结果，不能依赖 GitHub 隐式注入的 `success()`。

2026-08-11 确认的范围扩展：PR 分类器输出 `frontend_ci`、`rust_ci` 与 `shared_ci`。前端路径只运行 frontend，Rust 路径只运行 Rust；跨层生成绑定、依赖/构建配置、CI 控制面、未知路径以及分类错误均运行两端。`dev`/`main` push 和 `workflow_dispatch` 强制两端运行，保证候选制品与主干集成验证不因路径优化被削弱。`ci-gate` 必须根据输出分别断言成功或预期 skipped，不能把一端的预期 skipped 当作失败。

PR 标题检查迁移到独立的 `pr-title.yml`。它不 checkout、不执行仓库代码，只读取事件 payload；`edited` 只触发该工作流，避免标题改动重新启动完整 CI。

手动 `ci` 仍支持现有 `build_release_candidate` 输入。`workflow_dispatch` 的 scope 结果保持 full CI，但把 benchmark 输出置为 false；PR 使用分域输出，`dev`/`main` push 强制 full CI。`performance.yml` 只承接显式手动 benchmark，不参与 required gate。

## 2. 上游同步授权

`sync-upstream.yml` 顶层权限收缩到 `contents: read`。job 先检查 `SYNC_UPSTREAM_APP_ID`（Actions variable）和 `SYNC_UPSTREAM_APP_PRIVATE_KEY`（Actions secret）非空且 App ID 为十进制，再用固定的 `actions/create-github-app-token@fee1f7d63c2ff003460e3d139729b119787bc349` 申请仓库限定 token，并显式请求 Contents read、Pull requests write。checkout 和所有 `gh` 调用只使用该输出 token；不保留任何 `github.token`、PAT 或旧 secret fallback，并保留 token 自动撤销。

App 只安装到本仓库，不能写 Contents，不能批准 PR。现有 no-push/no-merge/人工审核合同不变。

## 3. 安全与维护

- `.github/dependabot.yml` 覆盖根 npm、`/src-tauri` Cargo 和 GitHub Actions，每周运行。
- `codeql.yml` 使用 `github/codeql-action` v4.37.6 固定 SHA `5595ccaf912efad79be6eef63a5619ff05969be3`，扫描 `javascript-typescript` 和 `rust`；两种语言均使用受支持的 `build-mode: none`，不安装 Rust 构建依赖、不运行 Autobuild。工作流覆盖 PR、main/dev push、每周 schedule 和手动运行，权限仅 `contents: read` 与 `security-events: write`，初期不加入 `ci-gate`。
- pin policy 读取 `.github/workflows/*.yml`，允许固定 40 位远程 SHA、本地 action 和 Docker digest；拒绝 tag、branch、短 SHA、动态 ref。其 selftest 接入 `support-contract`。
- timeout 依据审计 p95 加缓冲：轻量 10m、frontend 20m、rust 60m、performance 45m、候选/dev-build 60m、assemble/release 20m、sync 10m、CodeQL 45m。

## 4. 保留边界

候选构建的 `main`/事件条件、`release-signing` 环境、artifact 完整性校验和 release 对成功 main CI 的选择逻辑必须保持。Rust 的 `CARGO_BUILD_JOBS=1`、`--test-threads=1` 和共享测试状态本期不动。
