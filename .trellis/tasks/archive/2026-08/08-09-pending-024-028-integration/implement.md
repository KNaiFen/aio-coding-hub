# Implementation Plan

## Phase A: Code-Complete Barrier

- [x] 确认变基后基线、用户未跟踪文件和当前任务范围。
- [x] 完成图表 B 格式/动态轴宽和余额左对齐，同时写好前端回归测试。
- [x] 完成状态枚举、阈值、当前格最后观测和前端三色渲染。
- [x] 完成流式最终 attempt 计时修复、单条 TPS 和全部统计聚合的算术平均。
- [x] 完成 schema 52、Provider 配置跨层贯通、probe coordinator、后台 scheduler 和手动/定时观测持久化。
- [x] 完成 Rust/前端/迁移测试代码、活跃规范更新和 Trellis/PENDING 一致性检查。
- [x] 修复云端暴露的 Provider 查询漏列、OAuth 测试环境恢复和 Codex drain 提前 finalize。
- [x] 复核并修复 scheduler 固定前缀截断、跨代际双 flight、过期排队补跑和 stale tombstone。
- [x] 执行变更清单对照；在全部项目完成前不运行任何测试/构建/格式化。

## Phase B: Unified Verification

- [x] 运行 `node scripts/check-cloud-only-verification.selftest.mjs`。
- [x] 运行 `node scripts/check-cloud-only-verification.mjs`。
- [x] 对本批变更的 `.mjs` 执行必要的 `node --check`（本批无 `.mjs` 变更）。
- [x] 运行 `git diff --check`，检查调试输出、TODO、敏感信息、旧 75%/加权 TPS 公式和非预期文件。
- [x] 执行全范围正确性、架构、安全、性能和测试覆盖审查，修复后重新执行本阶段。

## Phase C: Commit, Actions, PR

- [x] 显式暂存本批路径，对照 allowlist 排除既有未跟踪文件。
- [x] 创建 `feat: 统一用量速率与供应商可用性监测` 功能提交并推送；后续云端反馈使用独立有界修复提交。
- [x] 对精确分支触发 `ci.yml workflow_dispatch`，核对 `head_sha` 并等待 `ci-gate`。
- [x] CI 漂移只应用绑定当前 SHA 的有界补丁；功能失败修复后重跑完整 CI。
- [x] CI 绿色后创建一个指向 `main` 的 PR；经用户明确授权后合并。

## Delivery Evidence

- 功能 PR #98：head `9d4123ff230d5c3ef8bedbfcd1b3d95206fd3611`，PR CI `31311729903`、workflow_dispatch `31309798396`，merge commit `66b9716690d3026b9d1b8d3d8765ead46d17e291`。
- 版本 PR #99：版本提交 `589d4fa4844167956714c37b958162e9aab58e9e`，PR CI `31314550647`，merge commit `4cc63a3190dfa0d8351294e428b968c9df82fb7b`。
- 主线候选 CI `31315530596`：Windows/macOS 签名桌面候选、四平台 TUI、统一候选聚合和 `ci-gate` 全部成功。
- 正式版本 `aio-coding-hub-v0.60.50`：tag peeled 到 `4cc63a3190dfa0d8351294e428b968c9df82fb7b`，release workflow `31317631120` 成功，12 个资产全部上传。
