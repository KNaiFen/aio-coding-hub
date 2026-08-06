# AUD-056 技术设计

## 设置迁移

升级 settings schema，将请求日志默认/最小值统一为 7/1 天。迁移只把历史有效配置中的 0 改为 7；字段名和 IPC 形态保持不变。运行日志继续复用现有 retention 字段。

## 数据保留

请求日志清理保留现有“usage ledger 覆盖完成后才能删除”的门禁。原始 request row 可删除，ledger 与聚合表不参与 retention。

运行日志只枚举 `tracing-appender` 产生的精确 UTC 日滚动文件名，排除当日活动文件，先按天龄、再按总字节从最旧关闭文件回收，256 MiB 为软上限。

## 可回收空间

`DbDiskUsage` 增加 `freelist_count * page_size` 计算的 SQLite 可回收字节。清空路径只 checkpoint，不自动 VACUUM；显式 `db_compact` 保持现有职责。
