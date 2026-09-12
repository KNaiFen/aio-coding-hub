# 依赖升级 PR 审查

- 日期：2026-09-12；PLAN r1；当前结论：七项升级方向均可接受；#121、#197 的兼容性修复和 #120 的锁文件收窄已完成静态复核，全部 PR 的最终新 head CI 待执行。
- 审查范围：PR #110、#111、#112、#120、#121、#122、#197 的 diff、当前 main 兼容性、项目 API 用法、manifest/lockfile、旧 Actions 结果与上游官方迁移资料。未把无法下载的过期日志推测成事实。

## 必须修复

1. PR #121 把根 rand 升到 0.9.4，但保留 15 处 rand::thread_rng()。rand 0.9 已弃用该名称，项目 Clippy 使用 -D warnings。最小改为 rand::rng()，不改变 trait、分布或随机调用语义。
2. PR #197 把三个 workspace 的 Vitest 升至 4.1.11，却保留 @vitest/coverage-v8 3.2.4；锁文件显示两者 peer 主版本不兼容。同步覆盖率插件到 4.1.11，并删除 Vitest 4 已移除的 coverage.all；coverage.include 和既有阈值保持。
3. PR #120 的旧锁文件除 tar 外还重解析多个 Windows 依赖。当前验证不能证明这些额外变化，更新到 main 后只保留 tar 0.4.46 所需变化。

## 修复复核

- #120 本地 head `ab91fe7a96fc79246dde02bc1b7519d7303672c7` 相对基线只修改 `src-tauri/Cargo.lock` 中 tar 的版本与校验和；`git diff --check` 通过。
- #121 本地 head `75c1f4aeea2edaf2b2441fa7a9d21d538c393b57` 在 15 个文件内完成升级与 API 改名；对应工作树检索不到 `rand::thread_rng()`，`git diff --check` 通过。
- #197 已推送 head `45bdca002bf5ce752aeb2637da86c1766eb16823`：Vitest 与 coverage-v8 均为 4.1.11，删除 `coverage.all`，锁文件使用满足七天冷却期的 obug 2.1.4。锁文件由临时 Actions run 34686854665 生成并以 artifact patch 应用；最终 diff 不含临时工作流，且检索不到 coverage-v8 3.2.4、obug 2.2.1 或 `all: true`。

## 已通过静态审查

- #110 的补丁目录 time 0.3.47、#111 PostCSS 8.5.23、#112 Vite 7.3.5、#122 serde_with 3.22.0 未发现代码或工具链阻塞。
- #110 不影响应用根 Cargo.lock；#122 是 Tauri 传递依赖；#111 无直接 PostCSS API 调用；#112 的 Node 22、plugin-react 与现有 Vite 配置兼容。
- 所有旧 PR 均落后当前 main。文本合并预演未发现冲突，但旧 CI 不包含当前 main 的全部代码与门禁，因此只能作辅助证据。

## 实施审查边界

- main 可按 PLAN 的顺序创建隔离 worktree，准确实现上述最小版本与兼容修改，并逐项使用自动 CI 验证。任何新代码问题必须在对应 PR 范围内解决后再合并。
- 归档前 main 复核每项最终 diff，确保没有无关升级、覆盖率阈值降低、deprecated allow、测试删除或工作流放宽。最终可交付项统一交给一个 gkd_closeout 连续归档、推送、CI、合并、本地同步和清理。
- #113 已按用户明确要求关闭；其跨仓源码不进入依赖维护。

## 消融结论

没有理由合并 Dependabot 的历史锁文件噪声，或为标准 API 改名和配置删除增加抽象。每项只改依赖声明、精确锁文件和版本要求的直接调用点。
