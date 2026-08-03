# 执行计划

- [x] 更新 Tray 固定宽度布局与单元测试，验证成功/失败分区和极值计数。
- [x] 抽取可复用的网关限额判定结果，保持 OAuth、消费窗口与 fail-open 语义一致。
- [x] 在 forced provider 后、Session 绑定前过滤限额候选，并补充空候选内部诊断。
- [x] 调整发送前限额竞态 gate，使其不生成 attempt，并补齐纯限额竞态的 NoEnabledProvider 收口。
- [x] 更新 OAuth、消费限额、Session 复用、全限额、混合 gate、forced/managed 与真实 429 路由测试。
- [x] 修订 gateway failover 与 Tray geometry 跨层规范，明确新旧契约边界。
- [x] 运行前端目标/全量测试、typecheck、lint、build、format check、diff check 和五轴代码审查。
- [x] 提交并推送到 origin，创建 PR；等待 GitHub Actions 原生检查并检查 dev-build 视觉结果。
- [x] 合并前 fetch 最新 main；如有漂移则合并主线、解决冲突、重新运行验证与 CI，再合并 PR。

## 交付证据

- 功能 PR：[#35](https://github.com/KNaiFen/aio-coding-hub/pull/35)，head `b91cd16e4acedc2ed94273497c9fea769451d1e6`，merge commit `a0db6c20cfbae0d2b3cb64fbf868eed4110979b0`。
- PR CI：`30838390347` 全部通过，包括 frontend、Rust format/bindings、Clippy、Rust tests、dependency audit、契约检查与总门禁。
- macOS arm64 dev-build：`30838393906` 构建与开发制品上传通过。
- 合并后 main CI：`30840406383` 全部通过；候选构建经计划判定正常跳过。
- 合并前最终 fetch 的 `origin/main` 仍为已审查的 `523256fc4108f03731bedb3962ff1d88acab01f4`，新增提交和文件均为 0；合并提交与已验证 head 的文件树差异为 0。

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
