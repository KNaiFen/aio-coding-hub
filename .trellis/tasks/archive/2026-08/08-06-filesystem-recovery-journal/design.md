# AUD-002 技术设计

## Journal 与状态机

新增外部副作用 recovery journal，记录 operation kind、CLI、workspace/entity 标识、受管 artifact 引用与 hash、状态、尝试次数和定长错误分类。journal 先以独立事务提交为 `prepared`，外部投影完成后记录诊断阶段，业务事务提交后再以当前 DB 重放并收敛，最终标记 resolved 或清除。所有状态转换必须幂等。

## 操作协议

Prompt、MCP 和 Skills 的写入口统一采用 prepare -> external effect -> business commit -> authoritative projection -> resolve。任何阶段失败都保留 journal。同步返回 primary error，并聚合 compensation failure；不能再使用 `let _ = restore(...)`。workspace switch 使用一个父 operation 关联分阶段投影，使 replay 只暴露最终已提交的 active workspace。

## Skills 受管制品

Skills 的目录内容不完整存在 SQLite。涉及替换或删除 SSOT 的操作，在副作用前把所需字节放入 journal 专属受管 staging/backup，并记录规范化相对引用和内容 hash。恢复器拒绝 symlink、越界、所有权 marker 缺失或 hash 不匹配的 artifact；resolved 后再回收，不扫描或删除非受管目录。

## 启动与维护态

DB 初始化完成后立即扫描 pending journal，并在任何 observer、gateway、日志留存、usage backfill 或前端后台任务之前阻断 replay。复用 AUD-008 maintenance coordinator 的 running/failed/retry/exit gate；reset marker 优先，因为 reset 会销毁 journal 的 DB 权威源。普通写 IPC 必须通过同一 gate。

## 脱敏

journal 不保存正文、env/header JSON、认证值、原始错误或任意绝对路径。错误归一为稳定 operation/error code，再对 token、key、secret、password、Authorization/Bearer 模式做兜底清洗和长度限制。
