# AUD-055 技术设计

## 格式与所有权

引入显式 v2 session-only manifest。清理器把目录分类为 v2 managed、v1 managed 或 unmanaged：v2 只留最新一代；v1 在首次成功建立 v2 后删除；其余目录和 symlink 一律跳过。目录名、mtime 和文件存在本身都不能证明所有权。

## 同步与回滚

变更集只枚举活动 `sessions` 的 rollout 文件。配置写入仍属于 Provider Sync 的主操作；备份和 snapshot 只覆盖本轮确实会改变的配置/session 文件。所有外部写入沿用先快照、再写、失败恢复的顺序。

## 兼容

公开结果 DTO 尽量保留字段，移除已不执行的 SQLite/global-state 语义时以云端 binding drift 为准做最小同步。非受管目录保护优先于清理空间。
