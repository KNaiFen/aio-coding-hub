# AUD-016 实施清单

- [ ] failure-first 覆盖非回环 health/proxy/provider route 无认证可达及 header 伪造。
- [ ] 增加 token 摘要、确认代次、settings migration 和 transient one-time result。
- [ ] 用真实 peer `ConnectInfo` 在 Router 最外层鉴权并剥离敏感/身份头。
- [ ] 将 token 创建/轮换纳入 settings owned transaction、runtime refresh 和 WSL 同步。
- [ ] 删除 provider route、forced-provider 数据流与 Claude Terminal 全栈入口。
- [ ] 覆盖 loopback、LAN/custom、严格 Bearer、迁移、轮换、重启和日志脱敏。
- [ ] 本地只执行源合同、解析和 diff；bindings/native/frontend 由 Actions 验证。
- [ ] 合并后在 AUD-008 候选记录证据。
