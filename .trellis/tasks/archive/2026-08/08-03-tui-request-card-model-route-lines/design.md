# 技术设计

## 行模型

`request_card_lines` 从固定数组改为 `Vec<RequestCardLine>`。每行携带 `RequestCardLineKind`：Status、Model、ModelTarget、Provider、Route、Metrics。UI 根据 kind 选 tone，不再根据插入前的固定索引推断。

## 模型格式化

- 无有效路由：复用请求原模型，若思考强度已应用则追加 `·effort`，与 CLI、压缩标记组成单行并普通截断。
- 模型未变化：使用 route source model 和可选 `·effort`，仍为单行。
- 模型变化：源行将 `→` 作为保留尾部；完整内容超宽时先截断箭头前的 lead，再追加箭头。宽度 1 只输出箭头，宽度 0 输出空串。
- 目标行组装 `effective_model[·effort][ 压缩·模式]`，按显示宽度截断后以 `width - rendered_width` 个空格左填充。

## 兼容与失败

`request_model` 保持为 Statusline/详情页的旧单行所有者。新逻辑只读取已经过 Observer 边界验证的可选 route；不存在 route 时保持普通卡语义。
