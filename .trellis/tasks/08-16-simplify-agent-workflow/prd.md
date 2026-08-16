# 精简多 worktree 多 session 工作流

## 方案状态

- 实施授权：已确认
- 确认日期：2026-08-16
- 确认摘要：采用短 `AGENTS.md`、按角色 skills、脚本化持久状态与交接、分阶段 Markdown，并修复现有文档与实现偏移。
- 实施路线：独立 worktree，由当前 session 连续实施并自检，最终仍按 PR 交付。
- 规划版本：由规划提交的完整 SHA 确定，写入 `task.json.coordination.planning_commit`。
- 材料性未决问题：无。
- PENDING 审阅：`PENDING.md` 当前没有未解决条目。

## 问题与目标

当前通用规则、Trellis workflow、交付规范和模板约 115 KB；复杂任务会把历史验收轮次、重复状态字段和通用教程一起加载到执行者上下文。branch、worktree、writer、PR、SHA 等事实还依赖多份 Markdown 手写，已经出现 stale session pointer、文档引用不存在能力、脚本能力与文档声明不一致等偏移。

目标是把跨窗口协作建立在可重建的机器事实和短入口上，使新 session 只加载当前角色、当前阶段和当前任务需要的内容。

## 锁定决定

1. `task.json` 是活动任务协调状态的唯一持久化结构化事实源；不新增第二套状态 JSON。
2. 根 `AGENTS.md` 只保留角色权限、唯一写者、Git/CI/安全硬边界和 skills 路由。
3. 仓库只提供带统一短前缀的 `gkd-main`、`gkd-execute`、`gkd-accept` 三个角色 skill；不补齐不存在的旧微型 skills。
4. `task.py` 负责状态读取、确定性 preflight、委派登记、交接文本生成、交付、阻塞和恢复；Markdown 不再手写可由 Git 或 `task.json` 得出的状态副本。
5. 多 worktree 规范拆为一个主入口和四份单层链接专题；执行模板只保存任务特有差量。
6. 保留一任务一 worktree、一任务一 PR、执行 session 不合并、main 验收固定 head 和唯一写者边界。
7. 只对本地可确定事实 fail closed；不新增自动合并、复杂审批、模型手填 checklist 或 GitHub CI 状态缓存。

## 范围

必须完成：

- 扩展任务协调状态与 CLI，修复 stale session pointer，增加无依赖回归测试。
- 建立三个仓库级 skills，并修正 `.gitignore` 使其可随 clone/worktree 分发。
- 精简 `AGENTS.md`、`.trellis/workflow.md`、多 worktree 规范、模板和导航。
- 修正文档中的不存在命令、死状态、错误默认 base、验证能力夸大和 SHA 自引用问题。

明确不做：

- 不自动创建、合并或删除 PR/worktree，不接管 GitHub 审批。
- 不把 CI 结果持久化为新的 canonical state，不实现 `acceptance.json`。
- 不批量重写已归档任务，不清理其历史审计记录。
- 不引入完整状态机框架、数据库、守护进程或复杂 hooks。

## 验收标准

- AC-01：新窗口仅凭根规则、对应 role skill、任务入口和 `task.py status/doctor` 即可判断角色、路径、分支、writer、base 和下一步。
- AC-02：`task.py delegate/handoff/deliver/block/resume` 只通过代码写入或读取 `task.json.coordination.version=1`，保留未知字段并对无效转换返回非零。
- AC-03：失效 session pointer 不再作为当前任务返回；多个有效 session 时仍拒绝猜测。
- AC-04：三个 repo skills 可被 Git 跟踪，`SKILL.md` 简短并只按需链接阶段专题或脚本。
- AC-05：`AGENTS.md` 不再承载完整生命周期教程；通用文档总量显著下降，执行 session 固定入口目标不超过 `AGENTS + skill + execution` 约 3,000 tokens。
- AC-06：多 worktree 主入口直接链接四份专题；没有更深的必读链接链。
- AC-07：`execution.md` 模板不复制 PRD/design/implement，`delivery.md` 不要求在候选分支内自引用当前 head，`findings.md` 只有一个可重复 finding schema。
- AC-08：现行文档只声明脚本真实支持的命令、状态和校验能力，所有本地 Markdown 链接有效。
- AC-09：新增与现有 Python 单元测试、skill 校验、Node source contract、`node --check` 和 `git diff --check` 全部通过。

## 停止条件

遇到需要改变产品行为、CI 权限模型、远端 `main`、现有历史任务语义或来源不明修改时停止并报告；不得以本任务为由扩大到产品代码重构。
