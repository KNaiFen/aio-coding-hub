# AUD-008 技术设计

## 维护协调器

新增应用级 maintenance coordinator，持久输入包括 reset marker 和后续 recovery journal，内存状态为 clean/running/failed。bootstrap 在任何会打开 DB 或启动后台任务之前运行它；失败状态通过启动状态 IPC 投影，但不允许进入普通 startup。

## Reset 协议

IPC 校验确认后使用原子文件 helper 写 durable marker，再走专用退出路径。该路径停止必要资源但禁止 reconciliation 或其他 `ensure_db_ready` 调用。下次进程首先删除 settings 与 SQLite sidecars；全部成功后才清 marker。

## 失败语义

逐文件删除保持幂等。任一失败不清 marker、不开始 DB 初始化，维护 UI 提供 retry/exit。重试重新执行完整目标集合。reset 优先于 recovery journal，因为权威 DB 本身将被销毁。

## 前端 gate

启动状态增加明确 maintenance 投影；前端 startup/background tasks 只有 clean/ready 后才能启动。普通 query 不在维护失败时发起。
