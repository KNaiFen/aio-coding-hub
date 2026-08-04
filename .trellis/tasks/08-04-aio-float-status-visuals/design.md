# AIO 悬浮窗状态条与液态背景：技术设计

## 几何

原生窗口继续为 404px：`1 border + 12 padding + 92 provider + 8 gap + 198 availability + 8 gap + 72 totals + 12 padding + 1 border`。

- 状态条从 170px 增至 198px，起点左移 4px，18 格和 17 个 2px 间隔保持等宽。
- 状态字容器最小 `40px`，容纳两个 18px 标记及 4px 间隔；第三个去重标记出现时容器自然扩展，供应商名在剩余空间截断。
- totals 删除可见“成/败”和中间 divider，只保留两个 32px 右对齐数字，间隔 8px。
- 完整数值继续保留在每个数字的 `title` 和 totals 的 `aria-label`。

## 静态液态玻璃

- 保留原生透明窗口、零 RGBA 底色和 macOS `Effect::Popover`。
- 新增专用 surface class，使用约 52% 浅色/42% 深色背景、24-28px backdrop blur、饱和度增强、半透明边缘和统一光源的内高光/阴影。
- `html/body/#root` 继续透明；不添加动画、图片、渐变球或新依赖。
- 不支持 backdrop filter 时由半透明背景保持可读，不回退为全透明。

## 兼容与文档

- 快照 DTO、18 格、计数压缩、十行滚动、窗口尺寸与定位不变。
- 同步更新托盘几何合同及其跨层固定值测试。
- 原生材质只能通过 GitHub Actions `dev-build` 的 macOS 制品验收。

## 验证

- DOM 无可见“成/败”，但精确无障碍名称和 tooltip 保留。
- 两个状态字、长名称、最大精确/压缩计数不重叠。
- 明暗主题结构截图；原生制品截图能看见悬浮窗后的桌面内容。
