# Main Coordinator Rules

本文件位于仓库协调主检出 `main/`，只补充 main session 才需要执行的规划、协调、验收和收尾规则。父目录的 [AGENTS.md](../AGENTS.md) 承载所有 checkout 共用的项目合同和完整执行 session 规则；任务 worktree 中的执行 session 不应把本文件当作自己的施工授权。

## Main 职责

- main 负责需求细化、方案取舍、任务路由、Markdown 计划、worktree 创建与登记、用户启动交接、跨 worktree 协调、固定候选验收、返工意见、PR 合并、最终记录、知识库/PENDING 处置以及 worktree/分支清理。
- 创建任务 worktree 前，main 必须 `fetch origin`，确认本检出干净且与 `origin/main` 同步，并记录派生所用的完整 `origin/main` SHA；不得从脏的或过期的 main 派生任务。
- main 按“规划与登记 -> 执行 session 施工并暂停 -> 固定 head 验收/返工 -> 再核验 CI 后合并 -> 记录结果与归档 -> 确认干净后清理”的顺序推进。执行 session 的新提交会使旧交付和验收失效。
- main 不得强制清理含有未提交内容、来源不明文件或仍被 session 使用的 worktree；无法确认归属时停止并向用户报告。

## Planning Mode

- 在 Plan mode 主动细化请求、调查当前行为和相关代码，形成明确目标、范围、约束、锁定决定和验收标准后再实施。
- 能从仓库或现有证据确认的技术事实由 main 主动查清；涉及用户意图、产品行为、优先级、取舍或成功标准的不确定性必须询问，不得猜测。
- 区分已确认事实、假设、未决问题和用户决定；材料性未决问题未关闭时不得开始修改。

## Task Routing And Records

- 简单、低风险且由 main 连续完成的任务可以直接在本检出施工；远端 `main` 受保护时仍使用短期 PR 分支，不绕过保护。
- 交给独立执行 session、需要并行/长流程/隔离状态、范围较大或风险较高的任务，创建同级 worktree。任务开始简单但实施中扩大时，保留原记录并迁移，不继续堆叠风险。
- 用户确认方案并授权实施后，修改前先在月度记录或 Trellis 任务 Markdown 落盘；结束前补实际结果，无论完成、部分完成、失败、阻塞还是放弃。聊天不能替代仓库记录。
- 简单任务使用 `docs/history/change-records/YYYY-MM.md` 的“确认方案/实施结果”条目；复杂或委派任务使用 Trellis 任务包、`execution.md`、`delivery.md` 和必要的 `findings.md`。不新增自定义 JSON 门禁。
- 方案变化影响行为、兼容性、范围或 AC 时，回写原记录并重新取得用户确认后才能恢复；main 不维护与任务 worktree 竞争的第二份详细方案。

## Worktree Handoff And Coordination

- 创建并登记 sibling worktree 后，主动向用户发送可执行启动交接：任务/目标、权威任务目录、绝对路径、分支、完整 base SHA、规划提交、PR/head/CI、当前唯一写者、可否开工、依赖/冲突、范围外文件、禁止动作、交付/暂停条件和 main 下一道验收门。
- 交接必须附可直接粘贴的执行 Prompt，指向 `execution.md`，重复绝对路径、分支、唯一写者和 preflight，并要求按范围施工、提交推送、更新 `delivery.md`、等待最新 head 的必需 CI 后暂停。
- 当用户报告暂停 session 时，核验路径仍是当前 PR 的注册 worktree；已删除、归档、合并或不相关的目录必须关闭且不得复用。
- 若任务、用户或候选明确要求先等 CI，main 监控同一完整 head，只在必需检查绿色、head 未漂移、worktree 存在且状态可归属时通知开工；运行中、失败、未知、过期或路径异常时明确“暂不启动”及下一次检查条件。纯规划登记 CI 不自动构成实现门禁。
- CI 轮询、GitHub 事件/冲突诊断、用户沟通和跨 worktree 决策留在 main 的月度记录或活动索引；执行材料只保留执行者下一步所需的事实和简短启停条件。

## Acceptance And Closeout

- 收到交付后，确认执行 session 已暂停，读取 `execution.md`、`delivery.md`、设计/PRD 和实时 PR diff；检查冻结 head 的 base、必需 CI、相关编译、合并状态、范围、AC、回归风险、测试和文档。
- main 可派只读 subagent 提供特定风险线索，但必须亲自核对出处并作最终结论。每轮验收在 `delivery.md` 绑定完整 head SHA、`ci-gate` 证据、结论、接受的偏移/风险和日期；新提交使该轮结论失效。
- 验收不通过时，默认由执行 session 返工；main 写可执行的 `findings.md`（稳定编号、严重度、证据、影响、期望结果和复验标准）。符合下方低风险条件的记录性小问题，可以由 main 直接修复，不必为了往返而退回执行 session。
- 无论由谁修复，交付、修复和验收都必须绑定同一任务 lineage 的完整 PR head SHA；任何新提交都会使旧交付和验收结论失效。

### 验收返工分流

`main-direct-fix` 是验收返工的受控例外，不以改动行数或文件后缀作为唯一依据。以下条件必须全部满足，main 才能直接修复：

1. 执行 session 已明确暂停；main 先在活动索引和任务 `execution.md`（或同等权威交接记录）把当前唯一写者改为 `main-direct-fix`，记录接管时间、冻结 head、工作树状态和未提交内容归属，再在原任务 worktree、原任务分支和原 PR 上接管临时唯一写权。接管记录未落盘前不得编辑。
2. 问题是单一、明确、局部的问题，修复方案已确定或属于机械性修正，不需要重新设计、追加用户决定或扩大任务范围。
3. 修复不改变用户锁定决定、产品行为、API、兼容性、安全边界、数据迁移、架构、接口/数据流或验收标准。
4. 允许修改的内容限于任务已批准范围内的记录性文档，例如 `delivery.md`、`findings.md`、`execution.md`、`prd.md`、`design.md` 中缺失或过期的字段、状态、路径、SHA、链接、日期、拼写、格式，以及对既有事实的明确澄清。设计文档只有事实/表达修正才适用；改变设计决定不适用。
5. 按变更路径和 CI 分类预判，该修复只会进入 process/checked documentation 短合同检查；未知、未分类、控制面或 `shared` 路径一律不适用。推送后必须以实时 `change-scope` 输出记录 `scope`、`full_ci`、`frontend_ci`、`rust_ci`、`shared_ci`、`docs_checks`，并以 `ci-gate` 的选中/跳过结果证实不会选中 `frontend_ci`、`rust_ci`、`full_ci`，也不会触发编译、依赖安装、生成绑定、构建、签名、打包、发布候选或性能基准。
6. main 能在当前任务范围内独立证明结果，且不需要接管其他 worktree、改变分支关系或运行执行 session 专属施工/归档命令。

典型例子是：交付文档漏一个必填字段、验收记录遗漏 head/CI 链接、任务设计文档的事实性小错误、路径/状态/日期/拼写或格式同步。文档内容若涉及行为、权限、范围、兼容性或验收标准的实质变化，仍按高风险返工处理。

出现以下任一情况，必须把返工交回执行 session：涉及产品代码或测试逻辑；涉及 workflow、policy/selftest、依赖/锁文件、生成文件、公共 API、迁移或架构；问题原因不明确或跨多个模块；需要用户重新确认；worktree、分支、写权或未提交内容归属不清；预计或实际选中了 `frontend_ci`、`rust_ci`、`full_ci` 或任何编译/构建/生成/打包/长时监控 job。

main 直接修复时必须在 `findings.md` 标记 `返工责任：main-direct-fix`，记录接管时间、原因、修改范围、预期与实际 CI 分类字段、选中/跳过的 jobs、保持不变的行为和复验标准；在 `delivery.md` 追加 main 的修改、完整新 head、对应 CI 和新的验收轮次。修复后提交并推送原任务分支/原 PR，等待新 head 的检查终态并重新验收。若实际 scope 意外选中长任务，立即停止继续修改，在交回执行 session 前把活动索引和 `execution.md` 的当前唯一写者改回“执行 session 待返工”，记录交回时间、原因、最后安全 head、工作树状态和恢复条件，并明确通知后才允许 session 恢复；不得把 main 直接修复变成直接推送 `main`、合并、归档或清理。
- 验收通过且 CI 仍绿时，只由 main 合并 PR；fetch 并快进本地 `main`，记录真实 merge commit，更新长期知识和 PENDING 去向，按 Trellis 规范 archive/validate，再删除已合并、干净、无人使用且内容归属明确的 worktree/分支。
- 阻塞任务保持活动，不伪造合并或归档；失败、放弃或无功能 PR 的部分完成通过只含记录的收尾 PR 保存结论、风险、PENDING/知识库处置和清理计划。

## Project Knowledge And PENDING

- `docs/README.md` 是产品、架构、插件、运维、任务和历史文档的导航入口；当前代码和机器合同高于现行规范，现行规范高于任务记录，任务记录高于历史审计/会话日志。
- `PENDING.md` 是用户要求积累的未解决事项清单。正式计划或明确开始修改前读取全部 `pending`/`planned` 条目并纳入候选清单；完成合并和验证并有证据后才迁入 `PENDING_COMPLETED.md`，放弃必须有用户明确决定和原因。

## Release Coordination

- fork release 默认只递增 patch 版本（例如 `0.60.31` -> `0.60.32`）；需要更大版本变更或兼容性要求时，必须有用户明确决定。
- release workflow 不得只按 release tag checkout。Draft GitHub Release 可能早于可抓取的 Git tag；先解析或创建 tag，再把不可变的 commit SHA 传给后续构建 job。

## Machine Contract Anchors

仓库的 dependency-free contract checker 会直接校验以下稳定锚点；它们在这里保留是因为 CI 只看到仓库 checkout，详细操作含义仍以父目录共享规则和现行 CI 合同为准：

- Keep the local checkout zero-artifact.
- `node scripts/check-cloud-only-verification.mjs`
- Do not start an additional manual `ci` run for routine PR validation.

<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. This does not override the shared independent execution-session boundary: delegated writers stop at `delivery.md` and do not run main-only finish/archive commands.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
