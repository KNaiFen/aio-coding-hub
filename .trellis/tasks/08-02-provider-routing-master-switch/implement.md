# 执行

1. [x] 修正默认/自定义候选、桥接源查询和 Observer 资格投影。
2. [x] 增加每次上游发送前的运行时门禁，并接入供应商启停/保存/删除生命周期。
3. [x] 全局关闭时禁用路由编辑器控件、保留模板成员值并显示关闭状态。
4. [x] 增加 SQL、运行时门禁、Observer 与前端回归；Rust 全套由 CI 运行。

## 验证证据

- `vitest run src/pages/providers/__tests__/ProvidersView.test.tsx`: 40/40 通过。
- `tsc -p tsconfig.json --noEmit`: 通过。
- 变更文件 ESLint、Prettier、网关错误码同步和 spec link 检查：通过。
- 本机未运行 Cargo、rustfmt、Clippy 或 Rust 测试；交由 GitHub Actions。
