# 依赖升级 PR 审查与交付 PLAN

- Revision: r1，2026-09-12；状态：已获准执行；路线：direct-main。
- 用户授权：关闭长期冲突的上游同步 PR；审查现有依赖升级 PR，确认没有问题后合并，并保持本地与远端同步。审查发现的明确升级兼容问题允许在原依赖升级范围内作最小修复，修复后以新 head 的自动 CI 作为“没有问题”的必要证据。
- 仓库：KNaiFen/aio-coding-hub；远端：origin；目标分支：main；指定本地 checkout：当前主工作树。
- 起始基线：d50ecdb0cbb97287b29b88960605a7353d90b326，本地 main、origin/main 与服务器 main 一致，工作区起始干净。

## 已完成审查

1. PR #113 是 2026-08-11 创建、2026-09-08 最后更新的跨仓上游同步 PR，head 为上游 0.60.19，当前与 main 冲突。用户明确要求关闭，已于 2026-09-12 关闭。
2. PR #110：只更新 mac-notification-sys 补丁目录的独立 Cargo.lock；应用根锁已使用 time 0.3.47。可作为锁文件同步更新，但不声称升级应用运行时依赖。
3. PR #111：PostCSS 8.5.23 补丁升级；项目仅经 Vite/Tailwind/Autoprefixer 使用，Node 22 与 peer 范围兼容，旧 head 全部检查成功。
4. PR #112：Vite 7.3.5 补丁升级及真实传递锁文件更新；Node 22 和 plugin-react 兼容，现有配置无需迁移，旧 head 全部检查成功。
5. PR #120：tar 0.4.46 修复升级本身兼容，但旧 lock diff 混入 10 处 Windows 依赖解析变化。更新到当前 main 时只保留 tar 所需最小锁文件变化，避免扩大验证面。
6. PR #121：rand 0.9.4 满足 Rust 1.90，但 15 处 thread_rng 已在 0.9 弃用，当前 Clippy -D warnings 会阻止合并。全部替换为官方新名称 rng，保持随机语义和既有 trait 调用。
7. PR #122：serde_with 3.22.0 是 Tauri 传递依赖，MSRV 1.88 满足 Rust 1.90，旧 head 全部检查成功。
8. PR #197：Vitest 4.1.11 方向兼容，但 coverage-v8 仍为 3.2.4 且 coverage.all 已在 v4 移除。同步 coverage-v8 为 4.1.11、更新锁文件，并删除 coverage.all；不降低覆盖率阈值。

上述审查由六个互不重叠的只读代理完成；主代理负责方案、准确修改、最终审查和交付判断。旧 CI 只证明旧 head，所有 PR 必须更新到本轮基线或逐次最新 main，并以新 head 自动 CI 重新验证。

## 实施与合并顺序

1. 为 7 个 PR 分别创建临时 worktree 和本地分支，合入届时 origin/main；只写对应依赖升级、必要最小兼容修复及同分支锁文件。保留 PR 编号、来源分支和独立回滚边界。
2. 先处理独立补丁锁文件 #110，再处理根前端依赖 #111、#112、#197，最后处理根 Rust 依赖 #122、#120、#121。后一 PR 更新到前一项真实 merge 后的 main，避免用过期锁文件覆盖已合入升级。
3. 每项本地只运行 git diff --check、结构化 manifest/lockfile 比较和源码检索。MacBook Air 与项目合同禁止本地依赖安装、package manager、Cargo、测试、lint、编译、生成和打包。
4. 每个 PR 将 main merge 与修复提交推送到原 Dependabot 分支，不强推。自动 PR CI 绑定实际新 head；等待 ci-gate、pr-title 以及分类选择的 contracts/frontend/rust/observer-macos，CodeQL 结果如实记录。必要工作流失败先读取日志并在该升级范围内最小修复，不能重跑掩盖问题。
5. 检查成功后，以 --match-head-commit 绑定已验证 head 正常 squash 合并。每次合并后 fetch origin，确认 merge 是新 main，再开始下一项。若出现无法在升级范围内解决的行为不兼容，保留 PR 并停止该项；其他独立 PR 可继续。
6. 最后一个可交付 PR 的最终 head 合批归档本 PLAN、review 与简明 summary 到 .gkd/archive/dependency-pr-maintenance/2026-09-12-r1/，不创建 execution/progress；归档加入后必须由同一最终 head 重新通过必要 CI，不能回填自身最终 SHA 制造递归提交。
7. 全部可合入项完成后，等待最后实际 origin/main 的自动 main CI 成功。主工作树无未知变更时以 ff-only 更新本地 main；清理本轮临时 worktree和本地分支，并 fetch --prune。远端 Dependabot 分支由正常合并流程删除或在确认合并、无新增内容后清理。

## 成功标准

- PR #113 保持 CLOSED；7 个依赖 PR 均有逐项审查结论，未发现未解决问题的 PR 已正常合并，存在未解决问题的 PR 明确保留并报告。
- 每个合并 PR 的最终 head 有适用自动检查成功证据；rand、Vitest、tar 的已知阻塞均在合并前关闭，未降低测试、lint 或覆盖率标准。
- 最终本地 main、origin/main 和服务器 refs/heads/main SHA 相同，ahead/behind 0/0，工作区干净；本轮临时 worktree/本地任务分支已清理。

## 消融审查

每个 PR 只保留依赖版本、真实传递锁文件和该版本要求的兼容修改。不合并无关依赖重解析，不新增兼容层、测试框架、工作流或配置兜底；既有行为测试和云端门禁承担回归验证。
