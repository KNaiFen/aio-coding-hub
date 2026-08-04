# TUI 请求日志思考强度：技术设计

## 数据投影

- 在 Observer 请求投影中新增可选的请求侧 Codex 思考强度；旧协议与旧记录保持 `None`。
- 从请求特殊设置 `codex_reasoning_effort` 中读取 `effort`，兼容旧字段 `rawEffort`，仅接受现有受支持枚举并限制长度；解析失败时安全降级。
- 配置路由目标继续使用 `configured_model_route` 中的有效模型与路由强度。`model_route_mapping` 只表示响应不一致证据，不改变出站目标行。
- 实时活动请求和已结束历史请求经过同一投影函数，避免列表刷新前后格式变化。

## 格式

- Codex 模型名通过统一 helper 追加 `-<effort>`；缺失强度时只显示模型名。
- 非路由：`Codex / <request-model>-<request-effort>`。
- 模型路由：源行使用请求强度，目标行使用有效路由强度；目标行仍按 Unicode 显示宽度右对齐。
- 仅改变强度、不改变模型时保持单行，并显示最终有效强度，避免同一模型重复占两行。
- 继续保留末尾箭头与极窄宽度的现有截断规则。

## 兼容与验证

- 新协议字段使用 `Option` 与 serde 默认兼容旧 Observer 数据。
- 更新投影单元测试、普通/模型路由/仅强度路由/缺失证据/极窄宽度格式测试。
- 本地不运行 Rust、Cargo、rustfmt 或 Clippy；原生编译与测试交给 GitHub Actions `dev-build`。
