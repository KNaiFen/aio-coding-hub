# AUD-016 实施清单

- [x] 覆盖非回环 health/proxy/provider route 无认证可达及 header 伪造。
- [x] 增加私有 sidecar 摘要、确认代次和 transient one-time result；明文只在受控内存中驻留。
- [x] 用真实 peer `ConnectInfo` 在 Router 最外层鉴权并剥离敏感/身份头。
- [x] 将 token 创建/轮换与 gateway 启动、runtime verifier 和 WSL 同步收敛；WSL 失败回传一次性 reveal。
- [x] 删除 provider route、forced-provider 数据流与 Claude Terminal 全栈入口。
- [x] 覆盖 loopback、LAN/custom、严格 Bearer、轮换、旧路径 404、header 脱敏及 WSL manifest v2 无凭据持久化。
- [x] 本地执行 cloud-only contract/self-test 与 `git diff --check`；bindings/native/frontend 由 Actions 验证。
- [x] 合并后在 AUD-008 候选记录证据。
