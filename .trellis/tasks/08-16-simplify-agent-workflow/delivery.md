# 交付报告：精简多 worktree 多 session 工作流

> 本文件只写实际实现和证据。实时候选 head、base 和检查以 PR #152 与 GitHub Checks 为准。

## 结果

- 结果：等待验收；最终 head CI 待 `deliver` 状态提交推送后完成。
- PR：https://github.com/KNaiFen/aio-coding-hub/pull/152
- 执行者：当前 Codex execution session
- 一句话结果：跨窗口协作改为短根规则、`gkd-` 角色 skills、脚本化协调状态和分阶段文档，不再依赖模型手写多份 Git/worktree/验收状态。

## 实际实现

### 用户可见行为

- 新窗口通过 `$gkd-main`、`$gkd-execute`、`$gkd-accept` 进入对应角色；执行窗口的 handoff 会显式调用带统一短前缀的 skill。
- `task.py status/doctor/delegate/handoff/deliver/block/resume` 负责读取、校验和写入 `coordination.version=1`；阻塞、验收写权和返工写权都有确定转换。
- 主入口只直链四个阶段专题和四个模板；执行者不再加载 main 的历史验收与收尾轮次。

### 内部机制

- `task.json` 保留未知字段并原子替换；v1 校验使用严格整数版本，核对 canonical worktree、branch、完整 base/planning SHA 和祖先关系。
- stale/completed session pointer 会在可确定时清理；损坏 JSON 和多个 live session 仍拒绝猜测。
- GitHub 保持 PR head/base/CI/merge 的实时事实源；`status` 不输出 legacy `commit/pr_url`，`delivery.md` 不自引用当前 head。
- Codex Trellis dispatch 改为 `inline`；用户新开的顶层窗口由 handoff 和 repo skill 路由，而不是不存在的 `.codex/agents`。

## AC 证据

| AC | 结果 | 证据 |
|---|---|---|
| AC-01 | 通过 | handoff 固定输出 primary folder、`$gkd-execute`、status/doctor、任务入口；`doctor` 在当前 worktree 通过。 |
| AC-02 | 通过 | `task_coordination.py` 的集中 writer/validator；单测覆盖未知字段、非法转换、空输入、阻塞往返和写权交接。 |
| AC-03 | 通过 | `active_task.py` 的 stale 清理及 `test_active_task.py` 的 stale/live/损坏/多 session 用例。 |
| AC-04 | 通过 | 三个 `.agents/skills/gkd-*` 已被 Git 跟踪；cloud-only contract 固定精确 skill 名和默认 prompt 前缀。 |
| AC-05 | 通过 | 核心规则、workflow、专题、模板和三 skill 合计约 49 KB；`AGENTS.md + execute skill + execution template` 约 10 KB。 |
| AC-06 | 通过 | `multi-worktree-delivery.md` 直接链接四专题和四模板，无第二层必读索引。 |
| AC-07 | 通过 | execution 模板 80 行、delivery 74 行、findings 71 行；验收/收尾迁入 37 行 acceptance 模板。 |
| AC-08 | 通过 | 现行入口中的旧 slash 命令、死 completed breadcrumb 和 `review` 顶层状态已清理；扩展后的文档链接合同通过。 |
| AC-09 | 待最终 CI | 允许的本地检查已通过；标准库 Python unittest 已接入 contracts job，等待最终 head CI。 |

## 关键位置

| 文件或符号 | 实际变化 | 设计原因 |
|---|---|---|
| `.trellis/scripts/common/task_coordination.py` | coordination v1、状态转换、doctor、handoff | 用单一 writer/validator 替代模型手写 JSON 和交接状态。 |
| `.trellis/scripts/common/active_task.py` | stale pointer 清理 | session pointer 只做可丢弃路由，不污染持久任务事实。 |
| `.agents/skills/gkd-*/SKILL.md` | main/execute/accept 三角色入口 | 新窗口按角色、按阶段加载，不依赖自定义 subagent 身份。 |
| `AGENTS.md`、`.trellis/workflow.md` | 硬规则与状态导航收敛 | 减少常驻上下文和不存在能力造成的误导。 |
| `docs/operations/multi-worktree-delivery.md` | 主入口与四阶段专题 | 将约千行任务包中的通用教程移出执行者默认上下文。 |
| `.github/workflows/ci.yml` | Python coordination unittest | 依赖无关、单条短合同，不增加人工审批。 |

## 计划偏移

- 首次交付后按用户决定将三个 role skill 从 `aio-trellis-*` 统一改名为更短的 `gkd-*`；目录、frontmatter、UI metadata、handoff、文档和合同测试同步修改，不保留旧别名。
- 在原计划的六个 coordination 命令上增加 `deliver`，使“等待验收”和 writer 转移成为持久状态。
- 新增 `acceptance.md` 模板，把 main 验收/收尾从 `delivery.md` 分离；这是解决历史轮次污染和 SHA 自引用的直接实现。
- 扩展现有链接/cloud-only 合同，而不是新建门禁；用于持续校验文档链接和 `gkd-` skill 前缀。

## 验证

| 类型 | 命令或检查 | 结果 | 说明 |
|---|---|---|---|
| 本地 | `task.py status ... --json`、`task.py doctor ...` | 通过 | 当前路径、branch、writer、base 和 planning commit 一致。 |
| 本地 | `node scripts/check-cloud-only-verification.mjs` | 通过 | cloud-only、workflow 与 role skill 合同。 |
| 本地 | `node scripts/check-cloud-only-verification.selftest.mjs` | 通过 | 包含破坏 `gkd-main` 前缀的失败用例。 |
| 本地 | 变更 Node 文件 `node --check` | 通过 | 两个变更 `.mjs` 语法有效。 |
| 本地 | `node scripts/check-spec-links.mjs` | 通过 | 覆盖 spec、`docs/`、`.agents/skills/`。 |
| 本地 | `git diff --check <base>...HEAD` | 通过 | 检查整条任务分支，不只检查未提交 diff。 |
| 独立审查 | 两路 fresh read-only explorer | 通过（整改后） | CLI 6 项、文档 3 项已在提交 `0ec1e762` 修复。 |
| GitHub | `ci-gate` / `pr-title` / 适用 jobs | 等待 | 以最终 `deliver` 状态提交为准。 |

## 合同与影响

- 测试：新增 active-task、coordination、start/archive 回归测试；CI contracts job 运行全部 `.trellis/scripts/tests/test_*.py`。
- 现行文档与机器合同：根规则、Trellis workflow、运维入口、任务索引、四专题、四模板和 source/link contracts 已同步。
- API、兼容性与迁移：无产品 API 或数据迁移；旧任务缺少 coordination 时保持只读兼容，不批量重写归档历史。
- 数据、配置、安全与隐私：只写任务元数据；不缓存 CI，不增加凭据或外部服务。
- 发布与回滚：各阶段为独立提交，可按 CLI、skills、文档拆分分别回退。

## 风险与审查重点

- 剩余风险：Python unit tests 按仓库规则只在 GitHub Actions 执行；最终结论取决于 PR 最终 head 的 CI。
- main 重点审查：`task_coordination.py` 的 start/block/deliver/resume 转换；`.trellis/workflow.md` 的 breadcrumb/step 解析兼容性；CI scope 与 contracts 接线。
- 未完成项：无功能范围遗留；仅等待最终 CI 和 main 验收。

## 阻塞快照

无。
