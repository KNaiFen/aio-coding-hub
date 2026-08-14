# 实施计划：TUI 请求时间与 CLI 监听切换修复

## 0. 开工与 Draft PR

1. 完成 `execution.md` 全部 preflight，只在登记 sibling worktree 和任务分支操作。
2. 按顺序读取任务材料、context specs 和即将修改的源码/测试。
3. 确认 `chore/trim-redundant-tests` 没有产品文件重叠；不要进入或修改其 worktree。
4. 推送任务分支并尽早创建指向 `main` 的 Draft PR，正文链接任务目录。不要额外手动触发常规 CI。

完成信号：Draft PR 存在，当前唯一写者仍是本 execution session。

## 1. 恢复 TUI 状态时间

1. 在 `request_card_lines` 中引入按 `ObserverRequestState` 选择的卡片时间值：Active 使用 `duration_ms`，Terminal 使用 `ttfb_ms`。
2. 保留终态 TTFB、窄屏截断、紧凑路由计数、详情页和 output rate 逻辑。
3. 扩展邻近 `format.rs` tests：Active duration、Active 防御性 TTFB、Terminal TTFB、Terminal 无 TTFB、Active 不显示 output rate。
4. 仅在实现确实修改 observer projection 时才补 snapshot tests；不要为本修复改协议字段。
5. 更新 `local-observer-tui-contract.md`。

完成信号：测试能在修改前复现 Active 显示 `—`，修改后固定状态选择且终态语义不变。

## 2. 消除 gateway lifecycle 自锁

1. 将 `sync_cli_proxy_for_settings` 分为加锁 wrapper 与不重复获取 lifecycle lock 的核心 helper，命名遵循现有 `*_unlocked` 风格。
2. 在 `settings_set_impl_with_gateway` 已持 guard 时调用 unlocked 核心；没有 guard 的路径继续调用加锁 wrapper。
3. 保留 status reread、base URL 决策、CLI proxy sync、失败日志和返回布尔语义。
4. 不提前释放外层 guard 来换取表面通过，除非能证明 gateway convergence 与 proxy sync 的串行事务仍完整；任何设计偏移写入 `delivery.md` 并说明。
5. 增加 timeout 行为测试，覆盖已持 lifecycle lock 的同步路径不会再次等待自身；尽可能覆盖运行中 gateway 的双向 listen mode 变化。
6. 复核 settings ownership/rollback focused tests，不改变 CAS owner 或恢复条件。

完成信号：不存在同一调用链二次 lock，测试可在旧实现上超时、在新实现上有界完成。

## 3. 收敛监听模式 UI 状态

1. 将 `NetworkSettingsCard` 的 render-phase draft reset 移到 effect 或等价安全同步点。
2. `commitListenMode` 对 `null` 和 error 都回滚到 canonical settings；成功使用返回 settings 同步 draft。
3. 保存期间展示紧凑、可访问的 applying 状态，并继续禁用产生冲突的监听控件。
4. 成功、失败、`null` 后 pending 状态必须结束；成功关闭/确认 token dialog 后可以再次选择 `localhost`。
5. 添加 deferred promise tests，验证 pending、成功、`null`、error 和 lan -> localhost。

完成信号：UI 不会在保存失败后伪装成另一监听模式，也不会在 mutation 已结束后保持不可操作。

## 4. 建立单一 token reveal owner

1. 把 token dialog/controller 提升到跨 General tab 存活的 page/data-model 层；根据现有结构选择最小清晰的 props/hook 边界。
2. 初始恢复和保存成功共用 serialized/deduplicated reveal；删除 `NetworkSettingsCard` 中无协调的 mount reveal。
3. dialog 在 page-level 渲染，tab 切换/卸载不丢 token state；不要缓存或记录 token 到持久化存储、URL、日志或 query key。
4. 迁移 rotate/copy/acknowledge/error feedback，保持 close-without-ack 需要 rotate 的现有文案与语义。
5. 扩展 `NetworkSettingsCard` 和 `CliManagerPage` tests：tab 卸载期间 deferred reveal、单次 reveal、复制、确认、轮换、关闭后控件恢复。

完成信号：保存 LAN 后同一次交互显示 token；无论 tab 是否切换，后端一次性明文只被一个 owner 消费并能展示。

## 5. 固化安全与跨层合同

1. 新建 `.trellis/spec/aio-coding-hub/cross-layer/gateway-listen-token-contract.md`。
2. 写清 lifecycle lock owner、listen rebind 有界完成、非回环鉴权、digest/明文边界、one-shot reveal、page-level owner、失败回滚和必测矩阵。
3. 更新 cross-layer `index.md` 的 topic、pre-development checklist 和 quality check。
4. 不改历史审计/归档记录；任务文档记录实际偏移，现行 spec 只写合并后应成立的规则。

完成信号：未来修改 listen/token/settings UI 时可从 index 找到唯一现行合同。

## 6. 本地允许验证与提交

本地只允许无依赖、非写入检查：

```bash
node scripts/check-cloud-only-verification.selftest.mjs
node scripts/check-cloud-only-verification.mjs
node scripts/check-spec-links.mjs
python3 ./.trellis/scripts/task.py validate .trellis/tasks/08-14-tui-duration-cli-listen-fix
git diff --check origin/main...HEAD
```

禁止运行 pnpm/npm/yarn、Vitest、Cargo、Rustfmt、Clippy、构建、生成、dev server、Tauri、签名或打包。按逻辑切片提交，建议顺序：TUI；backend lock；frontend token/state；spec/delivery。不得 amend。

## 7. CI、交付与暂停

1. 推送候选，等待自动 `ci-gate` 与 `pr-title`；本任务跨 frontend/Rust，应为 full scope。
2. 检查 CI 中 TUI/Rust focused tests、完整 Rust suite、frontend unit/typecheck/lint/coverage/build，candidate/release 对 PR 按设计跳过。
3. 只监控同一完整 PR head，常规 3-5 分钟一次，最长 60 分钟；失败只修任务范围内问题。
4. 真实桌面手工验证若当前环境不可用，在 `delivery.md` 明确交给 main/用户，不虚构结果。
5. 绿色后将 PR 标记 Ready for review，完整填写 `delivery.md`，然后停止写入并通知 main 验收。
6. 不得 merge、auto-merge、archive、删除 worktree/branch 或运行 `/trellis:finish-work`。
