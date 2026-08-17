# 交付报告：Codex 配置生命周期与 372K 开关

> 只记录实际实现和证据。PR head、base 与 CI URL 以 GitHub 和 `task.py status` 的实时结果为准，不在本文缓存。

## 结果

- 结果：等待验收（`deliver` 状态提交后的最终 head 检查以 PR 实时状态为准）
- PR：https://github.com/KNaiFen/aio-coding-hub/pull/158
- 执行者：`codex-372k-executor`
- 一句话结果：Codex 设置页新增后端权威的 372K 开关，配置、代理、MCP、目录与退出/恢复现在通过同一 canonical/live 生命周期写入边界。

## 实际实现

### 用户可见行为

- Codex 设置区新增 `开启上下文 372K`；加载或保存期间禁用，并沿用现有成功/失败反馈。
- 开启后，AIO 从完整用户目录或 Codex bundled 目录生成一个自有目录，只把三个固定模型的 `context_window` 与 `max_context_window` 改为 `372000`。
- 关闭后，无 profile 时恢复原目录指针或缺省状态；仍有 profile 时保留同一自有目录并仅移除 372K 变换。

### 内部机制

- direct/canonical 配置与 proxy/live 投影分离；structured、raw TOML 与 Codex MCP 写入都先更新 canonical，再生成 live，禁止从已投影 live 刷新 direct backup。
- Codex 生命周期共享一把锁和退出准入门，使用有界 journal、pre-image、哈希和 compare-before-rollback 串联 backup、live、catalog、manifest 与 preference。
- 目录 owner 统一组合 base、372K policy 与 managed profiles；用户目录优先，否则以独立参数执行 `debug models --bundled`，输入或所有权不可信时 fail closed。

## AC 证据

| AC | 结果 | 证据 |
|---|---|---|
| AC1 | 通过 | `cli_proxy_startup_recovery::codex_config_updates_are_preserved_when_cli_proxy_enabled`；`canonical_config_bytes_locked`、`project_codex_config_if_enabled` 与 merge-restore 覆盖 direct/provider/auth/catalog/provider-table 恢复。 |
| AC2 | 通过 | `codex_config_toml_set_raw` 与 `mcp_sync::sync_codex_cli_locked` 都进入 `apply_canonical_bytes_with_completion_inner_locked`；Codex MCP manifest 与 canonical transaction 同步提交/回滚。 |
| AC3 | 通过 | `lifecycle_coordinator_serializes_writers_without_timing_assumptions` 和 config-import/profile barrier tests；config、MCP、proxy、catalog/372K 与 cleanup 均使用同一 lifecycle lock/gate。 |
| AC4 | 通过 | `canonical_config_journal_recovers_each_persisted_phase` 覆盖 persisted phases；backup snapshot/refresh、catalog-policy failpoint 与 ownership-aware rollback 测试覆盖部分写入。 |
| AC5 | 通过 | `recover_interrupted_lifecycle`、`rollback_canonical_files`、profile external-replacement tests 与 cleanup shutdown gate 保留外部 drift 并拒绝退出后的新写入。 |
| AC6 | 通过 | `CodexTab.test.tsx` 的 authoritative state、loading/saving disablement；`cliManager.test.tsx` 的统一 mutation scope；service tests 覆盖 boolean IPC 映射。 |
| AC7 | 通过 | `context_window_372k_changes_exactly_six_target_values` 对结构化前后 diff 断言恰好六个叶值，均为 `372000`。 |
| AC8 | 通过 | 同一 pure test 深比较 percent、未知字段、顺序、非目标模型与 auto-compaction 字段；集成 fixture 保留根和模型级非目标值。 |
| AC9 | 通过 | `context_window_372k_is_idempotent_and_composes_with_proxy_startup_and_exit` 覆盖用户目录及 `models_cache.json` sentinel；`bundled_catalog_runs_cmd_wrapper_from_a_path_with_spaces` 覆盖结构化 bundled 调用。 |
| AC10 | 通过 | missing/duplicate target pure tests、bounded JSONL/size tests、catalog drift/alias/profile ownership tests及 symlink/path tests均断言失败不留下部分状态。 |
| AC11 | 通过 | 372K 集成测试断言关闭后恢复原 pointer；`enabled_proxy_projects_profiles_into_picker_catalog_and_restores_on_delete` 覆盖共享 profile catalog。 |
| AC12 | 通过 | `context_window_372k_is_idempotent_and_composes_with_proxy_startup_and_exit` 覆盖重复切换、proxy startup/exit、pointer 与 cache；managed-profile tests 覆盖零/一/多 policy 组合。 |
| AC13 | 通过 | CodexTab 372K 测试明确断言 `refreshCodex` 未调用；后端 mutation 只写 preference/config/catalog，不启动 Codex、请求模型、重扫或写 auto-compaction。 |
| AC14 | 通过 | temp-home 集成、backup-rel/symlink、Codex-home symlink、bounded read/write 与 external-replacement tests；错误只记录有界原因，不记录配置/目录内容。 |
| AC15 | 通过 | 两份 Codex cross-layer spec、checked-in bindings、service/query/page/component 与对应 tests 同步；本文不声明运行时采用 372K。 |
| AC16 | 本地通过；最终云端状态实时确认 | 固定 local runner 对登记 base 返回 `local_ready`；实现候选的 `frontend`、`rust`、`contracts`、`ci-gate`、`pr-title` 与 CodeQL 已通过，状态转换后重新等待最终 head。 |

## 关键位置

| 文件或符号 | 实际变化 | 设计原因 |
|---|---|---|
| `src-tauri/src/infra/codex_config/mod.rs:apply_canonical_bytes_with_completion_inner_locked` | 统一 canonical/live journal、backup、catalog、provider sync 与 rollback。 | 防止投影污染 direct backup，并让中断可恢复。 |
| `src-tauri/src/infra/cli_proxy/{mod,codex}.rs` | 读取 canonical backup、生成 `aio`/`OpenAI` live 投影并精确 merge-restore。 | proxy-only 字段不得成为用户基线。 |
| `src-tauri/src/infra/codex_model_catalog/managed.rs:context_window_372k_set` | 组合 catalog policies、持久化开关、验证 owner 与恢复 pointer。 | 保持一个完整目录 owner 和原子开关语义。 |
| `src-tauri/src/infra/codex_model_catalog/managed.rs:apply_context_window_372k` | 对三个固定 slug 执行六值结构化变换。 | 后端固定拥有目标和值，保留其他 JSON。 |
| `src-tauri/src/infra/mcp_sync/sync.rs:sync_codex_cli_locked` | MCP 变更加入同一 Codex transaction 与 manifest completion。 | 避免 MCP 旁路写 live 配置或重入生命周期锁。 |
| `src-tauri/src/domain/codex_managed_profiles.rs`、`src-tauri/src/app/cleanup.rs` | 共享写者锁、退出 gate 与 profile 文件所有权保护。 | 固定锁序并让退出恢复 direct 语义。 |
| `src/components/cli-manager/tabs/CodexTab.tsx`、`src/query/cliManager.ts` | 展示权威开关并把所有 Codex 写入串行化。 | UI 不自行持有 slug、路径或运行时证明。 |

## 计划偏移

- 无产品范围偏移。CI 回归额外暴露并修复了 lifecycle lock 重入、事务中重复初始化数据库、remote-compaction canonical 污染及旧测试目录夹具前置条件。
- 合并最新 `origin/main` 后，移除了本任务早期规划提交携带、但主线已归档的 TUI 活动任务副本；archive 内容保持不变，PR 不再重新引入非任务文件。

## 验证

| 类型 | 命令或检查 | 结果 | 说明 |
|---|---|---|---|
| 本地 | `node scripts/check-local-verification.mjs --base <task.py status 登记的完整 SHA>` | 通过 | 最新代码与主线集成后返回 `local_ready`；按仓库合同未本地运行 Cargo、formatter、generator、依赖安装、前端测试或构建。 |
| GitHub | `frontend`、`rust`、`contracts`、`ci-gate`、`pr-title`、CodeQL | 实现候选通过；最终 head 待状态转换后确认 | Rust cloud job 执行 fmt/bindings、Clippy、完整测试与 audit；release candidate jobs 按 PR 范围跳过。 |
| 人工 | 实际 Codex 使用 372K | 未运行 | PRD 明确排除 runtime refresh/restart/request/rescan；用户后续实测不属于 artifact/state 验收。 |

## 合同与影响

- 测试：新增 `src-tauri/tests/codex_context_window_372k.rs`，扩展 proxy/config/catalog/profile/settings Rust tests，以及 component/page/query/service 前端 tests。
- 现行文档与机器合同：更新 `codex-config-contract.md`、`codex-managed-model-route-contract.md`、settings schema/migration 与 generated bindings。
- API、兼容性与迁移：新增两个 boolean/state IPC；现有 IPC 不变。设置迁移只增加默认 `false` 的隐藏字段，无破坏性数据迁移。
- 数据、配置、安全与隐私：新增 AIO-owned catalog 与有界 lifecycle journal；继续执行 absolute path、size、symlink/reparse、atomic write、drift-wins 和无敏感内容日志约束。
- 发布与回滚：本任务不改版本或发布配置。需要回退已启用环境时，应先用当前版本关闭 372K 以恢复 pointer/owned catalog，再回退代码。

## 风险与审查重点

- 剩余风险：实际 Codex 版本是否采用生成目录中的 372K 值未验证；bundled catalog schema 若变化会按设计 fail closed。
- main 重点审查：`apply_canonical_bytes_with_completion_inner_locked` 的写入/补偿顺序、`recover_interrupted_lifecycle` 的 phase 判定、`prepare_for_profiles_with_policy` 的 owner/pointer 恢复，以及 proxy 开启时 remote-compaction 的 canonical/live 分离。
- 未完成项：无任务内实现项；最终 head CI 与固定 head 验收由实时 PR、`$gkd-ci-monitor` 和 `$gkd-accept` 完成。

## 阻塞快照

无。
