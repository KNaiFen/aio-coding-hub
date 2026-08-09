# 实施清单

1. [x] 激活子任务并创建独立分支/worktree。
2. [x] 重构 totals 与 row tracks，固定状态字预留空间。
3. [x] 新增静态 Liquid Glass surface 并保持原生透明链路。
4. [x] 更新组件测试与跨层固定几何合同测试。
5. [x] 更新托盘几何规范。
6. [x] 运行目标 Vitest、TypeScript、lint 和 Vite build。
7. [x] 通过浏览器明暗主题截图检查布局；原生材质交给 `dev-build` 制品验收。
8. [x] 已同步 `origin/main@02b9980d`（PR #51 合并提交）并完成相关实现审计；无直接路径冲突，托盘几何与共享状态协议保持兼容。

## 回滚点

- 原生窗口几何和生命周期不变；视觉回滚只涉及前端组件、surface CSS 和合同文本。
