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
- [ ] 对精确分支触发 `ci.yml workflow_dispatch`，核对 `head_sha` 并等待 `ci-gate`。
- [ ] CI 漂移只应用绑定当前 SHA 的有界补丁；功能失败修复后重跑完整 CI。
- [ ] CI 绿色后创建一个指向 `main` 的 PR，不自动合并。
