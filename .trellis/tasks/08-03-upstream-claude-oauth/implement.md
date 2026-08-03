# Claude OAuth 执行计划

- [ ] 为 OAuth endpoint/request 增加可选 token UA metadata。
- [ ] 实现 Claude 内置/自定义 client 分流并覆盖构造测试。
- [ ] exchange 与 refresh 共用 Anthropic request builder，保留当前安全错误处理。
- [ ] 通过 GitHub Actions 生成 native dev-build。
- [ ] 使用隔离账号完成完整 OAuth 门禁；记录成功证据后才允许合并。
