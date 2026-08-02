# 设计

请求格式继续由 `format` 纯函数拥有；供应商卡改为可变 `Vec<Line>`。新增共享语义 palette/profile，所有视图从角色取色。Observer 客户端增加有界 POST，TUI 会话按 provider ID 保存测试状态。可用性行消费后端已分类的 12 桶。
