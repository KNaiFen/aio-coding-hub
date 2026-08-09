# 首页代理链 UI 调研

## 截图证据

- `截屏2026-08-01 16.41.10.png`：固定 110px 链路摘要列把“3 家 · 切换 2 次 · 尝试 3 次”挤成两行。
- `截屏2026-08-01 16.41.39.png`：深色页面上的链路详情使用浅色反向 Tooltip surface，但内容仍包含面向深色背景的 `text-white` / `text-*-300`，形成低对比；面板过窄且逐跳信息重复。

## 代码证据

- `src/ui/shadcn/tooltip.tsx` 的默认 surface 有意使用 `bg-foreground text-background`，适合短提示，不应全局改色。
- `src/components/home/HomeRequestLogsPanel.tsx` 只给链路详情设置 `max-w-[400px]`，链路摘要列固定为 `w-[110px]`。
- `src/components/home/RouteTooltipContent.tsx` 使用 `text-white` 和单侧亮色，并同时渲染 error、decision、reason 和 skip 说明。
- `src/components/home/requestLogPresentation.ts` 当前把 provider、transition 与 attempt 组成自然语言短标签，且 attempt 包含 skipped row，不能代表实际上游请求数。
- `.trellis/spec/aio-coding-hub/cross-layer/gateway-failover-route-contract.md` 明确区分 route hop、transition 和 persisted attempt；新 UI 必须继续遵守该区分。

## 决策

- 默认 Tooltip 不动，代理链详情显式选择主题面板 surface。
- 实际请求数只累计非 skipped hop 的 attempts；额外重试数按每个非 skipped hop 的 `attempts - 1` 累计。
- 已知网关内部 reason 仅在已由中文错误分类完整表达时隐藏，未知 future reason 保留。
