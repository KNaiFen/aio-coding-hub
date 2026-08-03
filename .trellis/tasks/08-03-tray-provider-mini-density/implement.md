# 实施计划

1. 同步修改前端行高、滚动上限和原生行高常量。
2. 扩展 Tray React 测试覆盖密度、十行上限和最后一行可达。
3. 扩展 resident Rust 几何测试覆盖关键行数与缩放。
4. 新增 Tray mini 跨层几何合同并运行允许的前端验证。

## 独立检查记录

- 结论：未发现阻塞或需修复的问题；实现符合 PRD、设计及
  `.trellis/spec/aio-coding-hub/cross-layer/tray-provider-mini-contract.md`。
- `node node_modules/vitest/vitest.mjs run src/tray/__tests__/TrayProviderMiniApp.test.tsx`：4 个测试通过。
- `node node_modules/typescript/bin/tsc -p tsconfig.json`：通过。
- `node node_modules/eslint/bin/eslint.js src/tray/TrayProviderMiniApp.tsx src/tray/__tests__/TrayProviderMiniApp.test.tsx`：通过。
- `node node_modules/prettier/bin/prettier.cjs --check`（受影响前端、合同与任务文件）：通过。
- `node scripts/check-spec-links.mjs`、`git diff --check`：通过。
- `node node_modules/vite/bin/vite.js build`：通过；仅报告既有 Browserslist 数据过期及大 chunk 警告。
- 未运行 Cargo、Rust 单测、rustfmt、Clippy、Tauri 或截图/DPR/实体屏验证，符合仓库规则及用户豁免；原生验证待 GitHub Actions。
