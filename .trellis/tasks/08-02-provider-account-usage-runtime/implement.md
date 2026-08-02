# 执行

- [x] 提取现有 fetch 执行器并建立共享状态/调度器。
- [x] 增加共享缓存与消费者租约，桌面 query 改为 5 秒心跳并保留强制手动刷新。
- [x] 向 Observer 投影可选余额摘要，供应商视图快照只续租、不直接请求远端。
- [x] 覆盖全局关闭供应商、间隔、同供应商并发、失败、配置变更与硬过期。
- [x] 前端定向验证：TypeScript、119 个 query/service/card 测试、ESLint、Prettier、diff check。
- [ ] GitHub `dev-build` 原生编译、Rust 测试及生成绑定漂移验证（本地规则禁止运行）。
