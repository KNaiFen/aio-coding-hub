# 熔断恢复探测接入

## Plan Status

- Implementation authorization: confirmed
- Confirmation date and summary: 2026-08-11；用户确认本任务创建独立 worktree，由单独执行窗口按本任务材料施工。
- Confirmed coverage: 手动与定时可用性探测均可作为 HalfOpen 恢复证据；不缩短 Open 时长；HalfOpen 探测失败立即重新 Open；定时成功且写入后仍为 HalfOpen 时，30 秒后补测一次；Gateway 未运行时只记录可用性观测。
- Planning revision: pending initial planning commit; the coordinator will freeze the full SHA before the execution session starts.
- Execution route: delegated worktree
- Migrated from direct-main record: none

## Material Facts, Assumptions, and Open Questions

| Item | Source | Status / closure condition |
|---|---|---|
| 熔断器只在 `should_allow` 发现 Open 已到期时转为 HalfOpen；`record_success` 只在 HalfOpen 累计成功，`record_failure` 在 HalfOpen 立即重新 Open。 | 现有 `shared/circuit_breaker` 状态机 | confirmed |
| 手动和定时探测最终共同汇入 `ProviderAvailabilityProbeRuntimeState::finish_probe`，并有同 Provider 的 in-flight 合并与 generation 失效保护。 | 现有 probe runtime | confirmed |
| 当前 Gateway 运行时持有唯一的持久化 `CircuitBreaker`；Gateway 停止时没有可安全写入的活跃实例。 | 现有 Gateway runtime / 用户选择 | confirmed |
| 定时补测必须属于现有调度器生命周期，不能用脱离配置失效保护的裸 `sleep(30s)`。 | 现有调度器 / 用户需求 | confirmed |
| `AIO-PENDING-029` 只涉及 `upgrade-tui.command`，用户已锁定本任务不得读取、执行、修改、移动或删除该文件。 | `PENDING.md` | confirmed, out of scope |
| 协调 `main` 有上述未跟踪文件；本任务从干净的 `origin/main` SHA `9b05b28d5841584dc6f2a867947afd5d23f76246` 创建，未从脏工作树派生。 | Git 基线核验 | confirmed |

没有材料性未决问题。任何影响用户行为、兼容性、范围或 AC 的变化必须先写回本文件并重新确认。

## Goal

将手动与定时可用性探测接入 HalfOpen 熔断恢复，并在定时成功后安排 30 秒恢复探测。

## Requirements

- R-01: 保持现有 Open 冷却时长。探测在 Open 尚未到期时成功，不得提前进入 HalfOpen 或关闭熔断器。
- R-02: 对当前 generation 的有效可用性探测结果，在运行中的 Gateway 中按结果完成时刻检查熔断器；已到期 Open 可进入 HalfOpen，手动和定时成功均可累计现有 HalfOpen 成功次数。
- R-03: HalfOpen 中的手动或定时失败结果必须调用既有失败路径，立即重新 Open 并清零恢复进度；无明确 `ProviderAvailabilityResult` 的内部错误不作为熔断证据。
- R-04: 仅定时探测在成功完成、熔断结果已写入且状态仍为 HalfOpen 时，安排同 Provider 的一次恢复补测，名义触发时刻为成功完成后 30 秒。
- R-05: 补测必须复用 Provider generation、同 Provider in-flight 合并、全局定时探测并发上限和过期任务丢弃机制；配置禁用、删除或 revision 变化必须取消补测。
- R-06: 同一实际 HTTP probe 无论被多少手动/定时调用者合并，都只累计一次熔断恢复证据；补测使用独立 trace ID，确保两次可用性观测均可落库。
- R-07: 不改变 Tauri IPC、可用性结果结构、设置、数据库 schema、生成绑定或前端合同；继续使用既有 `gateway:circuit` 事件。
- R-08: Gateway 未运行时，探测仍按原行为记录可用性结果，但不尝试创建第二套熔断器或修改离线持久化状态。

## Acceptance Criteria

- [ ] AC-01: 已 Open 但未到期时，手动或定时成功只写可用性观测，熔断状态和 HalfOpen 成功计数不变。
- [ ] AC-02: Open 到期后的有效手动或定时探测可进入 HalfOpen；三次连续成功（沿用现有阈值）后状态为 Closed，并发出既有恢复事件。
- [ ] AC-03: HalfOpen 中的有效失败结果立即使状态回到 Open，且之后不保留成功计数或补测链。
- [ ] AC-04: 定时探测成功后，只有结果写入后仍为 HalfOpen 才在约 30 秒后产生一次补测；使其 Closed 的第三次成功不会再排队。
- [ ] AC-05: 同 Provider 合并、重复 scheduler tick、禁用、删除、revision 更新、应用休眠恢复和状态在等待期间改为 Closed 时，都不会产生重复、过期或无用的补测 HTTP 请求。
- [ ] AC-06: Gateway 停止时无熔断状态副作用；现有可用性 IPC/前端合同与数据迁移保持不变。
- [ ] AC-07: 本任务分支通过仓库要求的 GitHub Actions 后端测试、编译和 `ci-gate`；交付记录绑定最新 PR head SHA 与运行链接。

## Scope and Decision Changes

| Date | Old / new decision | Affected acceptance criteria | Decision owner / resume condition |
|---|---|---|---|
| 2026-08-11 | Initial plan: freeze the user-confirmed recovery semantics and delegated-worktree route. | All ACs | user / execution resumes after planning revision is committed |

## PENDING Review

- `AIO-PENDING-029`: remains `pending` and is not included. It has no dependency on circuit recovery; its explicit prohibition on handling `upgrade-tui.command` remains in force.

## Notes

- `design.md` is the authoritative technical design, `implement.md` is the ordered implementation checklist, and `execution.md` is the delegated-session entry point.
- The execution session must stop for any material behavior, scope, or acceptance-criterion change and obtain a recorded confirmation before continuing.
