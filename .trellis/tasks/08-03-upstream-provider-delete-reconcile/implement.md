# Provider 删除收敛执行计划

- [x] 添加 sort-mode query prefix 和定向测试。
- [x] 扩展 Provider 删除 mutation 的 cancel、cache update 与 invalidate 集合。
- [x] 移植请求时 Provider 身份展示，不覆盖 fork 的 request log 类型扩展。
- [x] 覆盖迟到响应、多个 sort mode、默认路由和历史身份回归。
- [x] 运行前端定向测试、类型检查和 Vite build；native 测试交由 Actions。

## Review Findings

- 范围内的旧 query 竞态已由 cancel → filter → invalidate 顺序和 deferred query 测试覆盖。
- 范围外：`providerDelete` 完成后才校验 `cliKey`，内部调用方若传入非法或错配 CLI，可能删除成功但未收敛正确缓存。该行为在固定上游提交中独立存在；后续任务应在副作用前校验，并考虑由后端返回实际 Provider 身份。
- 范围外：本次只取消 query，较早启动的 reorder/upsert 等写 mutation 仍可能晚回写旧 Provider。若产品要把“迟到响应”扩大到所有 mutation，后续需引入删除 generation/tombstone 或统一写入守卫。
- 项目规则禁止本地 native 验证；Rust 删除级联测试仅做静态审阅，编译与执行交由 GitHub Actions。
