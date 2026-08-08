# AUD-055 技术设计

## 格式与所有权

引入显式 v2 session-only manifest。清理器把目录分类为 v2 managed、v1 managed 或 unmanaged：v2 只留最新一代；v1 在首次成功建立 v2 后删除；其余目录和 symlink 一律跳过。目录名、mtime 和文件存在本身都不能证明所有权。

## 同步与回滚

变更集只枚举活动 `sessions` 的 rollout 文件。配置写入仍属于 Provider Sync 的主操作；备份和 snapshot 只覆盖本轮确实会改变的配置/session 文件。所有外部写入沿用先快照、再写、失败恢复的顺序。

## 兼容

公开结果 DTO 尽量保留字段，移除已不执行的 SQLite/global-state 语义时以云端 binding drift 为准做最小同步。非受管目录保护优先于清理空间。

## 清理安全边界

清理从可信根句柄相对枚举、分类和隔离，子项以 no-follow 方式打开并绑定文件身份。普通文件使用 64 KiB 分块的 SHA-256 内容摘要，单文件、单次 prune 的总读取/哈希、深度、条目和详细 warning 都有上限；预算耗尽、身份/内容变化或平台能力不足均 fail closed，保留候选或隔离数据。

Unix 使用双 tombstone、句柄相对重命名/枚举/删除，Windows 使用相对句柄打开、ChangeTime、受检目录记录解析和删除 disposition 前复验。这些措施只缩小从末次复验到最终删除 syscall 的竞态窗口。对于能够以同 UID 或同等权限并发写入父目录/已打开文件的恶意进程，POSIX 与 Windows 都没有可移植的“按已验证身份原子删除”保证；实现、测试和文档不得把残余窗口描述为彻底消除。
