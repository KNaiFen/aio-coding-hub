# 技术设计

## 固定横向合同

Tray mini 的逻辑宽度由原生常量与前端网格共同拥有，必须同步修改：

```text
404 = 1 左边框 + 12 左内边距 + 96 名称 + 8 间距 + 178 状态条 + 8 间距 + 88 合计 + 12 右内边距 + 1 右边框
```

- `src-tauri/src/app/resident.rs`：`TRAY_PROVIDER_MINI_WIDTH = 404.0`。
- `src/tray/TrayProviderMiniApp.tsx`：供应商行网格固定为 `96px 178px 88px`，不再使用可伸展名称列。
- `.trellis/spec/aio-coding-hub/cross-layer/tray-provider-mini-contract.md`：记录 404px 宽度和三列所有权，防止前端与原生再次漂移。

名称区继续使用 `min-width: 0` 与单行省略。原因标记留在名称区内部，名称文本先收缩；完整供应商名称继续由 `title` 提供。状态区保留 18 个等分轨道和既有 2px 间距，在 178px 内精确得到 18 个 8px 状态格。

## 稳定计数合同

合计区不再把 `成{count}`、`败{count}` 当作两个可变宽 flex 项，而是使用固定 `12px / 32px / 12px / 32px` 网格：

- 两个 12px 标签列分别只渲染 `成` 和 `败`，水平起点在所有行完全一致。
- 两个 32px 数值列右对齐、tabular numbers、单行显示，数字变化只影响各自单元内部。
- `0..=99_999` 显示完整十进制数；更大值按 `万/亿` 规则压缩到固定列可容纳的文本。
- 可见紧凑值只影响表现层；`aria-label` 与每个数字的 `title` 始终使用原始 `u32` 精确值。

紧凑格式化函数保持纯函数并单独测试边界，不改变 DTO、后端统计或排序。

## 几何与兼容边界

24px 行高、42px 标题、68px 空状态、最多十行和 `42 + content + 2` 高度公式不变。原生 placement 使用新的 404px 逻辑宽度参与屏幕边缘 clamp 和 scale factor 计算；应扩展既有 1x/2x 测试，而不运行本地 Rust 工具链。

窗口仍不可聚焦、无边框、置顶、透明并使用现有 Popover 材质。CLI 选择、供应商快照、路由冻结、滚动和 hover IPC 均不改。

## 验证设计

- React 测试断言固定网格类、名称省略、18 格状态条以及 0/9/1034/99999/超大计数的固定四列与精确可访问文本。
- 跨层合同测试锁定 404px、96/178/88px 与 12/32/12/32px，不让 CSS 和原生常量独立演化。
- 通过 Vite mini 入口和可控 Tauri bridge fixture 生成 404px 逻辑视口截图，覆盖普通和 2x 像素密度、多行长短名称与计数混排。
- 本地只运行 Node、TypeScript、ESLint、Prettier、Vitest 和 Vite build；Rust、原生几何和桌面构建由 GitHub Actions 执行。
