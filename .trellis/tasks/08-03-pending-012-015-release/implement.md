# 实施计划

1. 完成并独立检查余额心跳子任务，提交 `fix(providers)`。
2. 完成并独立检查 Tray 密度子任务，提交 `fix(tray)`。
3. 完成并独立检查 TUI 路由卡子任务，提交 `feat(tui)`。
4. 运行允许的完整本地验证、差异与敏感信息审计，创建功能 PR并处理云端漂移补丁。
5. 合并功能 PR，创建并合并 `0.60.45` 版本 PR，等待精确 main SHA 候选 CI。
6. 创建标签、发布 Release，核对资产、校验和、更新元数据与标签指向。
7. 归档 PENDING、三个子任务与父任务，通过最终文档 PR 合入证据。

## 实施记录（2026-08-03）

- AIO-PENDING-014 已提交为 `518e984`，保留五秒消费者心跳并隔离后台、首次和手动刷新可见状态。
- AIO-PENDING-015 已提交为 `a9859c3`，同步前端 24px 行高、240px 滚动上限和原生窗口几何；截图、DPR 与实机视觉验证按用户决定豁免。
- AIO-PENDING-012 已提交为 `cd8af24`，请求卡使用动态语义行并将实际模型路由拆为源/目标两行，Statusline、详情与协议边界保持不变。
- 本地源码范围 Vitest 307 个文件、2672 个测试通过；TypeScript、ESLint、Vite build、Prettier、规格链接、TUI 发布合同、Trellis 73 份清单和 diff check 均通过。
- 工作区级 Vitest 另扫描到不提交的 `.local/codex-cli-reference` 四个 Jest suite，因本地缺少 `@jest/globals` 失败；`vitest run src` 已证明项目源码范围全绿。Rust/native 验证依仓库规则留给 GitHub Actions。
