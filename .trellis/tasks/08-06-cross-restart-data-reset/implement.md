# AUD-008 实施清单

- [ ] failure-first 覆盖现有同进程删除、后台 owner 和退出重建 DB。
- [ ] 定义 durable reset marker 与 maintenance coordinator。
- [ ] 将 coordinator 放到 bootstrap 的 DB/logging/background 前置位置。
- [ ] reset IPC 改为写 marker + 专用退出，移除同进程文件删除。
- [ ] 实现幂等清理、失败保留 marker、retry/exit 与成功清 marker。
- [ ] gate 原生 startup 和前端 startup/background tasks。
- [ ] 更新 DTO、bindings、service/query/UI 文案和测试。
- [ ] 云端覆盖跨平台文件占用、marker 生命周期和首次正常启动。
- [ ] 合并后在 AUD-002 候选记录共享 gate 证据。
