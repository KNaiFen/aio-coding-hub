# 执行与交付

本页供执行者使用。执行者只能在登记的 worktree 中工作。

## 开工

1. 读取 handoff envelope、任务 requirements/plan/authorization 和项目 `.gkd/policy.json`。
2. 领取任务，验证 offer、capability、expected head、repository、branch、role 和 bundle digest。
3. 领取成功后才允许写入；任何 policy、角色、路径、base 或 head 漂移立即停止并报告。

## 施工边界

- 只修改任务授权范围；产品行为、公共 API、迁移、兼容性或发布边界变化先交 main。
- 不合并、归档、清理、改写协调 JSON 或启动其他 agent。
- 本地只运行 `scripts/gkd-verify --base-sha <full-lowercase-sha>`；依赖、构建、测试、签名和发布由 GitHub Actions 完成。

## 交付

1. 更新任务 `delivery.md`，记录实际实现、AC 证据、偏移和风险，不缓存 PR/head/CI 事实。
2. 保持 worktree clean，提交 delivery digest、candidate output bundle digest 和完整 head。
3. 交付后立即停止写入；把唯一 delivery 输入交给 main 和独立验收者。

任何新提交都会使旧 delivery、CI 和验收结论失效。
