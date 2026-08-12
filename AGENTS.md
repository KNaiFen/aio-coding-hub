# AIO Coding Hub Agent Rules

本文件位于仓库根目录并随每个 Git worktree 检出，是 main session、独立执行 session 和验收 subagent 共用的唯一项目级规则入口。共享合同和执行 session 规则在前，只有 main 才能执行的协调、验收与收尾流程在后；角色说明不构成跨角色授权。

## 角色分工

- **main session**负责需求细化、方案取舍、任务路由、worktree 生命周期、跨任务协调、最终验收、PR 合并、知识库/PENDING 处置和收尾清理。
- **执行 session**只在被分配的任务 worktree 内施工，可以提交、推送自己的任务分支、创建或更新 PR，并负责把该 PR 的 CI 和相关编译跑到通过；交付后暂停。
- **验收 subagents**由 main 按需启动，只读检查任务计划、PR diff 和最新 CI，把证据返回 main，不拥有写入、合并、归档或清理权限。
- `.trellis/agents/implement.md` 和 `.trellis/agents/check.md` 定义的是 Trellis channel agent 的专属行为，不能与这里的独立执行 session 混用。

## 项目级操作合同

- Keep the local checkout zero-artifact.
- 所有仓库和 PR 操作默认使用 `origin`；`upstream` 只读抓取，除非用户明确授权其他操作。
- GitHub CLI 不依赖隐式仓库解析；仓库同时存在 `origin`/`upstream` 时，使用 `gh repo set-default KNaiFen/aio-coding-hub`，并在查看或修改 PR、Actions、issue、release 时显式使用 `-R KNaiFen/aio-coding-hub`。
- 常规仓库 checkout 不得安装依赖，运行 package-manager 脚本、开发服务器、格式化器、类型检查、lint、测试、构建、Cargo、rustfmt、Clippy、Specta 绑定生成、Tauri、签名或打包；不要因为已有 `node_modules` 或 Rust target 就越过该边界。
- 本地验证仅限不写文件的依赖无关 Node.js source contract、对应 selftest、变更 Node 文件的 `node --check` 和 `git diff --check`。使用 `node scripts/check-cloud-only-verification.mjs` 与 `node scripts/check-cloud-only-verification.selftest.mjs`，不要通过 `pnpm` 调用。
- GitHub Actions 负责依赖安装、前端 lint/typecheck/tests/build、Rust 格式/锁同步/生成绑定/Clippy/测试、audit、签名和桌面打包。常规 PR 等待自动触发的 `ci-gate` 与 `pr-title`。Do not start an additional manual `ci` run for routine PR validation. 只有明确的 main 恢复/候选构建、性能基准或桌面集成需求才使用对应 workflow_dispatch。
- 当用户明确要求 upstream merge 或 drift repair 时，保留 `upstream/main` 的非冲突变更；若上游变更与 fork-specific 产品行为或功能冲突，先暂停并向 main 报告具体文件/行为证据和可选方案，不替用户选择任一侧。
- upstream merge/drift-repair 只做集成所需的最小冲突处理，保留明确的 fork 决定；上游已有缺陷记录为范围外事项，另行授权，不在同步任务中顺手修复。

## 共享 Git 与事实边界

- 一个 worktree 同时只能有一个当前唯一写者；执行 session 暂停且 main 确认接管前，其他 session 不得写入。
- 交付、CI、验收和返工证据必须绑定同一完整 PR head SHA；任何新提交都会使旧交付和验收结论失效。
- 不强制删除或清理含有未提交内容、来源不明文件或仍被 session 使用的 worktree。无法确认内容归属时停止并报告 main。
- 任务记录解释过程，不覆盖当前代码、机器可读合同和现行规范；发现冲突时以当前代码和合同为准，并把影响报告给 main。

## 执行 Session 规则

### 施工入口与开工核验

- 以被分配 worktree 内的 `.trellis/tasks/<task>/execution.md` 为唯一施工入口；完整交付规范和 Markdown 模板见当前 checkout 的 `docs/operations/multi-worktree-delivery.md`。任务材料只在该 worktree 的 `.trellis/tasks/<task>/` 保存一份：轻量任务至少有 `prd.md`/`execution.md`，可验收前补齐 `delivery.md`；复杂任务按入口要求增加 `design.md`/`implement.md`；只有 main 验收不通过时才创建 `findings.md`。
- 开始写入前必须阅读本文件、`execution.md`、`prd.md` 以及入口列出的设计、实施、现行规范，并核对：当前绝对目录等于登记 worktree、当前分支等于登记分支、规划提交存在、实施授权已确认、没有材料性未决问题，且 `BASE_SHA="<登记的完整 SHA>"; test "$(git merge-base "$BASE_SHA" HEAD)" = "$BASE_SHA"` 成功。任一项不一致都暂停并报告 main。
- 任务计划中的用户决定、范围、验收标准和停止条件是锁定边界；实现细节可以依据当前代码选择，但不得从聊天记录猜测或静默改变用户决定。
- 执行 session 必须同步任务要求的现行文档、机器合同、测试和迁移材料；发现方案与当前代码冲突、需要破坏兼容性、迁移数据、使用真实凭据或修改公共接口时，先暂停询问 main。

### 施工、PR 与交付

- 执行 session 尽早创建 Draft PR，标题和正文遵循仓库约定并链接任务目录；按锁定范围实现并提交，可以修复自己 PR 的 CI/编译失败，但不得静默扩大范围或改变用户锁定决定。
- 执行 session 可以提交和推送任务分支、创建或更新 PR；不得推送 `main`、合并 PR、开启自动合并、运行 main 专属 `/trellis:finish-work` 或任务归档，也不得删除 worktree 或分支。
- `delivery.md` 必须基于实际代码填写，不复制计划；至少记录用户可见/内部行为、关键文件/模块/符号、每条 AC 及证据、计划偏移、真实本地/云端/人工验证、配置/API/兼容性/安全影响、剩余风险，以及候选完整 PR head/base SHA 和对应 `ci-gate` 链接。
- 早期阻塞可以没有提交、PR 或 CI；必须如实记录“尚未提交/未触发及原因”，不制造空提交或虚构证据。等待验收或返工候选则必须绑定完整 head SHA。
- 实施完成的交付门是：范围内代码、测试和文档已提交推送，PR 指向 `main`，最新 head 的必需检查和相关编译为绿色，需要的人工验证已完成或明确交由 main/用户，`delivery.md` 已记录实际结果和证据。随后把 PR 标记可评审、停止写入并通知 main 验收。
- CI 绿色不替代 main 对需求、设计、回归风险和文档准确性的验收。main 验收期间不得继续修改；只有 main 可以创建整改 `findings.md`，执行 session 按轮次修复、推送新 head、更新交付记录并再次暂停。
- CI 失败时优先修复任务范围内的问题；若疑似基础设施或 main 既有问题且没有可靠的任务内修法，保留日志链接并交 main 判断，不自行忽略。

### 执行 Session 停止条件

出现以下任一情况，停止写入并报告 main：

- 当前目录、分支、base SHA、规划提交、任务授权或唯一写者与登记不一致。
- 必须修改“允许修改”之外的重要文件、公共 API、数据迁移、发布配置或其他活动 worktree 的语义边界。
- 用户锁定决定、现行合同、任务材料与当前代码互相冲突，或材料性未决问题尚未关闭。
- CI/环境失败无法证明属于本任务且没有安全修法，或无法满足验收标准。

阻塞时在 `delivery.md` 记录证据、最后安全提交、工作树状态、受影响 AC、决定归属和恢复条件，再暂停。执行 session 不运行 main 专属归档或清理命令，即使 PR 已合并也等待 main 明确处理。

## Main 专属规则

本节只授权 main session。执行 session 和验收 subagent 即使读到本节，也不得据此接管 main 的写入、合并、归档或清理权限。

### 规划、任务路由与记录

- main 按“规划与登记 -> 执行 session 施工并暂停 -> 固定 head 验收/返工 -> 再核验 CI 后合并 -> 记录结果与归档 -> 确认干净后清理”的顺序推进。
- 在 Plan mode 主动调查当前行为和相关代码，形成明确目标、范围、约束、锁定决定和验收标准；能从仓库或现有证据确认的技术事实主动查清，涉及用户意图、产品行为、优先级、取舍或成功标准的不确定性必须询问。材料性未决问题未关闭时不得开始修改。
- 简单、低风险且由 main 连续完成的任务可以直接在本检出施工；远端 `main` 受保护时仍使用短期 PR 分支。交给独立执行 session、需要并行/长流程/隔离状态、范围较大或风险较高的任务，创建同级 worktree；任务实施中扩大时保留原记录并迁移。
- 用户确认方案并授权实施后，修改前先在月度记录或 Trellis 任务 Markdown 落盘；结束前补实际结果，无论完成、部分完成、失败、阻塞还是放弃。简单任务使用 `docs/history/change-records/YYYY-MM.md`；复杂或委派任务使用 Trellis 任务包、`execution.md`、`delivery.md` 和必要的 `findings.md`。聊天不能替代仓库记录，不新增自定义 JSON 门禁。
- 方案变化影响行为、兼容性、范围或 AC 时，回写原记录并重新取得用户确认后才能恢复；main 不维护与任务 worktree 竞争的第二份详细方案。
- 创建任务 worktree 前必须 `fetch origin`，确认 main 检出干净且与 `origin/main` 同步，并记录派生所用的完整 `origin/main` SHA；不得从脏的或过期的 main 派生任务。

### Worktree 交接与协调

- 创建并登记 sibling worktree 后，主动向用户发送可执行启动交接：任务/目标、权威任务目录、绝对路径、分支、完整 base SHA、规划提交、PR/head/CI、当前唯一写者、可否开工、依赖/冲突、范围外文件、禁止动作、交付/暂停条件和 main 下一道验收门。
- 交接必须附可直接粘贴的执行 Prompt，指向 `execution.md`，重复绝对路径、分支、唯一写者和 preflight，并要求按范围施工、提交推送、更新 `delivery.md`、等待最新 head 的必需 CI 后暂停。
- 当用户报告暂停 session 时，核验路径仍是当前 PR 的注册 worktree；已删除、归档、合并或不相关的目录必须关闭且不得复用。
- 若任务、用户或候选明确要求先等 CI，main 监控同一完整 head，只在必需检查绿色、head 未漂移、worktree 存在且状态可归属时通知开工；运行中、失败、未知、过期或路径异常时明确“暂不启动”及下一次检查条件。纯规划登记 CI 不自动构成实现门禁。
- CI 轮询、GitHub 事件/冲突诊断、用户沟通和跨 worktree 决策留在 main 的月度记录或活动索引；执行材料只保留执行者下一步所需的事实和简短启停条件。

### 验收、返工与收尾

- 收到交付后，确认执行 session 已暂停，读取 `execution.md`、`delivery.md`、设计/PRD 和实时 PR diff；检查冻结 head 的 base、必需 CI、相关编译、合并状态、范围、AC、回归风险、测试和文档。
- main 可派只读 subagent 提供特定风险线索，但必须亲自核对出处并作最终结论。每轮验收在 `delivery.md` 绑定完整 head SHA、`ci-gate` 证据、结论、接受的偏移/风险和日期。
- 验收不通过时，默认由执行 session 返工；main 写可执行的 `findings.md`（稳定编号、严重度、证据、影响、期望结果和复验标准）。符合下方低风险条件的记录性小问题，可以由 main 直接修复。

#### `main-direct-fix` 验收返工分流

该例外不以改动行数或文件后缀作为唯一依据。以下条件必须全部满足，main 才能直接修复：

1. 执行 session 已明确暂停；main 先在活动索引和任务 `execution.md`（或同等权威交接记录）把当前唯一写者改为 `main-direct-fix`，记录接管时间、冻结 head、工作树状态和未提交内容归属，再在原任务 worktree、原任务分支和原 PR 上接管临时唯一写权。接管记录未落盘前不得编辑。
2. 问题单一、明确、局部，修复方案已确定或属于机械性修正，不需要重新设计、追加用户决定或扩大任务范围。
3. 修复不改变用户锁定决定、产品行为、API、兼容性、安全边界、数据迁移、架构、接口/数据流或验收标准。
4. 修改限于任务已批准范围内的记录性文档，例如 `delivery.md`、`findings.md`、`execution.md`、`prd.md`、`design.md` 中缺失或过期的字段、状态、路径、SHA、链接、日期、拼写、格式，以及对既有事实的明确澄清；设计决定本身不得改变。
5. 预判并由实时 `change-scope` 证明只进入 process/checked documentation 短合同检查，不是未知、未分类、控制面或 `shared` 路径；`ci-gate` 必须证实不会选中 `frontend_ci`、`rust_ci`、`full_ci`，也不会触发编译、依赖安装、生成、构建、签名、打包、候选发布或性能基准。
6. main 能在当前任务范围内独立证明结果，不需要接管其他 worktree、改变分支关系或运行执行 session 专属施工/归档命令。

典型例子是交付文档漏字段、验收记录遗漏 head/CI 链接、设计文档的事实性小错误，以及路径、状态、日期、拼写或格式同步。出现产品代码或测试逻辑、workflow/policy/selftest、依赖/锁文件、生成文件、公共 API、迁移、架构、原因不明、跨模块、需要用户重新确认、写权/内容归属不清，或预计/实际触发长任务时，必须交回执行 session。

main 直接修复时必须在 `findings.md` 标记 `返工责任：main-direct-fix`，记录接管时间、原因、修改范围、预期与实际 `scope`、`full_ci`、`frontend_ci`、`rust_ci`、`shared_ci`、`docs_checks`、选中/跳过 jobs、保持不变的行为和复验标准；在 `delivery.md` 追加修改、完整新 head、对应 CI 和新的验收轮次。修复后推送原任务 PR，等待新 head 的检查终态并重新验收。若实际 scope 意外选中长任务，立即停止，在活动索引和 `execution.md` 记录交回时间、原因、最后安全 head、工作树状态和恢复条件，把唯一写者改回“执行 session 待返工”并明确通知后才允许恢复；不得把该例外继续推进为直接推送 `main`、合并、归档或清理。

- 验收通过且 CI 仍绿时，只由 main 合并 PR；fetch 并快进本地 `main`，记录真实 merge commit，更新长期知识和 PENDING 去向，按 Trellis 规范 archive/validate，再删除已合并、干净、无人使用且内容归属明确的 worktree 和分支。
- 阻塞任务保持活动，不伪造合并或归档；失败、放弃或无功能 PR 的部分完成通过只含记录的收尾 PR 保存结论、风险、PENDING/知识库处置和清理计划。

### 知识库、PENDING 与发布

- `docs/README.md` 是产品、架构、插件、运维、任务和历史文档的导航入口；当前代码和机器合同高于现行规范，现行规范高于任务记录，任务记录高于历史审计/会话日志。
- `PENDING.md` 是用户要求积累的未解决事项清单。正式计划或明确开始修改前读取全部 `pending`/`planned` 条目并纳入候选清单；完成合并和验证并有证据后才迁入 `PENDING_COMPLETED.md`，放弃必须有用户明确决定和原因。
- fork release 默认只递增 patch 版本（例如 `0.60.31` -> `0.60.32`）；需要更大版本变更或兼容性要求时，必须有用户明确决定。
- release workflow 不得只按 release tag checkout。Draft GitHub Release 可能早于可抓取的 Git tag；先解析或创建 tag，再把不可变的 commit SHA 传给后续构建 job。

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. The independent execution-session boundary above still applies: delegated writers stop at `delivery.md` and do not run main-only finish/archive commands.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
