# 执行计划

- [ ] 更新 Tray 固定宽度布局与单元测试，验证成功/失败分区和极值计数。
- [ ] 抽取可复用的网关限额判定结果，保持 OAuth、消费窗口与 fail-open 语义一致。
- [ ] 在 forced provider 后、Session 绑定前过滤限额候选，并补充空候选内部诊断。
- [ ] 调整发送前限额竞态 gate，使其不生成 attempt，并补齐纯限额竞态的 NoEnabledProvider 收口。
- [ ] 更新 OAuth、消费限额、Session 复用、全限额、混合 gate、forced/managed 与真实 429 路由测试。
- [ ] 修订 gateway failover 与 Tray geometry 跨层规范，明确新旧契约边界。
- [ ] 运行前端目标/全量测试、typecheck、lint、build、format check、diff check 和五轴代码审查。
- [ ] 提交并推送到 origin，创建 PR；等待 GitHub Actions 原生检查并检查 dev-build 视觉结果。
- [ ] 合并前 fetch 最新 main；如有漂移则合并主线、解决冲突、重新运行验证与 CI，再合并 PR。

## 验证命令

- `pnpm test:unit -- src/tray/__tests__/TrayProviderMiniApp.test.tsx`
- `pnpm test:unit`
- `pnpm typecheck`
- `pnpm lint`
- `pnpm build`
- `pnpm format:check`
- `git diff --check`
- Trellis task validation
- GitHub Actions: Rust tests、rustfmt、Clippy、生成绑定与原生构建

## 回滚点

- Tray 布局是独立前端提交，可单独回退。
- 网关限额资格过滤、测试与契约组成一个行为提交；不包含迁移或生成绑定，可整体回退。
- 主线同步使用普通 merge，不重写已推送历史；冲突无法保持双方语义时不合并 PR。
