# 最终修复与本地零产物：实施清单

- [x] 建立并合并 AUD-054：云端验证与本地零产物合同。
- [x] 合并后核验并清理精确的 `src-tauri/target*`、仓库内 `node_modules` 和已知前端缓存。
- [x] 建立并合并 AUD-056：请求日志与运行日志留存。
- [x] 建立并合并 AUD-016：非回环 Gateway Bearer Token。
- [x] 建立并合并 AUD-008：跨重启数据重置与维护态。
- [x] 将 AUD-055、AUD-002、AUD-035、AUD-033 的产品代码汇集到 `codex/final-hardening-unified`。
- [x] 更新报告、PENDING、Trellis 与后端规范，使统一 PR 成为剩余四项的唯一交付表面。
- [x] 完成本地允许的零依赖合同、源码/JSON 解析和 `git diff --check`，并完成 Unix、Windows、集成/文档三路只读复审。
- [x] 提交、重放最新 `origin/main`、推送并创建面向 `main` 的唯一统一 PR。
- [x] 对统一 PR 精确远端 head 触发 `ci.yml` 全量 `workflow_dispatch`，等待 `ci-gate`；仅接纳同一 SHA 的云端生成漂移。
- [x] Actions 启动后按 #94、#93、#92、#87 顺序关闭旧 PR，并在评论中链接统一 PR。
- [x] 合并统一 PR，核验主线树，再按完成证据收口仍在活跃列表的 PENDING/Trellis 状态。

统一候选执行：最新主线门 -> 本地允许检查 -> 独立复审 -> 提交 -> 重放最新主线 -> 推送/建 PR -> 精确 head `workflow_dispatch` -> 等待 `ci-gate` -> 最终主线门 -> 合并 -> 合并后树核验。
