# AUD-008 跨重启数据重置

## Goal

把数据重置改为持久 marker 驱动的跨重启维护流程，彻底避免活跃 DB/background owner 下的部分删除。

## Requirements

- reset IPC 只持久写入 marker 并立即走不会重开 DB 的退出路径。
- 下次启动在 logging、数据库、observer、gateway 和任何后台任务之前消费 marker。
- 删除失败时保留 marker，进入阻断式维护态；用户只能重试或退出，不能继续使用部分清理状态。
- reset 成功后删除 marker，并以默认配置完成首次正常启动。
- 维护态作为应用级 gate，供 AUD-002 recovery journal 复用。
- reset 与 recovery journal 同时存在时，reset 优先并使旧 journal 失效。

## Acceptance Criteria

- [ ] marker 原子持久化、重复请求幂等，写入后当前进程不再初始化或写入 DB。
- [ ] 启动前 gate 明确早于 DB、observer、retention、gateway、usage backfill 和前端后台任务。
- [ ] 任一 settings/SQLite/WAL/SHM 删除失败均保留 marker，重试可完成，期间产品功能不可用。
- [ ] 成功后 marker 消失、数据完整为空，下一次正常启动只创建一套 DB/runtime owner。
- [ ] UI 准确描述“登记重置并退出”“维护态重试/退出”，不再声称同进程已完成清理。
- [ ] Windows/macOS/Linux 云端测试覆盖 marker 生命周期、失败重试和首次正常启动。

## Notes

- marker 写入后的退出清理不得调用 `ensure_db_ready`；关联 `AIO-PENDING-020`。
