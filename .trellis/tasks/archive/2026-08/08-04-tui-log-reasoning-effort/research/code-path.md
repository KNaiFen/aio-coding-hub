# TUI 思考强度代码调研

## 结论

- TUI 普通请求目前只显示 `Codex / model`；配置路由只在目标侧显示 `·effort`：`src-tauri/crates/aio-tui/src/format.rs:461-515`。
- 原请求强度已作为 `codex_reasoning_effort` 特殊设置保存：`src-tauri/src/gateway/proxy/handler/middleware/model_inference.rs:139`。
- 配置路由标记保存源/目标/有效模型、路由强度和 applied 标志：`src-tauri/src/gateway/configured_model_route.rs:114`。
- 响应不一致证据另存为 `model_route_mapping`，包含请求/实际模型和强度：`src-tauri/src/gateway/model_route_mapping.rs:45-102`。
- Observer 实时/历史投影目前只解析 `configured_model_route`：`src-tauri/src/app/observer/snapshot.rs:822-932,1084`。
- 协议 `ObserverConfiguredModelRoute` 缺少原请求强度与响应观察字段：`src-tauri/crates/aio-observer-protocol/src/lib.rs:268-276`。

## 最小边界

预计修改 Observer 协议、快照投影和 TUI 格式化/测试。除非产品决定第二行显示响应观察值，否则不需改变网关采集。

## 现有测试抓手

- 两行与右对齐：`src-tauri/crates/aio-tui/src/format.rs:1188`。
- 仅强度路由：`src-tauri/crates/aio-tui/src/format.rs:1227`。
- 极窄宽度：`src-tauri/crates/aio-tui/src/format.rs:1246`。
- 卡片行数与语义色：`src-tauri/crates/aio-tui/src/ui.rs:1919`。
