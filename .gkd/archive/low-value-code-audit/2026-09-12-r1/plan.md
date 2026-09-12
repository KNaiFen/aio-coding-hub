# 低价值代码审计与最小清理

- Revision: r1，2026-09-12。
- 用户已要求全面审计、验证安全后直接清理、运行聚焦测试与 lint 并报告证据；本记录落实该执行授权。
- 仓库：`KNaiFen/aio-coding-hub`，远端 `origin`，审查目标 `main`。
- 基线：`147c891d99404e51a87c1fb40a7588476d19a92a`，任务分支 `refactor/low-value-code-audit`。
- 使用 sibling worktree `low-value-code-audit`；原 `main` 有既有未提交归档改动及独有历史，本任务不整合这些历史。活动记录随任务工作树保存，避免覆盖原工作树已暂存的 `.gkd/plan.md` 删除。
- 路线：main 实施、取舍和最终验证；五个只读探索代理分区审计，无执行 writer 委派。
- 交付：本任务中文提交、推送任务分支并创建草稿 PR，以现有自动 Actions 验证。终点为可审查且必要 CI 通过的草稿 PR；合并与发布未纳入。未合并成果所在工作树和分支保留；本任务临时文件在证据归档后移除。

## 目录覆盖

采用目录清单、符号引用扫描、入口检查及候选定点阅读，不宣称逐行审计全仓。

| 范围 | 已检查内容 |
| --- | --- |
| `src/pages`, `components`, `ui`, `layout`, `tray`, `styles` | 页面入口、组件与 shadcn 边界、smoke/专项测试差异、样式引用 |
| `src/services`, `query`, `hooks`, `utils`, `constants`, `app`, `config` | 服务与 IPC 边界、查询缓存、导出/调用、启动编排、孤立模块 |
| `src/plugins`, `schemas`, `test`, `__tests__`, `e2e`, `generated` | 贡献消费者、schema、测试 fixture、启动/安装流程、生成声明 |
| `src-tauri/src/app`, `commands`, `domain`, `infra` | 模块入口、状态/生命周期、IPC 注册、数据/迁移/恢复路径、死代码标记 |
| `src-tauri/src/gateway`, `shared`, 根 Rust 源文件 | 路由与协议桥、provider adapter、公共 helper、能力矩阵及相关测试 |
| `src-tauri/crates`, `tests`, `examples`, `patches` | 两个 crate、集成回归与 fixture、绑定生成入口、平台 patch 引用 |
| `src-tauri/resources`, `icons`, `capabilities`, `wix`, 构建配置 | 打包与运行时引用、平台清理及 build hook |
| `packages`, `scripts`, `.github` | SDK/API 与脚手架入口、校验/selftest、工作流触发和检查合同 |
| `docs`, `.trellis`, `.gkd`, 根文档 | 现行/历史分类、索引及合同；历史档案作代表性检查 |
| `public`, `src/assets`, `src/templates`, 根配置 | 静态资源引用、已交付模板身份、Vite/Vitest/TS/ESLint/Tailwind 配置 |

## 精确清理范围与安全依据

1. 删除 `src/hooks/useMediaQuery.ts` 及专属测试：导出仅自用/测试使用，无生产导入或顶层副作用。
2. 删除 `src/query/index.ts` 与导出存在性测试：唯一消费者是该测试；真实消费者直接导入具体 query 模块。保留 queryClient、keys、providers 行为测试。
3. 删除未导入的 `src/App.css`；删除 `globals.css` 中只有定义的 scrollbar-thick、glow-pulse 和 indicator 动画组，保留真实使用的 loading 动画。
4. 删除 `vitest.config.ts` 中指向不存在的 claudeValidationTemplates.ts 的排除项；阈值不变。
5. 删除 `gateway/proxy/provider_adapters/` 四文件及注册：ProviderCapabilities 只有模块内部引用，两个测试仅断言字段赋值和 Default。
6. 删除无调用的 `skills/update.rs::update_installed_commit`、`runtime_executor.rs::dispose_runtime_caches_for_tests`、`http_client.rs::is_proxy_enabled`。真实事务更新、宿主清理和代理读取入口保留。
7. 删除 `mcp/cli_specs.rs`、`mcp/validate.rs` 两个原样转发的 validate_cli_key；`mcp/db.rs`、`mcp/sync.rs` 直接导入 shared 同一函数。参数、返回值和错误保持一致。
8. 删除 `shared/cli_key.rs::grok_capabilities_match_first_release_scope`，保留遍历全部 14 项能力的严格矩阵测试，包含被删测试全部 11 个正向和 3 个反向断言。
9. 删除 plugin-sdk 后一项 WASM 拒绝测试；保留前一项相同分支且同时断言错误码/消息的测试。实现先按 runtime.kind 返回，ABI 版本不会进入不同分支。
10. 删除 `ProviderForGateway` 三个字段已失效的 allow(dead_code) 和阶段注释：failover_loop/prepare 已实际读取这些字段。
11. 删除 `scripts/codex-candy-prompt.txt`：唯一消费者在 ba6b7f87 已删除，当前无引用。

保留 IPC/API 短包装、平台兼容/迁移逻辑、有效 smoke/交互断言、SDK 公共别名、vendored patch、生成文件和历史记录。其他候选不为扩大删除量纳入本轮。

## 验证与成功标准

- 本机为 M4 MacBook Air、16 GB 内存、约 228 GiB 磁盘；不安装依赖、不本地运行 package scripts、Cargo、编译、全套测试或覆盖率。
- 本地仅静态引用/历史核对、`git diff --check` 及无依赖 Node 的 cloud-only、CI scope、plugin API 合同检查与对应 selftest。
- 标准公开仓库 Actions 执行现有 `pnpm lint`、SDK/脚手架类型检查与测试、根 `pnpm test:unit:coverage`、前端构建、Rust formatting/bindings、Clippy 与 workspace 测试，保留现有门槛。
- 重点从已有日志确认 queryClient/keys/providers、SDK manifest 拒绝、Rust MCP/shared CLI、skills、runtime_executor/http_client 与 protocol_bridge 测试；这些聚焦范围包含在自然 CI 中，不另建 workflow 或重复全量测试。
- `ci-gate`、`frontend`、`rust`、`observer-macos`、`contracts` 和 `pr-title` 均通过；预期检查名以 GitHub 实际源核对。固定实际 PR head，监控预算 6 小时。
- 每项删除有无引用或保留覆盖证明，最终差异不含功能变更、新抽象、依赖变化、阈值下调或原工作树内容。
- main 完成消融审查后，使用 gkd-closeout 单个角色归档 plan/review、提交/推送最终任务内容并等待该 head CI，返回可访问证据和保留对象。
