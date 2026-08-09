# 上游合并价值与冲突审计

> **状态：历史审计。** 本报告只描述 2026-08-03 固定基线下的上游候选与冲突；候选是否已经集成、当前 upstream 状态和现行操作规则应从 [项目知识库](../../README.md)、当前 Git 历史与 `AGENTS.md` 判断。正文中的 SHA、分支和路径按当时证据保留。

**状态：完成**

**审计日期：2026-08-03**

**审计对象：**

- 当前项目 `origin/main`（最终比较基线：`aa0a536b`）
- [FingerCaster/aio-coding-hub](https://github.com/FingerCaster/aio-coding-hub) 的最新发布与默认分支
- [dyndynjyxa/aio-coding-hub](https://github.com/dyndynjyxa/aio-coding-hub) 的最新发布与默认分支
- 当前仓库当时的健康审计，现归档为 [`codebase-health-audit-2026-08-09.md`](./codebase-health-audit-2026-08-09.md)

## 审计目的与边界

本报告回答四个可执行问题：

1. 自共同分叉点以来，哪些上游提交可低冲突、独立地合并；
2. 哪些新功能具有足够的产品价值，值得作为单独集成项目；
3. 哪些重要提交与本项目现有架构、Fork 行为或已知健康度问题存在实质冲突；
4. 合并顺序、验证方式和不能自动合并的明确原因。

本次为只读审计。除本文件外不修改产品代码、配置、锁文件或既有报告。所有 SHA、发布信息和文件冲突结论在采集后写入；未验证的推断会明确标为“待验证”。

## 比较基线

| 项目 | 基线 | 状态 |
|---|---|---|
| 当前项目 | 最终 `origin/main` = `aa0a536bb08ce1aeb8438db278b4863d2b7dffd8`；最近项目 release 为 `aio-coding-hub-v0.60.43` (`32247d3c`) | 已固定 |
| FingerCaster | 默认分支 `main` 与最新 Release `aio-coding-hub-v0.60.33` = `86368d1533df87909d660cd9d64ee236e4bf0645`（2026-07-31） | 已固定，已完成 |
| dyndynjyxa | 默认分支 `main` 与最新 Release `aio-coding-hub-v0.60.16` = `4f02ba3d6e7bee9539fb4aee3dc3a10e022726ee`（2026-07-27） | 已固定，已完成 |
| 既有风险 | `CODEBASE_HEALTH_AUDIT.md` 的 confirmed 项 | 已映射相关项 |

## 扫描进度

| 编号 | 范围 | 状态 | 证据 / 结论摘要 |
|---|---|---|---|
| S0 | 本地基线、工作树和现有健康度报告 | 完成 | 扫描起点为 `9d1fb966`；审计期间 PR #21 合并后最终基线刷新为 `aa0a536b`。工作树只有既有未跟踪材料和审计报告，未改产品文件。 |
| S1 | FingerCaster 最新发布、提交差异与候选功能 | 完成 | `v0.60.31..33` 为有效增量；从 38 个提交筛出 9 个行为候选。 |
| S2 | dyndynjyxa 最新发布、提交差异与候选功能 | 完成 | `4f02ba3` 是 FingerCaster 当前 `main` 的祖先；其 7 个新增提交已并入 S1 候选链，避免重复推荐。 |
| S3 | 候选提交冲突、依赖链与健康度风险映射 | 完成 | 已对 9 个候选逐项三方模拟，识别 22 个整支合并冲突，并映射 `AUD-013`、`AUD-028`、`AUD-047` 等。 |
| S4 | 合并批次、验证方案与未覆盖项 | 完成 | 给出安全、Provider、OAuth、Codex 投影和用量功能的独立治理批次。 |

## 结论索引

状态含义：`推荐合并` 指可以以最小手工移植进入近期批次；`条件合并` 指需要额外产品/运行时验证；`暂不合并` 指当前不应 cherry-pick 或整支合并；`待验证` 指外部行为尚未得到本项目环境证实。

| ID | 状态 | 优先级 | 来源 / 提交 | 结论摘要 |
|---|---|---|---|---|
| UPA-001 | `暂不合并` | `P1` | 两个源复用的 `aio-coding-hub-v0.60.*` tags | 同名 tag 指向不同提交，不能用 tag 名做跨上游 fetch、比较或发布来源；应始终固定 owner + SHA。 |
| UPA-002 | `推荐合并` | `P1` | `7cc1d8ac`，ChatGPT account header 隔离 | 小型安全/身份修复，直接消除 `AUD-028` 的 token 与来访 account header 混搭。 |
| UPA-003 | `推荐合并` | `P1` | `45691b89`，Provider 删除后的缓存同步 | 精确修复 `AUD-013`；需按当前 Query 缓存结构手工移植。 |
| UPA-004 | `条件合并` | `P1` | `7bd1812f`，Claude OAuth 入口与 token UA | 当前仍使用旧 authorize endpoint；先用隔离账号验证真实登录/刷新，再保留当前错误脱敏逻辑手工移植。 |
| UPA-005 | `推荐合并` | `P2` | `84564a5b`，OAuth 到期状态缓存 | 修复刷新后旧 React Query 快照覆盖 `expires_at` 的确定竞态；不等同于 `AUD-012`。 |
| UPA-006 | `暂不合并` | `P1` | `6cf0aa21` + `d7ba5735`，Codex OAuth projection/remote compaction | 设计价值很高，但投影、回滚和同步语义已与 fork 深度分叉；应专题重实现。 |
| UPA-007 | `条件合并` | `P2` | `c9326c0a`，文件夹排行与开发时长 | 当前已有文件夹筛选/日明细；开发时长估算仍是新能力，但统计口径与 ledger 冲突，需作为新功能重做。 |
| UPA-008 | `条件合并` | `P2` | `d27efdb8`，Provider 指标走势 | 有价值的新用量视图，但原实现会扩展 `AUD-047` 的无界数据点风险，不能原样移植。 |
| UPA-009 | `推荐合并` | `P3` | `de09d645`，About 页隐藏未知 Bundle/运行模式 | 两个前端文件零文本冲突的低风险体验修复。 |
| UPA-010 | `暂不合并` | `P1` | FingerCaster release 资产矩阵 | 其 Intel/Linux 正式资产与本 fork 已锁定的发布矩阵、签名和 Homebrew 策略冲突。 |

## 待验证假设

| 编号 | 假设 | 验证方式 | 状态 |
|---|---|---|---|
| HYP-U01 | 两个指定仓库的默认分支和 Release tag 可从公开 GitHub 元数据稳定解析。 | GitHub API、签名 tag 与本地只读对象交叉核对。 | 已验证 |
| HYP-U02 | `origin/main` 是用户所称“当前 main”的唯一代码基线，当前 `codex/tui-polish-release` 不应混入比较。 | 记录本地/远端 main SHA、分支图与当前工作树状态。 | 已验证 |
| HYP-U03 | `platform.claude.com/oauth/authorize` 在本 fork 的 OAuth client/账户组合下仍会失败回调。 | 用隔离 Claude 账号完成授权、token exchange、refresh 和 401 refresh 四段测试。 | 待验证 |

## 审计方法和限制

- 采用共同祖先、补丁等价性（patch-id）、逐提交 diff、调用路径和测试覆盖检查；不以提交标题或 Release notes 单独作为结论依据。
- 重点审查 gateway、身份与 OAuth、配置/持久化、插件运行时、发布工作流与前端状态管理等高风险区域。
- 指定仓库为外部公开源；发布条目先通过 GitHub 页面/API 采集，再用 Git 对象验证。
- 本地不运行 Cargo、Rust/Tauri 或打包命令，遵循仓库规则；本报告不宣称原生验证覆盖。

## 详细证据

### S0：本地基线、工作树和历史关系

- 扫描起点是 `origin/main` / 本地 `main` 的 SHA `9d1fb9664b4a783622d937a84381cdd103f7bcc2`（2026-08-03 03:18:26 +0800）。审计期间，用户说明的并行前端/TUI 会话经 PR #21 合并，GitHub 只读查询确认最终 `origin/main` 为 `aa0a536bb08ce1aeb8438db278b4863d2b7dffd8`（`feat: polish TUI and macOS tray observability`）。本报告以 `aa0a536b` 作最终比较基线，以 `9d1fb966` 保留既有健康度报告的证据时间点。
- 审计开始时已存在未跟踪目录或文件：`.impeccable/`、`.playwright-cli/`、`.trellis/workspace/KNaiFen/`、`CODEBASE_HEALTH_AUDIT.md`、`PRODUCT.md`、`upgrade-tui.command`。本审计只新增本报告，未触碰这些内容。
- 当前 `main` 的最近正式项目 tag 是 `aio-coding-hub-v0.60.43`（`32247d3c`）；`aa0a536b` 在其后合并了 release/journal 文档与 PR #21 的 TUI/托盘可观测性改动。因而当前产品版本仍为 `0.60.43`，本报告的代码比较使用 `aa0a536b`。
- GitHub API 显示 FingerCaster 是 `dyndynjyxa/aio-coding-hub` 的 fork；两个指定源的默认分支均为 `main`。因此不能把 FingerCaster 的 `0.60.33` 与 dyndynjyxa 的 `0.60.16` 当作并行版本序列；本报告将优先以祖先关系、补丁等价性和实际 diff 判断。
- FingerCaster 的最新 Release `v0.60.33` 说明仅包含 `45691b8` 的 provider 删除/日志尝试修复；`v0.60.32` 还包含远程 compaction 同步与 OAuth projection 稳定化。dyndynjyxa `v0.60.16` 则包含用量趋势/文件夹排行，以及 Claude OAuth、ChatGPT 账户头和 OAuth 到期展示修复。这些仅作为候选入口，尚未因 Release notes 被直接推荐。
- 既有健康度报告的 51 项 confirmed 问题作为合并过滤器：上游提交只有在确实缓解一个已知根因，或带来独立产品价值且不加剧 P1/P2 风险时才列入推荐项。
- PR #21 相对扫描起点变更 35 个路径（TUI、tray、resident、provider availability、入口/样式、Cargo 元数据及 Trellis/PENDING 文档）。对 FingerCaster 候选涉及的 `gateway/**`、`query/providers.ts`、`query/sortModes.ts`、`pages/providers/**`、`pages/usage/**`、`components/home/**`、`domain/usage_stats/**`、`infra/codex*` 与 `app/settings_service.rs` 做路径交集检查，结果为空。因此本报告对这些候选做出的三方合并冲突结果不因 PR #21 失效；最终仍以 `aa0a536b` 的对象读取作为证据。

### S2：dyndynjyxa 最新发布的去重结论

- `dyndynjyxa/main` / `aio-coding-hub-v0.60.16` 是 `4f02ba3d6e7bee9539fb4aee3dc3a10e022726ee`。它与 FingerCaster 当前 `main` 的共同祖先就是自身；`FingerCaster/main...dyndynjyxa/main` 的提交计数为 `316 0`。这证明 dyndynjyxa 最新发布已被 FingerCaster 全量包含。
- 当前项目与 dyndynjyxa 的共同祖先是 `419086fb`（dyndynjyxa `v0.60.15`），计数为当前项目 433 个提交、dyndynjyxa 7 个提交。那 7 个提交分别是：`de09d64`（About 页面显示）、`7bd1812`（Claude OAuth）、`c9326c0`（文件夹用量/开发时间）、`84564a5`（OAuth 到期展示）、`7cc1d8a`（ChatGPT 账户头）、`d27efdb`（Provider 指标走势）和 release commit `4f02ba3`。
- 因为这 6 个行为提交同时也是 FingerCaster 从 `v0.60.30` 之后的历史祖先，所有是否合并、冲突与验证结论均在 S1 统一给出；本报告不会把同一补丁推荐两次。
- **UPA-001 证据：** FingerCaster 的 `aio-coding-hub-v0.60.16` 指向 `bd3d03b1`，而 dyndynjyxa 的同名 tag 指向 `4f02ba3d`。在以 FingerCaster 镜像为基础的临时只读对象库拉取 dyndynjyxa tags 时，Git 对 `v0.60.3`、`v0.60.6` 至 `v0.60.16` 等同名 ref 报 `would clobber existing tag`；目标 `main` ref 仍可被拉取。该冲突不是文本合并冲突，而是来源身份/发布可追溯性冲突。任何后续集成必须使用完整仓库名与 peeled commit SHA，不能使用裸 tag 名或 `--tags` 同步。

### S1：FingerCaster `v0.60.31..33` 提交筛选

#### 历史与等价性证据

- 最终 `origin/main@aa0a536b` 与 FingerCaster `main@86368d1` 的共同祖先是 `1a551cbee35960fbb954e475a13b2d8d55d709df`，即 FingerCaster `aio-coding-hub-v0.60.30`。
- 分叉后提交计数为本 fork 148、FingerCaster 38；去掉 5 个 merge/release 整合提交后，FingerCaster 有 33 个非 merge 提交。`git cherry -v` 和稳定 `patch-id` 均显示这 33 个提交为 `+`，没有 SHA 不同却已经等价吸收的补丁。
- `v0.60.27..30` 已在共同祖先之前，不能重复合并。`v0.60.31..33` 的真正生产候选为 `de09d64`、`7bd1812`、`c9326c0`、`84564a5`、`7cc1d8a`、`d27efdb`、`6cf0aa2`、`d7ba573`、`45691b`；`fd5b84b` 只是 `d7ba573` 的测试补充。
- 上游范围内没有 `.github/workflows/**` 或插件 SDK/Extension Host 的增量。因此本轮不建议以“上游较新”为由引入任何 CI、发布工作流、插件运行时或 SDK 改动。

#### UPA-002：隔离来访 ChatGPT account header

- 状态：`推荐合并`
- 优先级：`P1`。该补丁直接保护 OAuth token 与账户身份的一致性，且当前健康审计已确认该路径真实存在。
- 来源：dyndynjyxa `v0.60.16` / FingerCaster `v0.60.31`，`7cc1d8accc3725d63ff34519fde9d82f285d3510`，`fix(gateway): 隔离客户端 ChatGPT 账号头 (#347)`。
- 当前证据：最终主线 `src-tauri/src/gateway/proxy/handler/failover_loop/prepare/codex_chatgpt.rs:76-85` 遇到来访 `chatgpt-account-id` 就返回；`src-tauri/src/gateway/util.rs:536-543` 的 `clear_all_auth_headers` 不会移除该 header。之后的 `attempt_auth` 会先调用清理函数再注入 provider auth，故来访值可保留到上游。
- 上游行为：在清理函数中移除 `chatgpt-account-id`；在 `maybe_inject_codex_chatgpt_headers` 中无条件先删除来访值，只有选中 Provider 能解析出非空账户 ID 时才插入。上游附带“provider 覆写 client”及“缺 provider ID 时无残留 header”的测试。
- 影响与根因：修复 `CODEBASE_HEALTH_AUDIT.md` 的 `AUD-028`。它只消除账户头混搭，不提供非回环入站认证、也不阻止伪造 `x-aio-gateway-forwarded`，因此不能把 `AUD-016` 标为已解决。
- 冲突与最小移植：以 `aa0a536b` 在临时 worktree 运行 `git cherry-pick --no-commit 7cc1d8a`，`codex_chatgpt.rs` 自动应用；唯一冲突是 `gateway/util.rs` 测试模块的 import 列表。手工摘取两个生产 hunk 和三条 header 测试，不携带上游任务文档。
- 验证：Rust 单测覆盖来访 `acct_client` 被 Provider `acct_provider` 覆写、Provider 无账户 ID 时 header 消失；网关集成测试确保 failover 的每一次 attempt 均使用当前 Provider 派生的 ID。

#### UPA-003：删除 Provider 时收敛路由与排序缓存

- 状态：`推荐合并`
- 优先级：`P1`。删除后前端仍能显示或提交已删除 provider ID，会影响实际路由配置与用户操作。
- 来源：FingerCaster `v0.60.33`，`45691b89c495bb011fd89f97b6953dd6e5d988ae`，`fix: sync provider deletion and identify log attempts`。
- 当前证据：`src/query/providers.ts:314-336` 的 `useProviderDeleteMutation` 只过滤 `providersKeys.list(cliKey)` 和模型/账户缓存；`providersKeys.defaultRoute(cliKey)` 与每个 sort mode 的 provider rows 没有被取消、过滤或失效。`src/query/sortModes.ts:20-27` 也没有按 CLI 聚合这些 keys 的 prefix。这与 `AUD-013` 完全一致。
- 上游行为：新增 `sortModeProvidersQueryPrefix(cliKey)`，先取消主列表、default route 与 sort-mode provider queries，再同步删除缓存行并失效同一组 keys，避免迟到请求复写旧 ID；同一提交还让历史 attempt 显示请求时的 provider 名称和稳定 ID。
- 冲突与最小移植：临时 `cherry-pick --no-commit` 在 `query/providers.ts`、相关前端/Rust 测试和 Trellis 文件冲突。冲突来自本 fork 的 account usage、availability、model catalog 等扩展，不能简单接收上游版本。仅按当前查询键手工实现 cache reconciliation；ProviderChainView 的历史身份展示可作为同批但独立的 UI 小项。
- 验证：预置主列表、default route 和多个 sort-mode rows 后删除 Provider，断言全部缓存都不含该 ID；让所有相关 in-flight 请求在删除后返回旧数据，断言旧行不能复活；UI 测试断言排序提交 payload 和默认路由没有已删除 ID。

#### UPA-004：恢复 Claude OAuth 授权入口与 token 请求身份

- 状态：`条件合并`
- 优先级：`P1`。若上游报告的外部行为仍成立，当前 Claude OAuth 登录会在用户关键路径超时；但端点行为会随远端服务变化，必须实际验证。
- 来源：dyndynjyxa `v0.60.16` / FingerCaster `v0.60.31`，`7bd1812f9502670dd7536f251fbaf8fcc27966bd`，`fix(gateway): restore Claude OAuth login via claude.ai authorize endpoint`。
- 当前证据：`src-tauri/src/gateway/oauth/adapters/claude.rs:24-28` 仍用 `https://platform.claude.com/oauth/authorize`；`src-tauri/src/gateway/oauth/token_exchange.rs:57-80, 141-149` 的 Anthropic token exchange/refresh 直接发送 JSON，没有该 endpoint 的 axios UA；`gateway/upstream_identity.rs` 无 `CLAUDE_OAUTH_TOKEN_USER_AGENT`。
- 上游行为：授权入口改为 `https://claude.ai/oauth/authorize`，token endpoint 保持 `platform.claude.com/v1/oauth/token`；两个 token 流程经单一 request builder 发送 `User-Agent: axios/1.13.6`。上游说明原 Console 入口不再回调 localhost。
- 冲突与最小移植：三方模拟只有 `token_exchange.rs` 冲突，且冲突在本 fork 增加的错误脱敏测试模块；功能 hunk、adapter 和 identity 常量可合。移植时必须保留本 fork 的 `safe_token_request_error`、响应大小限制和错误脱敏，不能以整个上游文件覆盖。
- 验证：先完成 HYP-U03 的真实 OAuth flow；再增加端点组合、exchange/refresh UA 的单测，以及浏览器回调、手动 refresh、401 refresh 的 GitHub Actions native 验证。

#### UPA-005：刷新 OAuth token 后使状态缓存收敛

- 状态：`推荐合并`
- 优先级：`P2`。该问题不会改变真实 token，但会把新到期时间显示成旧值，导致用户错误地重登或忽略即将过期的凭据。
- 来源：dyndynjyxa `v0.60.16` / FingerCaster `v0.60.31`，`84564a5b27db017cab02c77e5f8ad82f799befef`，`fix(providers): 刷新 OAuth Token 后同步更新到期时间展示 (#353)`。
- 当前证据：`src/query/providers.ts:96-105` 的 `fetchProviderOAuthStatus` 直接 `fetchQuery`，既不取消在途查询，也没有 `staleTime: 0`；`providerEditorOAuthActions.ts` 在登录后的 status 读取失败时只提示错误，断开时也不清 query cache。全局 stale cache 可返回刷新前的 `expires_at`。
- 上游行为：先 `cancelQueries`，后以 `staleTime: 0` 强制读取；登录状态读取失败时将成功结果写入 cache，断开时同时清本地状态和 cache，并有 stale/in-flight regression 测试。
- 影响与边界：属于“旧缓存覆盖新状态”的同类问题，但 `AUD-012` 是整份 settings 并发写回；合并此项不能把 `AUD-012` 标为 fixed。
- 冲突与最小移植：四个生产文件自动合并，只有 `providerEditorOAuthActions.test.ts` 和 `providers.test.tsx` 的测试冲突。可选择性移植生产逻辑并合并两组测试意图。
- 验证：登录成功但 status 拉取失败时 cache 必须有可用 fallback；令旧 status 请求晚于 refresh 完成，最终 `expires_at` 必须保持新值；disconnect 后在 staleTime 内重新打开编辑器不能显示旧连接状态。

#### UPA-006：Codex OAuth projection 与 remote-compaction 有界同步

- 状态：`暂不合并`
- 优先级：`P1`。这是当前上游最有价值但也是最不适合直接搬运的可靠性专题。
- 来源：FingerCaster `v0.60.32`，`6cf0aa21d74e4d6a363130c7fcc4d390b0f0426b`（OAuth proxy provider projection）、`d7ba57358fcb32d07a469f24fcf1f0be3757a7a1`（bounded remote compaction sync），以及只应随该专题使用的测试 `fd5b84b8`。
- 上游行为：`6cf0aa2` 引入约 1,101 行 `infra/codex_config/provider_projection.rs`，并重构 settings side effect、Codex config/auth 文件投影、原子备份/恢复和失败后 convergence；`d7ba573` 进一步把 remote compaction 的 `OpenAI`/`aio` provider 切换、历史同步和 model catalog 变成受限的显式流程。
- 当前冲突：当前 fork 已有 managed profiles、model routing、session reuse、external gateway、usage ledger 和自有 Codex 配置路径。`6cf0aa2` 文本上主要冲突 settings/query 测试，却会引入完全不同的 projection SSOT；`d7ba573` 则直接冲突 `infra/cli_proxy/mod.rs`、`infra/cli_proxy/tests.rs`、`infra/codex_config/mod.rs`。直接 cherry-pick 很可能保留文本却破坏 fork 专属行为。
- 健康审计关系：可为 `AUD-002` 的补偿失败、以及 `AUD-012` 的跨层 settings 副作用提供设计参考，但不解决 Prompt/MCP/Skills 的双写问题，也不解决前端整文档写入竞争，不能标记任何 AUD 为 fixed。
- 建议：新建专题设计，提炼“canonical settings -> 显式投影 -> 可报告 rollback/convergence”、“default 不扫描历史、用户显式选择才 bounded sync”、“备份/恢复失败聚合错误”三个不变量；不要导入上游 Trellis 文件、锁文件或整个 module。
- 验证：settings side effect 失败后 settings、Codex config 和认证文件必须最终一致；并发 winner 不能被失败 rollback 覆盖；remote compaction 默认不扫描/重写历史，显式同步有边界；普通文件、缺失文件、symlink/no-follow、备份消失和恢复失败均须覆盖。

#### UPA-007：文件夹排行与开发时间估算

- 状态：`条件合并`
- 优先级：`P2`，属于用户可感知的用量洞察功能，不是既有健康问题的修复。
- 来源：dyndynjyxa `v0.60.16` / FingerCaster `v0.60.31`，`c9326c0a18ab15eebcc5513d2b195f8f1f1dbb2d`，37 个文件、约 `+3478/-2475`。
- 价值：新增 folder leaderboard、开发时间估算、按可见日行（最多 200）计算 activity 的局部优化，并附带较完整的前后端测试。当前 `HomeTokenCostPanel` 已有文件夹多选、folder options 和按日展开后的 folder 明细（`src/components/home/HomeTokenCostPanel.tsx:1232-1652`；后端有 `leaderboard_v2_folder_filtered_with_conn`），所以 folder 维度存在部分重叠；以 `estimated_development_time_ms`、`development_time` 等名称对最终 main 的 `src`/`src-tauri/src` 搜索为 0，开发时长估算仍是净新增能力。
- 冲突与根因：对 `aa0a536b` 的三方模拟有 11 个冲突，涵盖 `package.json`、lockfile、`leaderboard_v2.rs`、主页、usage service；还出现 `HomeTodayProviderUsageOverview` 的 modify/delete。更关键的是上游以 `request_logs` 聚合，当前 fork 的 `leaderboard_v2.rs:166,301,438,579` 和 `cache_rate_trend_v1.rs:98` 以持久 `usage_events`/ledger 为统计事实源。直接合并会回退本 fork 的历史统计语义。
- 依赖结论：上游此提交携带的 `js-yaml`、React Router、React、PostCSS 升级已被当前 lockfile 的解析版本等效或更高地吸收；不能把该 commit 的 package/lockfile 部分当成安全修复再合一次。
- 建议与验证：仅在产品确定需要文件夹洞察时，基于当前 ledger 新实现；保留可见行/日期范围预算并明确“跨文件夹并行可能使 folder 合计超过 day 总计”的语义。验证 SQL 聚合与时区边界、200 行上限、空日、跨文件夹并行，以及长范围基准。

#### UPA-008：Provider 指标走势

- 状态：`条件合并`
- 优先级：`P2`，功能价值明确，但原实现不能通过当前性能门槛。
- 来源：dyndynjyxa `v0.60.16` / FingerCaster `v0.60.31`，`d27efdb8c8bbfadf12c3b76c677a9524f312baee`，29 个文件、约 `+1772/-190`。
- 价值：新增平均请求时延、TTFB、出字速率的 Provider 趋势，抽取 bucket/limit/name fallback 共用逻辑，并提供前后端图表与错误态测试。
- 冲突与根因：三方模拟冲突 cache trend、usage service/query 与测试。上游同样以 `request_logs` 聚合，而本 fork 的用量组件依赖 `usage_events` ledger。更严重的是上游 `trend_common.rs:25-31` 将 `None/0` 映射为无限 limit；页面传 `limit: null`。这会再创建一个 Provider x 日期桶的无界图表，扩大健康报告 `AUD-047`，而不是修复它。
- 建议与验证：若决定建设此功能，先完成 `AUD-047` 的前后端共同预算（最大 custom range、Top-N、bucket/downsample、最大数据点、SQL explain/性能阈值），然后基于 ledger 实现 metrics trend；不要移植 `request_logs` 查询或无限 limit 约定。

#### UPA-009：About 页隐藏未知元数据

- 状态：`推荐合并`
- 优先级：`P3`，只改善可见信息质量，不影响运行时正确性。
- 来源：`de09d64509a1d389e4da57c79317612b66cf02ea`，`fix(ui): 关于应用不展示未知的 Bundle/运行模式`。
- 当前证据：`src/pages/settings/SettingsAboutCard.tsx:31-38` 仍显示 `Bundle —` 和 `运行模式 unknown`。
- 冲突与验证：仅改 About 组件及其测试；对最终基线三方模拟无冲突。验证 known bundle/desktop、unknown bundle、unknown run mode 和 portable action 四种显示状态即可。

#### UPA-010：FingerCaster 的发布资产矩阵与 fork 产品决策冲突

- 状态：`暂不合并`
- 优先级：`P1`，影响发版策略、签名、updater、Homebrew 与用户平台承诺，而不是普通文本冲突。
- 证据：FingerCaster `v0.60.31..33` 发布 Linux AppImage/deb、macOS ARM/Intel、Windows portable/MSI 和 `latest.json`；当前 README、release workflow 和 `AUD-003` 的证据明确本 fork 的正式桌面矩阵与 Intel/Homebrew 假设不一致。
- 结论：只有在用户明确决定恢复 Linux 和 macOS Intel 的正式分发、并同时处理平台签名（见 `AUD-003`、`AUD-043`、`AUD-044` 相关风险）时，才可把 FingerCaster 发布链作为设计参考。当前不应导入其 release-please、版本、Cargo.lock、资产命名或 workflow 变更。

### S3：冲突矩阵与不可合并内容

所有结果在独立临时 clone/worktree 以最终 `aa0a536b` 运行 `git cherry-pick --no-commit <SHA>` 得出，未触碰用户工作区：

| 候选 | 直接移植结果 | 可执行结论 |
|---|---|---|
| `de09d645` | 无冲突 | 可直接挑取或手工重做。 |
| `7bd1812f` | 仅 `gateway/oauth/token_exchange.rs`；冲突在测试模块 | 手工合入功能 hunk，保留当前错误安全处理。 |
| `84564a5b` | 仅两个测试文件冲突，生产文件自动应用 | 可选择性移植并合并测试意图。 |
| `7cc1d8ac` | 仅 `gateway/util.rs` 测试 import；生产 `codex_chatgpt.rs` 自动应用 | 高优先级手工摘取。 |
| `45691b89` | `query/providers.ts`、相关测试、Rust 测试及 Trellis 冲突 | 只实现当前 Query key 的 cache reconciliation。 |
| `6cf0aa21` | 文本冲突少但新增 projection 架构 | 语义冲突高，禁止整提交 cherry-pick。 |
| `d7ba5735` | `cli_proxy/mod.rs`、tests、`codex_config/mod.rs` 核心冲突 | 与 UPA-006 一并专题重设计。 |
| `d27efdb8` | cache trend、usage service/query、测试冲突 | 基于 ledger 新做，先处理性能预算。 |
| `c9326c0a` | 11 个冲突，含旧 Home 组件的修改/删除 | 新功能专题，不导入旧页面或依赖元数据。 |

- 将 FingerCaster `main` 整支直接 merge 会产生 22 个冲突，覆盖版本/锁文件、Cargo 元数据、Usage/Home、Provider Query、Codex 配置和 Trellis。它不满足“没有严重冲突”的前提。
- release-please/version/Cargo.lock 提交、任务归档、journals、上游 merge commits、`fd5b84b` 单独测试提交均无独立产品价值；不列入候选。
- 本轮没有发现可独立移植的上游 CI workflow、插件 SDK 或 Extension Host 改动。不要将“新 release”误用为把这些目录整体更新的理由。

## 最终合并批次

所有批次均是实施建议，不构成对产品代码修改的授权。每个批次应建立独立 PR，并从最终基线 `aa0a536b` 重新做三方冲突检查。

| 批次 | 范围 | 前置条件 | 预期收益 | 必要验证 |
|---|---|---|---|---|
| B0. 来源卫生 | `UPA-001` | 固定 `owner/repo + peeled SHA`；禁止裸 tag/`--tags` 同步 | 避免错误来源、tag 覆盖和不可审计的集成 | 对两个源的同名 tag fixture 断言必须拒绝歧义来源。 |
| B1. OAuth 身份边界 | `UPA-002` / `7cc1d8a` | 无 | 关闭 `AUD-028` 的来访账户头信任路径 | Rust header 单测、Codex ChatGPT failover 集成；同时保持 `AUD-016` 未解决状态。 |
| B2. Provider 删除一致性 | `UPA-003` / `45691b89` | 与最终前端 main 再做一次最小三方 diff | 关闭 `AUD-013` 的失效缓存引用 | React Query late-response 回归、Provider 页面删除交互、默认路由/排序 payload 测试。 |
| B3. Claude/OAuth 可用性 | `UPA-004`、`UPA-005` | HYP-U03 的隔离账户验证；保留当前错误脱敏 | 恢复登录兼容性并让到期状态可靠收敛 | 端点/UA 单测；OAuth 浏览器回调、手动 refresh、401 refresh、断开重开编辑器。 |
| B4. Codex 配置可靠性专题 | `UPA-006` | 先决定现有 Codex config 的唯一事实源和 rollback 责任边界 | 将 settings、投影、备份/恢复和 remote compaction 明确化 | Native CI 覆盖 settings 失败、并发 winner、文件恢复、显式 bounded sync；不 cherry-pick 上游实现。 |
| B5. 用量洞察功能 | `UPA-007`、`UPA-008` | 先治理 `AUD-047` 并确认以 usage ledger 为统计事实源 | 引入文件夹/开发时长、Provider 性能趋势等新能力 | 范围/Top-N/数据点预算、SQL 性能阈值、时区和跨文件夹语义、前端图表错误态。 |
| B6. 可选体验修复 | `UPA-009` | 无 | 清理 About 页无意义的未知字段 | 组件四状态测试。 |
| 不纳入当前路线 | `UPA-010`、上游 release/lockfile/task 提交 | 用户未改变正式平台与发布策略 | 防止发布链、签名、Homebrew 和资产语义被隐式改变 | 如需扩展正式平台，另起发布策略决策与端到端 release 审计。 |

推荐执行顺序：`B0 -> B1 -> B2 -> B3 -> B4 -> B5 -> B6`。B1/B2/B3 可以由不同 PR 并行，但均不得混入 B4/B5 的架构或产品新功能。

## 未覆盖项和验证盲点

- **原生验证受限：** 按项目规则没有运行 Cargo、Rust tests、Clippy、Tauri、Specta 或桌面打包。所有 Rust/Codex/OAuth 结论来自静态 diff、调用路径、上游测试和临时 Git 合并模拟；实施后必须由 GitHub Actions 原生 job 验证。
- **外部 OAuth 行为：** HYP-U03 尚未使用真实 Claude 账号执行，因此 `7bd1812` 的“旧入口实际挂起”不能以本报告当作已复现故障。当前代码仍使用旧入口是已确认事实。
- **发布资产与签名：** 只读取 GitHub Release 元数据和工作流/文档，没有下载、运行或验证任何外部二进制、Notarization、Authenticode 或 updater 签名。`UPA-010` 是策略冲突结论，不是对上游制品安全性的认可或否定。
- **健康度报告时间点：** `CODEBASE_HEALTH_AUDIT.md` 的确定性证据基线是 `9d1fb966`。PR #21 已合并为 `aa0a536b`，本报告已证明它与本轮候选路径无交集；但 PR #21 自身的 TUI/托盘/availability 变更仍应在其发布后独立复审，不能被本报告的上游结论覆盖。
- **上游可变性：** 两个公开仓库在审计结束后仍可能移动。任何实际实现前都必须重新解析 Release tag 的 peeled SHA、复跑 `git cherry` / 三方模拟，并确认 `origin/main` 未再前进。
- **跨产品语义：** 没有用真实历史数据库验证 `request_logs` 与 `usage_events` ledger 在所有迁移/删除/补账场景的差异。该限制正是拒绝直接合并 `UPA-007`/`UPA-008` 的理由，而非遗漏。
- **工作区保护：** 审计过程中当前检出分支从 TUI 工作分支切换到 `codex/release-0.60.44`；未修改其已跟踪内容。结束前仍存在用户/其他 Session 所有的未跟踪 `.impeccable/`、`.playwright-cli/`、`.trellis/workspace/KNaiFen/`、`CODEBASE_HEALTH_AUDIT.md`、`PRODUCT.md`、`upgrade-tui.command`，以及本报告。

## 最终结论

- **不应整支合并任何一个上游。** FingerCaster `main` 的 22 个三方冲突覆盖 fork 已明确分叉的统计、Codex 配置、Provider Query、Home 和发布语义；dyndynjyxa 的最新发布已被 FingerCaster 完整包含。
- **近期最值得合并的是三项小而确定的修复：** `7cc1d8a`（`AUD-028`）、`45691b8`（`AUD-013`）和 `84564a5`（OAuth stale cache）；`7bd1812` 在真实 OAuth 验证通过后也应进入同一优先级队列。
- **最有价值但不能直接拿进来的是 `6cf0aa2`/`d7ba573`。** 它们揭示了当前 fork 在 Codex projection、外部副作用和 remote compaction 上应建立的治理模型，但需要专门设计和原生回归，而不是 cherry-pick。
- **两个用量新功能值得产品评估，但必须先完成资源预算。** `c9326c0` 与 `d27efdb` 的数据源和性能边界不能覆盖当前 ledger 架构；其中 metrics trend 还会直接放大 `AUD-047`。
- **指向上游的任何后续操作都应采用 FingerCaster 为候选源、dyndynjyxa 为历史验证源。** 两者之间同名 tag 冲突，且 dyndynjyxa 没有额外增量；不要把两条来源混成自动同步目标。
