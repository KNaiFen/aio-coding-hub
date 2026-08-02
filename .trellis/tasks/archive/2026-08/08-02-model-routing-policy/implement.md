# 实施清单

1. 增加策略类型、默认值、校验和前端编辑模型。
2. 增加数据库 v44 迁移与 Provider CRUD/share/config-migrate 传播。
3. 在 provider preparation 尾部实现有界、fail-open 的模型和强度改写。
4. 增加 provider-scoped 审计标记、最终模型计价和响应观测兼容。
5. 增加桌面路由徽标、Observer 可选字段及 TUI 紧凑/详情展示。
6. 补齐 Rust、前端与 TUI 测试；本地只跑允许的 Node/前端检查，Rust 交给 CI。
7. 更新跨层规范并提交 `feat(gateway): add configurable model routing`。
