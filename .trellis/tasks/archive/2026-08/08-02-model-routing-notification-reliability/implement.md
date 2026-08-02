# 集成实施计划

1. 完成并验证 `08-02-model-routing-policy`，提交模型路由功能。
2. 完成并验证 `08-02-task-complete-notify-reliability`，提交通知修复。
3. 更新跨层规范，执行全量本地允许的验证，推送分支并创建 PR。
4. 只使用云端 CI 运行 Rust、TUI、数据库迁移、格式化、Clippy 和生成绑定检查；应用 CI 提供的有界漂移补丁。
5. CI 全绿后合入 `main`，不改版本号、不创建 Release。
