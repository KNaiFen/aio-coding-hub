# 当前实现证据

- 模型在 `BodyReaderMiddleware` 完成解码和 `RequestAfterBodyRead` 插件后，由 `ModelInferenceMiddleware` 识别。
- `provider_iterator` 为每个供应商从 `request_body_state.decoded_clone()` 重新构造请求，再执行 OAuth、CX2CC/协议桥接、Claude 映射、元数据和 ChatGPT 兼容处理。
- `PreparedProvider.active_requested_model` 进入 attempt 和活动事件，适合作为最终出站模型真值。
- `request_logs::effective_cost_basis` 当前依次处理 CX2CC、托管模型和原 requested model；新配置路由必须按 final provider 增加更高优先级的最终计价依据。
- `model_route_mapping.rs` 是响应侧“请求与返回模型不一致”观测，不是用户配置路由；新审计类型必须避免命名与语义混淆。
