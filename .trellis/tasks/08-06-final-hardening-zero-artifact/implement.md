# 最终修复与本地零产物：实施清单

- [x] 建立并合并 AUD-054：云端验证与本地零产物合同。
- [x] 合并后核验并清理精确的 `src-tauri/target*`、仓库内 `node_modules` 和已知前端缓存。
- [x] 完成 AUD-055 代码、提交和草稿 PR；CI 重跑、Ready、主线门与合并列入待执行交付。
- [x] 完成 AUD-056 代码提交 `28d65b2d`、`c86799ce`；PR、Actions、bindings 核验与合并列入待执行交付。
- [ ] 建立并合并 AUD-016：非回环 Gateway Bearer Token。
- [ ] 建立并合并 AUD-008：跨重启数据重置与维护态。
- [ ] 建立并合并 AUD-002：SQLite 权威恢复 journal。
- [ ] 建立并合并 AUD-035：Observer 有界摘要查询与 folder cache。
- [ ] 建立并合并 AUD-033：插件激活事件与持久 quarantine。
- [ ] 创建并合并纯文档收口 PR，归档 PENDING/Trellis 并更新审计汇总。

每项统一执行：最新主线门 -> failure-first 合同/测试 -> 最小实现 -> 本地允许检查 -> 独立五轴审查 -> 推送 -> 精确分支 `workflow_dispatch` -> 等待 `ci-gate` -> 最终主线门 -> 合并 -> 合并后树核验。
