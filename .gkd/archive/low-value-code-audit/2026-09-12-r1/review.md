# 低价值代码清理审查

- 当前结论：PLAN r1 范围内静态审查通过；尚未提交，最终 GitHub Actions 待定，不宣称测试/lint 已通过。
- Revision: r1，2026-09-12；基线 `147c891d99404e51a87c1fb40a7588476d19a92a`。
- 审查对象：本任务工作树相对基线的 23 个代码/测试/配置文件差异；main 是唯一实现 writer，已停止实现写入。

## 删除证明

| 删除项 | 已核实证据与保留覆盖 |
| --- | --- |
| useMediaQuery 模块及测试 | 全仓符号引用仅定义及专属测试；无生产导入、无顶层副作用。Tailwind 与全局 matchMedia fixture 保留 |
| query barrel 及存在性测试 | 相对/alias/index 导入搜索仅有被删 index.test.ts；queryClient.test.ts 断言配置，keys.test.ts 断言键值，providers.test.tsx 断言真实 query/mutation |
| App.css、旧全局样式 | App.css 无导入；scrollbar-thick、glow-pulse、indicator-shimmer、indicator-grow 仅命中被删 CSS 定义；loading 在 UsageDataPanelContent 中实际引用并保留 |
| provider_adapters | ProviderCapabilities 及三 getter 的引用全在删除目录；两个测试仅检查字段/派生 Default；真实 protocol_bridge 与 provider prepare 逻辑未改 |
| 三个无调用函数 | update_installed_commit、dispose_runtime_caches_for_tests、is_proxy_enabled 全仓各仅有定义；保留技能事务 UPDATE、异步宿主清理、get_current_proxy_url 和对应行为测试 |
| MCP 两个转发 | 删除前两函数均逐字返回 shared::cli_key::validate_cli_key(cli_key)，现在 db/sync 直接 import；函数调用顺序、参数、结果类型和错误不变 |
| Grok 重复测试 | 保留 registry_capability_matrix_is_exact：EVERY_CAPABILITY 全部 14 项过滤后必须恰为前 11 项，严格覆盖被删 11 正向/3 反向断言 |
| SDK 重复测试 | 保留 test("rejects wasm as unsupported public runtime") 的完整错误对象断言；validateManifest 在 runtime.kind 分支立即返回，ABI 1/2 不进入不同路径 |
| ProviderForGateway 标记/注释 | failover_loop/mod.rs 的 cross_temporary_work_item 真实读取 UUID 和跨供应商策略，prepare/provider_iterator.rs 读取 bridge_type；只移除失效的 dead_code 抑制 |
| 旧排除项与糖果输入 | 排除项对应源文件不存在；糖果输入唯一消费者 validate-codex-experimental-continuation.ps1 在 ba6b7f87 删除，现无引用 |

## 已执行验证

以下均在本机执行且退出码为 0；没有依赖安装、package scripts、Cargo、编译、全套测试或覆盖率。

- `git diff --check`：无空白错误。
- `node scripts/check-cloud-only-verification.mjs`：repository contract passed。
- `node scripts/check-cloud-only-verification.selftest.mjs`：all assertions passed。
- `node scripts/ci-change-scope.selftest.mjs`：CI change-scope self-test passed。
- `node scripts/check-plugin-api-contract.mjs`：退出码 0。
- FastCtx 全仓符号检索、Git 导入/字段引用及历史核对支撑上表；定义级零引用判断没有扩大为对 IPC/外部 SDK 的删除许可。

## 消融审查与收尾

- 没有新增抽象、依赖、功能或测试；3 行新增仅改变 MCP 共享函数导入，原测试阈值不变。
- 未为了凑删除量改动单纯短小但承担边界职责的代码；保留 smoke 独有交互、迁移回归、公开 SDK/API、生成物、平台 patch 和历史文档。
- 唯一格式问题为删 SDK 用例留下的空行，已由 main 修正。
- 主工作树原有 GKD 归档改动、暂存删除及独有提交不纳入本任务。
- 同意 gkd-closeout 将本 plan/review 移到 `.gkd/archive/low-value-code-audit/2026-09-12-r1/`，提交清理与记录，推送任务分支、创建草稿 PR，等待固定 head 的标准 CI。
- 最终测试/lint/构建结果、实际 SHA/PR/run URL 由收尾角色基于 GitHub 事实报告；失败时保持未完成状态并返回 main。
- PR 保持 draft/open；未授权合并、发布或更新主工作树，因此保留承载未合并成果的任务工作树和本地/远端任务分支。
