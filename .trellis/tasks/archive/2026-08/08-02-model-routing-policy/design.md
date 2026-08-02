# 技术设计

## 数据与接口

- 新增 `ModelRoutingPolicy { enabled, rules }` 和 `ModelRoutingRule { source_model, target_model, reasoning_effort }`。
- `AppSettings` 保存全局策略；数据库 v44 为 `providers` 增加 `model_routing_policy_json TEXT NULL`。
- Provider IPC 使用 override + specified 语义，和现有重试策略一致；分享 v2 与整库导入导出增加可选字段。

## 请求数据流

1. 使用原始客户端请求的 `requested_model` 作为不可变匹配键；插件后续改写不得改变规则匹配结果。
2. 每次 provider preparation 从请求基线构造桥接后的 path/query/body。
3. 选择 provider override 或全局策略，并对原模型单次精确匹配。
4. 在现有桥接、Claude 映射和兼容处理之后改写最终出站模型与协议对应强度字段。
5. 更新该尝试的 `active_requested_model`，写入 provider-scoped `configured_model_route` 标记。

协议字段：Claude `output_config.effort`；Responses `reasoning.effort`；Grok Chat `reasoning_effort`；Gemini 整数使用 `thinkingBudget`，其他文本使用 `thinkingLevel`。

## 计价与观测

- 标记保存 final provider、policy source、requested/effective model、effort、priced CLI/model 和 applied 状态，不保存正文。
- `effective_cost_basis` 优先解析与 final provider 匹配且已应用的配置路由；无价格时保留空费用。
- 响应模型观测以配置后的预期模型比较，避免主动路由被误报为不一致。
- Observer v1 为 `ObserverRequest` 增加 serde-default 可选字段；新旧客户端互相兼容。

## 失败语义

- 保存时严格校验配置；运行时损坏或无法改写则当前尝试使用桥接后但未应用新规则的请求继续发送。
- 失败标记不得改变供应商选择、重试、熔断或计价；只有应用成功的最终供应商标记参与费用计算。
