# AUD-055 Provider Sync 快照体积

## Goal

将 Provider Sync 快照收窄为 session-only 新格式，并把受管备份稳定限制为最新一代。

## Requirements

- 新格式不再扫描、改写或备份 `archived_sessions`。
- 备份只保留最新一代新格式 managed backup。
- 升级时自动删除能够由旧 v1 manifest 证明所有权的 managed backup。
- 无 manifest、损坏 manifest、marker 不匹配、符号链接或其他非受管目录绝不删除。
- managed backup 分类、隔离、复验和删除必须从已打开的可信根/子项句柄出发，拒绝跟随链接；普通文件使用有界流式摘要，单次 prune 共享深度、条目和哈希预算，超限或观察到变化时 fail closed 并保留隔离数据。
- 双 tombstone、句柄相对操作、内容摘要和删除前末次复验用于缩小竞态窗口。POSIX/Windows 对同 UID、同权限恶意并发写者不存在可移植的“按已验证身份原子删除”保证，文档和诊断不得宣称彻底消除该残余边界。
- 保留同步失败的完整回滚能力，不扩大到其他恢复系统。

## Acceptance Criteria

- [ ] 普通同步只修改活动 `sessions`，归档会话字节保持不变。
- [ ] 成功同步后最多存在一个新格式 managed backup。
- [ ] v1 managed backup 在升级后删除，非受管和损坏目录保持原样。
- [ ] 根/子项替换、链接、等长原位改写、删除边界变化和预算耗尽均由云端回归覆盖；任何可观察变化或预算超限都保留候选/隔离数据并返回有界 warning。
- [ ] session/config 任一写入失败时恢复原状态，失败快照按既定诊断语义保留。
- [ ] 云端 Rust 覆盖迁移、单代保留、回滚、非受管保护和归档不扫描。

## Notes

- “旧格式”必须由既有 `managed_by` 与 `version=1` manifest 精确识别，关联 `AIO-PENDING-017`。本任务与 AUD-002、AUD-035、AUD-033 共用统一 PR，但验收边界保持独立。
