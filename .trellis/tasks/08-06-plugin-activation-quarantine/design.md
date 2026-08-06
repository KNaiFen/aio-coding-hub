# AUD-033 技术设计

## 激活策略

manifest 保持 `Vec<String>` 兼容序列化，内部解析为 `Legacy` 或显式事件集合。缺失/空数组为 Legacy，只放行 manifest 已贡献的 command/hook；显式模式使用完整 command 与规范 hook 名精确匹配。只声明 `onStartup` 的插件由启动 dispatcher 激活，不能因此隐式获得 command/hook 权限。

## 持久隔离

扩展 runtime failure 记录的来源与稳定 error code。单一仓储事务负责插入严重故障、查询该 plugin 最近 600 秒计数，并在第三次时把 status 更新为 quarantined、写 last error 与 audit。并发越阈只能产生一次状态转换；非严重策略拒绝仍保留诊断但不计数。

## 运行时刷新

command、startup 与 gateway 共用严重故障分类。gateway 第三次失败先按当前 hook 的 fail-open/fail-closed 完成本请求，再刷新 enabled plugin snapshot 并释放 extension-host 实例；既有 in-flight 请求继续使用捕获的旧 snapshot，不能反向污染新版本。

## Revalidate 与迁移

新增 quarantined-only revalidate。完整复核安装和来源后只执行 `quarantined -> disabled`，保留 failure history 与 audit。升级时校验已安装 manifest：废弃显式事件转 disabled 并记录稳定原因；Legacy 不改写，避免改变 checksum/signature。

## 合同同步

Rust validator、plugin SDK 类型/校验、API 合同、文档、Tauri command、前端 query/service/lifecycle panel 同步。生成 bindings 由 CI 产出，本地不生成。
