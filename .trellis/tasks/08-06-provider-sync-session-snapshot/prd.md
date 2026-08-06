# AUD-055 Provider Sync 快照体积

## Goal

将 Provider Sync 快照收窄为 session-only 新格式，并把受管备份稳定限制为最新一代。

## Requirements

- 新格式不再扫描、改写或备份 `archived_sessions`。
- 备份只保留最新一代新格式 managed backup。
- 升级时自动删除能够由旧 v1 manifest 证明所有权的 managed backup。
- 无 manifest、损坏 manifest、marker 不匹配、符号链接或其他非受管目录绝不删除。
- 保留同步失败的完整回滚能力，不扩大到其他恢复系统。

## Acceptance Criteria

- [ ] 普通同步只修改活动 `sessions`，归档会话字节保持不变。
- [ ] 成功同步后最多存在一个新格式 managed backup。
- [ ] v1 managed backup 在升级后删除，非受管和损坏目录保持原样。
- [ ] session/config 任一写入失败时恢复原状态，失败快照按既定诊断语义保留。
- [ ] 云端 Rust 覆盖迁移、单代保留、回滚、非受管保护和归档不扫描。

## Notes

- “旧格式”必须由既有 `managed_by` 与 `version=1` manifest 精确识别，关联 `AIO-PENDING-017`。
