# Cloud-only build and artifact promotion

## Goal

让仓库控制的本地开发流程不再隐式或显式触发 Rust/Tauri 原生构建，并把所有必需的原生校验、生成与打包移到 GitHub Actions。正式发布必须直接晋升精确 `main` 提交已经由成功 CI 生成并签名的制品，标签工作流不得再次编译应用。

## Requirements

### 本地零原生构建触发

- `pnpm install`、`git commit`、`git push`、Trellis 任务归档或会话记录不得触发 Cargo、rustfmt、Clippy、Rust 测试、Specta 绑定生成或 Tauri 构建。
- 删除仓库托管的 Git hooks、自动安装器及 `postinstall` 副作用；仅当当前克隆的 `core.hooksPath` 精确等于 `.githooks` 时清除该本地 Git 配置，不覆盖用户自定义 hook 路径。
- 删除或改造名称看似普通检查、实际执行 Rust 或写回生成文件的 package scripts。保留的本地检查仅允许 Node/TypeScript、前端测试和前端构建。
- 本地不再提供仓库认可的 `tauri:dev`、`tauri:build:*`、`tauri:test`、`tauri:gen-types` 等原生入口；本地 UI 开发只使用 Vite，原生集成验证通过云端构建制品完成。
- 仓库无法阻止用户直接手输任意 `cargo` 命令，但 `AGENTS.md`、README、活动规范和未完成任务不得再要求或建议这么做。

### 云端原生校验与修复制品

- PR 与 `main` CI 在云端执行 Rust 格式化规范化、`Cargo.lock` 同步检查、生成绑定、Clippy、完整 Rust 测试和 Cargo audit。
- Rust 格式、锁文件或生成绑定有漂移时，CI 必须上传一个有界、可直接 `git apply` 的补丁制品后失败；不得要求开发者本地运行 Rust，也不得让机器人在不受信任 PR 上自动提交。
- 生成绑定不得继续在独立的前端 runner 中重复编译 Rust；它应与 Rust 校验共享 runner 和 Cargo cache。
- 前端 CI 不再安装仅为生成绑定所需的 Tauri/Linux 系统依赖。
- `main` 的 CI 运行使用唯一并发组，后续提交不得取消仍可能成为发布候选的精确提交；PR 和 `dev` 仍可取消被新提交取代的运行。

### 云端手动构建

- 现有支持矩阵中所有曾标记为源码构建或仅本地构建的目标，都必须能通过 `workflow_dispatch` 在云端选择并生成开发制品。
- 手动开发制品不得被正式 Release 使用，也不得接触正式 updater 签名密钥。
- 手动云端构建必须使用临时 Tauri config overlay 显式关闭 `bundle.createUpdaterArtifacts`，避免无签名开发构建继承正式配置后失败或生成伪正式 updater 文件。
- 交互式原生热更新不在云端等价实现范围内；本地仅保留前端 Vite 热更新。

### 构建一次并晋升

- 仅可信仓库的 `main` 版本变更提交，或从当前 `main` 控制面显式触发且经 ancestry/既有成功 exact-SHA CI 验证的恢复目标 SHA，可以生成正式发布候选；首次 main 候选以同一次运行的完整质量 job 为前置，不要求预先存在成功运行。任意功能分支或标签 ref 不得接触签名密钥。
- 正式矩阵保持 Windows x64 和 macOS ARM64，不借本任务扩大发布平台。
- 平台构建可以与质量校验并行，但只有同一次 CI 的全部必需检查成功后，才组装并保留最终发布候选制品。
- 两个 `TAURI_SIGNING_*` secret 必须从仓库级迁到受保护的 `release-signing` Environment；该 Environment 的 deployment policy 只允许 `main`，签名 job 必须显式声明该 Environment。
- 环境级 secret 验证可用后删除同名仓库级副本，避免任意同仓分支/workflow_dispatch 绕过环境策略读取签名材料。
- 迁移不得要求用户找回或学习私钥：一次性受审计云端步骤读取现有仓库 secret，并用目标 Environment 的 GitHub 公钥生成 LibSodium sealed-box 密文；本机已登录的 `gh` 只把该密文写入 Environment API。任何明文不得进入 artifact、日志、命令输出或工作区。
- 最终候选必须包含版本化清单，绑定仓库、源码 SHA、可信控制面 SHA、源码验证 run ID/attempt、应用版本、派生标签、候选 workflow run ID/attempt、目标集合，以及每个文件的名称、大小和 SHA-256；不得跨 run attempt 拼装。
- 最终候选制品保留 30 天；临时平台制品保持短期并且不能直接发布。
- 标签触发的 Release 只允许：验证标签与版本、定位精确成功的 `main` CI、按不可变 artifact ID 下载、校验清单和文件、生成发布元数据、上传并发布。
- Release 工作流不得包含 Cargo、Rust toolchain、`pnpm install`、Tauri action、签名密钥或构建矩阵，也不得在候选缺失、过期或不匹配时回退重建。
- Release 必须以只读解析 job 和独立写入 job 两次下载并验证同一 artifact ID，所有校验通过后才创建或复用带固定工作流所有权标记的 draft；重跑同一发布时使用稳定的发布时间来源，避免 `latest.json` 和校验和无故漂移。
- 复用 draft 时必须验证 draft 状态、标签和目标提交，删除该受管 draft 的既有资产后上传完整集合；上传完成后精确对账文件名、数量、大小和 SHA-256 digest，无缺失或额外资产才允许发布。若 API 不提供 digest，则回下载并计算哈希或失败关闭。

### 清理与文档

- 删除没有实际 release-please 入口且依赖不存在 secret 的 Cargo.lock 同步工作流、配置、manifest 和检查脚本。
- 更新支持矩阵合同，反向约束“CI 产出、Release 晋升”，并静态拒绝 Release 中重新出现原生构建命令。
- 更新中英文 README、`AGENTS.md`、相关 Trellis 规范和仍未归档任务中的未来执行指引；归档任务中的历史验证证据保持不变。
- 新增跨层规范，记录云端原生校验、漂移补丁、候选清单、制品晋升和失败关闭合同。

## Constraints

- 本任务实施和本地验证期间禁止运行任何 Rust/Cargo/Tauri 编译或生成命令。
- 不修改 AIO 运行时业务行为、数据库、IPC、前端产品功能或发布目标集合。
- 不操作 `upstream`，不新增长期写权限 token。是否为 `main` 和正式发布标签新增 GitHub Ruleset 是实施前待用户确认的仓库治理决定。
- GitHub 不允许读回现有 secret 值；签名迁移使用一次性密文桥接，不要求用户重新录入，且迁移 helper/workflow 必须在验证和删除仓库级副本后从代码库移除。
- 不在本任务中实现命令行 TUI，不自动提版本、打标签或发布 Release。
- 保留现有未跟踪的 `.trellis/workspace/KNaiFen/`，不得纳入提交。

## Acceptance Criteria

- [ ] 全仓审计证明不存在 `postinstall` hook 安装、有效跟踪 Git hook、编辑器任务、Trellis lifecycle hook 或 package 聚合脚本触发本地 Rust/Tauri。
- [ ] 当前克隆不再配置 `core.hooksPath=.githooks`，同时不会删除任何非 `.githooks` 的用户自定义路径。
- [ ] `pnpm install`、普通提交和普通推送均不执行仓库原生校验或修改工作树。
- [ ] 本地公开检查入口只运行前端/Node；所有 Rust 格式、锁文件、绑定、测试、Clippy、audit 和 Tauri 打包均有云端所有者。
- [ ] 云端漂移会产生可应用补丁并失败，补丁不含工作区外文件、secret 或构建输出。
- [ ] 手动云端构建覆盖支持矩阵全部六个既有目标，并与正式发布制品隔离。
- [ ] 手动云端构建在无签名 secret 时通过临时 overlay 关闭 updater 产物，正式 `tauri.conf.json` 保持不变。
- [ ] 精确 `main` 版本提交的成功 CI 产出一个绑定 source/control/validation SHA/run attempt 的 30 天发布候选；同版本后续提交可显式云端恢复，失败 CI 或跨 attempt 临时平台制品不能产生可晋升候选。
- [ ] 签名 job 只能通过 `release-signing` Environment 获得 secret；该 Environment 只允许当前 `main` workflow 部署，任意 PR、功能分支或标签 ref 均无法部署，仓库级不再保留签名 secret。
- [ ] `release-signing` 的 API 状态必须同时满足 `can_admins_bypass=false`；在网页关闭管理员绕过前不得写入 Environment secret 或删除仓库级 secret。
- [ ] Release 对同一 SHA 不再执行任何编译，下载并校验精确 artifact ID 后发布完全相同的二进制文件。
- [ ] 缺失、过期、重复、错误 SHA/版本/标签/run attempt/目标/大小/哈希的候选均在创建 draft 前失败，且不存在回退构建。
- [ ] Release 重跑不会保留旧 draft 资产；上传后远端资产名称、大小与 SHA-256 均和本次清单完全一致才发布。
- [ ] 后续 `main` push 不会取消较早的发布候选 CI；PR/dev 的过时运行仍会被取消。
- [ ] release-please 残留入口与不存在的 secret 依赖全部删除，支持矩阵自检覆盖新合同。
- [ ] 本地只运行非 Rust 校验；Rust、Tauri、签名、跨平台打包及最终工作流验收均由云端 CI 完成。

## Notes

- 直接手输系统级 `cargo` 无法由仓库技术上绝对禁止；验收范围是清除仓库控制的触发面和指引，并让代理规则明确禁止本地原生构建。
- 最近一次可比证据：同一提交 `ebecb287535092d308aec3b887c09d45e8e95fc2` 的 main CI 运行 `30617682335` 用约 18 分钟且产物为 0，Release 运行 `30619327949` 随后又用约 28 分钟执行 Windows/macOS 构建。

## Approved Decisions

- 用户已批准新增保护：`main` 必须通过 PR 合入、要求完整 CI 成功、禁止 force push/删除；`aio-coding-hub-v*` 使用两套独立 Ruleset，创建规则仅对明确授权维护者设置 bypass，更新/删除规则不设任何 bypass。Environment 仍只允许 `main`，标签 Ruleset 用于发布引用完整性而不是 secret 授权。
- 用户已批准创建 `release-signing` Environment，并通过一次性无明文密文桥接迁移现有 `TAURI_SIGNING_PRIVATE_KEY` 与 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`；未完成迁移、验证和仓库级副本删除前不得启用 main CI 签名构建。
