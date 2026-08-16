# Development Workflow

本文件是 Trellis 的精简状态与 CLI 参考。角色权限和本地执行边界以根 `AGENTS.md` 为准；多 worktree 的具体阶段流程由 `$gkd-main`、`$gkd-execute`、`$gkd-accept` 按需加载。

## 核心原则

1. 实施前把用户决定、范围和 AC 写入仓库文件。
2. `task.json` 保存机器协调事实，任务 Markdown 保存人类决定和结果，Git/GitHub 保存提交、PR 和 CI 实况。
3. 只加载当前角色、当前阶段需要的文档，不把整套工作流塞进每个 session。
4. 当前代码和机器合同高于现行规范，现行规范高于任务记录，历史记录只解释过去。

## Trellis 系统

### 身份与上下文

首次使用可初始化开发者身份：

```bash
python3 .trellis/scripts/init_developer.py <name>
```

`.trellis/spec/` 保存包和层级规范；`.trellis/workspace/` 保存 session 日志。按需读取：

```bash
python3 .trellis/scripts/get_context.py --mode packages
python3 .trellis/scripts/get_context.py --mode phase --step <X.Y>
python3 .trellis/scripts/add_session.py --title <title> --commit <sha> --summary <summary>
```

### 任务文件

- `task.json`：状态、路由、唯一写者和登记的 Git/worktree 事实；只通过 `task.py` 修改协调字段。
- `prd.md`：需求、确认状态、范围和 AC。
- `design.md`、`implement.md`：复杂任务的设计与有序实施步骤。
- `execution.md`：独立执行 session 的任务特有入口。
- `delivery.md`：执行者实际实现、AC 证据、偏移、验证和风险。
- `findings.md`：main 验收不通过时创建的当前整改清单。
- `acceptance.md`：main 在终态汇总验收、merge、归档和清理证据。
- `implement.jsonl`、`check.jsonl`：可选的 spec/research 路径清单，不替代计划或交付文档。

轻量 main 任务可以只用 `prd.md`；独立 worktree 在开工前还要有 `execution.md`，交付前有 `delivery.md`；复杂任务再增加 `design.md` 和 `implement.md`。完整文件职责见 `docs/operations/multi-worktree-delivery.md`。

### CLI

```bash
# 创建、选择和查看
python3 .trellis/scripts/task.py create "<title>" [--slug <name>] [--base-branch main] [--no-start]
python3 .trellis/scripts/task.py start <task> [--writer <id>]
python3 .trellis/scripts/task.py current --source
python3 .trellis/scripts/task.py finish
python3 .trellis/scripts/task.py list [--mine] [--status <status>]
python3 .trellis/scripts/task.py list-archive [YYYY-MM]

# 持久化协调
python3 .trellis/scripts/task.py status [task] [--json]
python3 .trellis/scripts/task.py doctor [task]
python3 .trellis/scripts/task.py delegate <task> --worktree <path> --branch <branch> --base-sha <sha> --planning-commit <sha> --writer <id>
python3 .trellis/scripts/task.py handoff [task] [--json]
python3 .trellis/scripts/task.py deliver [task]
python3 .trellis/scripts/task.py accept .trellis/tasks/<task> --worktree <absolute-candidate-worktree> --pr <number> --head <full-pr-head-sha>
python3 .trellis/scripts/task.py block <task> --reason <text> --resume-condition <text> --owner <id>
python3 .trellis/scripts/task.py resume <task> --writer <id>

# 可选上下文与元数据
python3 .trellis/scripts/task.py add-context <task> implement|check <path> [reason]
python3 .trellis/scripts/task.py list-context <task>
python3 .trellis/scripts/task.py validate <task>|--all
python3 .trellis/scripts/task.py set-branch <task> <branch>
python3 .trellis/scripts/task.py set-base-branch <task> <branch>
python3 .trellis/scripts/task.py set-scope <task> <scope>
python3 .trellis/scripts/task.py add-subtask <parent> <child>
python3 .trellis/scripts/task.py remove-subtask <parent> <child>

# main-only 终态归档
python3 .trellis/scripts/task.py archive --no-commit <task>
```

`task.py --help` 是命令列表的权威来源。`create` 默认 `base_branch=main`，不会从当前 feature branch 推断 PR base；`task.py` 没有 create-pr 命令。

`validate` 只检查已经存在的 JSONL 是否可解析及引用路径是否存在。缺失或只有 seed 行的 manifest 可以被兼容性逻辑跳过；成功不证明 PRD、交付、CI、Markdown 或归档资格。

### 状态与当前任务

顶层 `task.json.status` 只用于 Trellis 粗粒度生命周期：

```text
planning -> in_progress -> completed
```

带 `coordination.version=1` 的任务用 `coordination.phase` 记录细分状态；委派路线会经过 `ready/delivered`，main 连续路线从 `planning` 直接进入 `implementing`：

```text
planning -> ready -> implementing -> delivered -> completed
                          |              |
                          +-> blocked <-+
```

`create/start` 在能识别 session 时写 `.trellis/.runtime/sessions/` 指针；没有 session identity 时，`start` 仍持久化任务状态但不创建指针。可读且明确指向已删除或已完成任务的 stale pointer 会被清理；损坏 JSON 和多个有效 session 不会被猜测处理。无法可靠解析当前任务时显式传 `<task>`。

`start` 把顶层 `planning` 改为 `in_progress`；委派任务还会把 `ready` 改为 `implementing`。返工从 `delivered` 启动时必须传 `--writer`。`deliver` 要求实现和 `delivery.md` 已先提交、worktree 干净，然后把 phase 改为 `delivered` 并把 writer 固定交给 `main`。

`archive` 会写 `completed`、移动目录、重写上下文路径并清理 session pointer。它不是事务性的；失败后先检查实际目录、manifest 和 Git 状态，不盲目重跑。业务终态以 `acceptance.md` 为准。

## Phase Index

```text
Phase 1: Plan    -> 记录决定、研究、任务材料和实施授权
Phase 2: Execute -> main 直接施工，或按 execution.md 在独立 worktree 施工
Phase 3: Finish  -> 验证、沉淀知识、提交；委派任务随后固定 head 验收并由 main 收尾
```

### 请求路由

- 简单、低风险且由 main 连续完成：使用 `$gkd-main` 和月度 change record；无需为形式创建 Trellis task。
- 复杂、委派、并行、长流程或高风险：使用 `$gkd-main` 创建任务和规划；创建任务不等于授权实施。
- 独立执行窗口：只使用 `$gkd-execute`，以 `execution.md` 和登记 writer 为授权。
- 固定 head 验收与同步合并：使用 `$gkd-accept`；普通探索 subagent 仍只读，新开顶层窗口不会因自定义 agent 配置自动获得执行角色。

[workflow-state:no_task]
没有活动任务。先判断是直接 main 小任务还是需要 Trellis 的复杂/委派任务；实施前把确认的方案写入对应仓库记录。使用 `$gkd-main` 获取当前角色流程。
[/workflow-state:no_task]

[workflow-state:planning]
任务仍在规划。完成 prd.md 的授权、范围和 AC；委派任务再完成 execution.md，复杂任务再完成 design.md/implement.md。材料性问题未关闭时不要 start。使用 `$gkd-main`。
[/workflow-state:planning]

[workflow-state:planning-inline]
任务仍在规划。完成 prd.md 的授权、范围和 AC；复杂任务补 design.md/implement.md。材料性问题未关闭时不要 start。使用 `$gkd-main`。
[/workflow-state:planning-inline]

[workflow-state:in_progress]
先运行 task.py status 查看 coordination.phase 和 writer。main 使用 `$gkd-main`；登记的独立执行 writer 使用 `$gkd-execute`；固定 head 验收使用 `$gkd-accept`。不要从顶层 in_progress 猜测当前是施工、阻塞还是验收。
[/workflow-state:in_progress]

[workflow-state:in_progress-inline]
先运行 task.py status 查看 coordination.phase 和 writer。main 使用 `$gkd-main`；登记的独立执行 writer 使用 `$gkd-execute`。不要从顶层 in_progress 猜测当前阶段。
[/workflow-state:in_progress-inline]

## Phase 1: Plan

#### 1.0 Create task `[required · once]`

仅在任务确实需要 Trellis 时创建。`--slug` 不含日期前缀；`create` 自动添加 `MM-DD-`。使用 `--no-start` 保持规划态，避免把创建误当实施授权。

```bash
python3 .trellis/scripts/task.py create "<title>" --slug <name> --base-branch main --no-start
```

多交付物可使用 parent/child；树结构不表示依赖，依赖顺序写入子任务材料。

#### 1.1 Requirement exploration `[required · repeatable]`

查清代码可回答的事实，向用户确认产品行为、优先级、取舍和成功标准。把实施授权、范围、非目标、锁定决定、AC、事实/假设/未决问题写入 `prd.md`。规划前完整读取 `PENDING.md` 的未解决条目。

委派任务按 `docs/operations/multi-worktree/planning-and-handoff.md` 准备 `execution.md`；复杂任务把技术设计和步骤分到 `design.md`、`implement.md`。不要复制同一详细方案。

#### 1.2 Research `[optional · repeatable]`

研究问题要具体。代码库探索、外部 API 和第三方约束的可复用结论写入 `research/<topic>.md`；临时搜索日志不落盘。研究可以并行，但用户决定和最终方案由 main 完成。

#### 1.3 Configure context `[optional · once]`

只有实际消费者需要时才维护 `implement.jsonl`、`check.jsonl`。每行格式为 `{"file":"<repo-relative-path>","reason":"<why>"}`，只放 spec/research，不放代码或待修改文件。优先用 `task.py add-context`，不要手写 JSONL。

这些 manifest 是按需上下文路由，不是任务 ready 门禁；`task.py validate` 也不验证任务完整性。

#### 1.4 Activate task `[required · once]`

用户已授权实施、材料性问题关闭且任务材料已提交后才启动。main 连续施工可直接 `start`；委派任务先按 planning 专题创建和登记 worktree，再由 main 提交 `ready -> implementing` 转换并生成 handoff。

```bash
python3 .trellis/scripts/task.py start <task>
```

返工恢复用 `task.py start <task> --writer <execution-session>`；阻塞恢复用 `task.py resume`。

#### 1.5 Completion criteria

- `prd.md` 已记录实施授权、范围、锁定决定和可判定 AC。
- 材料性未决问题已关闭，规划材料已提交。
- 委派任务有 `execution.md`、完整 base/planning SHA、登记 worktree/branch/writer，且 `doctor` 通过。
- 复杂任务有 `design.md`、`implement.md`；轻量 main 任务可以没有。

## Phase 2: Execute

#### 2.1 Implement `[required · repeatable]`

main 连续施工遵循当前任务记录和适用 spec。独立执行 session 只读 `execution.md` 指向的任务材料和 `docs/operations/multi-worktree/execution-and-delivery.md`，维护自己的分支、PR、CI 与 `delivery.md`，不得再派实现链。

所有角色遵守 `AGENTS.md` 的本地 cloud-only 边界。范围、用户决定、公共接口、兼容性、安全或迁移需要改变时停止并持久化 blocker。

#### 2.2 Quality check `[required · repeatable]`

检查完整任务范围、AC、适用 spec、回归风险、测试、文档和 `git diff --check`。只运行 `AGENTS.md` 明确允许的本地命令；依赖和原生检查由 GitHub Actions 承担。

执行 session 在实现和 `delivery.md` 先提交后运行 `task.py deliver`，提交状态转换，推送并等待最终 head 的适用 CI，然后暂停。`$gkd-accept` 的固定 head 验收是独立步骤，不由执行者自审代替；通过后从干净且已同步的可信 main checkout 调用 `task.py accept` 同步合并。

#### 2.3 Rollback `[on demand]`

需求或设计有缺陷时回到 Phase 1 更新原材料并取得所需确认；实现错误时回退到最后安全提交；需要更多研究时把可复用结论写入 `research/`。不要静默扩大范围。

## Phase 3: Finish

#### 3.2 Debug retrospective `[on demand]`

同一问题重复修复时，记录根因、失败方案、新证据和防复发措施。只把长期有效的结论写入 spec/知识库，不保留完整调试流水账。

#### 3.3 Spec update `[required · once]`

判断本任务是否产生新的产品、架构、API、运维或防复发知识。需要时更新 `.trellis/spec/` 或 `docs/`；没有长期知识时记录“不需要”即可，不为过流程制造文档。

#### 3.4 Commit changes `[required · once]`

检查 dirty state，只提交本任务且归属明确的文件；按仓库历史拆分语义提交，不 amend，不混入未知修改。独立执行 session 可推送任务分支但不推 `main`；main 是否推送、建 PR 或合并按 `$gkd-main` 和用户授权执行。

#### 3.5 Wrap-up reminder

main 连续任务在同一 change record 补实际结果。委派任务由执行 session 在 `delivered` 后暂停；`$gkd-accept` 或 main 按 acceptance 专题验收，并从可信 main checkout 通过 `task.py accept` 同步合并。main 随后写 `acceptance.md`、archive 和清理。`task.py validate --all` 只验证已有 JSONL，不能替代这些判断。

## 解析合同

`get_context.py --mode phase` 依赖 `## Phase Index` 和 `#### X.Y` 标题；workflow-state 解析器只识别闭合的 `[workflow-state:STATUS]` block，STATUS 字符集为 `[A-Za-z0-9_-]+`。新增状态必须先有实际 writer/reader；不要只写一个永远不可达的 breadcrumb。

现行实现入口：

- `.trellis/scripts/common/workflow_phase.py`：Phase Index 与 step 解析。
- `.trellis/scripts/task.py`：命令路由和 start/finish。
- `.trellis/scripts/common/task_coordination.py`：协调状态、校验和交接输出。
- `.trellis/scripts/common/task_acceptance.py`：固定 head 验收后的 PR 重验与同步合并。
- `.trellis/scripts/common/task_store.py`：create/archive。
- `.trellis/spec/aio-coding-hub/cross-layer/trellis-task-context-archive-contract.md`：JSONL 归档路径合同。
