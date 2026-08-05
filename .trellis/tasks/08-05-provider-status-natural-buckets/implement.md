# 实施清单

- [x] 删除 36 格对齐特判，直接使用 bucket 宽度。
- [x] 更新自然边界测试，覆盖 36/18/12 格当前区间和历史聚合。
- [x] 保留最后请求覆盖及同毫秒 rowid 破局测试。
- [ ] 由 GitHub Actions 运行 Rust 格式化与测试。

## 本地验证

- `CI=true pnpm vitest run src/components/providers/__tests__/ProviderAvailabilityStrip.test.tsx src/tray/__tests__/TrayProviderMiniApp.test.tsx src/services/__tests__/trayProviderMini.test.ts`：3 个测试文件、23 个测试通过。
- 独立静态审阅确认 12/18/36 格的 30/20/10 分钟自然边界、当前格覆盖、历史成功率聚合与同毫秒 `rowid` 排序。
- 按仓库规则未在本地运行 Cargo、rustfmt 或 Rust 测试。
