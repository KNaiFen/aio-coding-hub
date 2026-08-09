# 执行计划

- [x] 扩展共享 Tooltip，增加默认关闭的 panel surface、hoverable content 和碰撞边距参数。
- [x] 在链路摘要构建器中派生实际请求与额外重试计数，生成单行紧凑标签并保留完整摘要。
- [x] 重构代理链详情的结构与语义颜色，去除已知重复内部原因并保留未知原因。
- [x] 将首页代理链入口接入 panel surface、单行数字样式和完整辅助说明。
- [x] 更新目标单元与组件测试，覆盖语义、容错和共享 Tooltip 默认行为。
- [x] 运行目标 Vitest、`pnpm typecheck`、`pnpm lint`、`pnpm build`。
- [x] 启动 `pnpm dev`，使用 Playwright 验证明暗主题、窄/宽视口、长链路和视口碰撞。
- [x] 检查差异、评估是否需要更新跨层规范，并按 Trellis 流程收尾。

## 回滚点

- Tooltip surface 扩展与摘要逻辑分别保持独立边界；任一部分失败时可单独恢复，不影响请求转发。
- 不生成或修改 Rust 绑定，不运行本地原生工具链。

## 验证结果

- 目标测试：4 个文件、67 项通过。
- AIO 源码全量测试：300 个文件、2625 项通过。
- `pnpm typecheck`、`pnpm lint`、`pnpm build`：通过。
- `pnpm test:unit` 会额外扫描未跟踪的 `.local/codex-cli-reference`，其中 4 个 Jest 文件因本项目未安装 `@jest/globals` 失败；AIO 自身测试无失败，因此改用 `vitest run src` 作为项目范围全量结果。
- Playwright：1024px 明暗主题与 1440px 浅色主题通过；摘要高度约 16.5px、`white-space: nowrap`。长链路面板在 768px 视口内底边为 756.25px，保留约 12px 碰撞边距；`scrollHeight=756`、`clientHeight=489`，滚动后 `scrollTop=267` 且面板保持打开。
- 未执行任何 Cargo、Rust、Clippy、rustfmt、Tauri 或绑定生成命令。
