# 实施计划

## 已交付基础（0.60.45）

- [x] 将前端供应商行从 36px 收紧到 24px，滚动容器上限从 360px 收紧到 240px。
- [x] 将原生行高同步为 24px，并覆盖 0/1/5/10/20 行及 2x placement 高度。
- [x] 保持十行滚动、快照回顶、标题、空状态和 hover 行为；功能 PR #24 与 `0.60.45` 发布验证通过。
- [x] 根据用户实际截图重新打开横向布局验收；不把首轮密度交付误记为 AIO-PENDING-015 全部完成。

## 待实施修正

1. [x] 在 `TrayProviderMiniApp.tsx` 把供应商行改为 96/178/88px 固定三列，名称保持单行省略，状态条保持 18 格。
2. [x] 将合计渲染拆为 12/32/12/32px 固定四列，增加纯计数紧凑格式化函数；超过 99,999 后按已锁定的`万/亿`向下截断规则显示，保留精确 `title` 与 `aria-label`。
3. [x] 将 `resident.rs` 原生窗口宽度从 440px 同步为 404px，扩展 placement 与 2x 缩放断言；不改变高度公式。
4. [x] 更新 Tray mini 跨层合同与 React 测试，覆盖长名称、原因标记、0/9/1034/99999/超大计数和多行稳定对齐。
5. [x] 运行定向 Vitest，再运行 `vitest run src`、TypeScript、ESLint、Prettier、Vite build、规格链接、Trellis validate 与 `git diff --check`。
6. [x] 启动前端 Vite mini 入口，通过可控 Tauri bridge fixture 生成 404px 的普通与 2x 截图，检查无换行、重叠、裁切和列位移。
7. [ ] 完成差异审查与敏感信息检查，创建 origin 功能 PR；等待 GitHub Actions 的 Rust、原生几何、格式、绑定、Clippy、测试和审计全部通过后合并。
8. [ ] 从最新 `origin/main` 创建独立版本 PR，将补丁版本提升到 `0.60.46`；只接受精确 main SHA 的成功 release-candidate。
9. [ ] 创建并验证 `aio-coding-hub-v0.60.46`，核对标签 SHA、12 个资产、11 个校验和和 `latest.json`。
10. [ ] 写回 PR、提交、CI、截图与 Release 证据，归档 AIO-PENDING-015 和父 Trellis 任务并提交最终归档 PR。

## 回滚点

- 前端固定列和计数格式化可以作为一个功能提交整体回退；回退不得删除 24px 行高基础。
- 原生宽度与前端宽度合同必须同进同退，任何 CI 漂移都阻止合并。
- 若 404px 视觉 fixture 证明原因标记或 18 格状态条不可读，先在同一 404px 总宽内重新分配固定列并更新计划证据；不得恢复可伸展列或允许计数换行。

## 首轮独立检查记录

- `0.60.45` 的 24px 高度实现通过前端定向测试、源码范围完整测试、TypeScript、ESLint、Prettier、Vite build、规格链接和云端 Rust/原生验证。
- 2026-08-03 的用户截图是新的反例：它证明 440px/可伸展名称/64px flex 合计合同仍会造成横向松散和四位计数错位，因此本任务保持 `in_progress`。
- 2026-08-03 用户确认紧凑计数使用向下截断而不是四舍五入：缩放值小于 100 时最多保留一位小数，否则显示截断整数，避免可见值高估精确计数。

## 续作本地验证记录

- Tray mini 定向 Vitest 为 17/17、跨层合同测试为 16/16；`vitest run src` 为 307 个测试文件、2686 个测试全部通过。
- TypeScript、ESLint、本次非 Rust 变更文件 Prettier、Vite build、规格链接、Trellis validate 与 `git diff --check` 通过；Rust/native 按仓库规则留给 GitHub Actions。
- 全仓 `prettier --check .` 仍会报告未改动的 `src/query/__tests__/gateway.test.tsx` 既有格式差异，本任务未扩大范围改写该文件。
- Playwright Tauri bridge fixture 在 404x284 逻辑视口验证 12 行数据：三列起点为 13/117/303，四个合计轨道起点为 303/315/347/359，所有计数均 `nowrap` 且 `scrollWidth <= 32`。
- 截图保存为 `/private/tmp/aio-tray-mini-density-1x.png`（404x284）与 `/private/tmp/aio-tray-mini-density-2x.png`（808x568）；两个浏览器会话均为 0 errors、0 warnings。
