# 本地与远端提交历史同步

- Revision: r1，2026-09-12；状态：已获准执行；路线：direct-main。
- 授权：用户要求“把本地/远端的提交记录同步，整理干净”，沿用此前合并、推送许可。本轮包括文档归档、任务 PR、正常合并、本地主分支对齐，以及已交付旧分支清理。
- 仓库：KNaiFen/aio-coding-hub；远端：origin；目标：main；指定 checkout 为主工作树 main。
- 初始本地：251dc18591c972d1c0ddede641741f93f728cd09；远端：c3097864ce54af7afff6252e1c558776b56c148a。本地独有 34 个提交，远端独有 0 个。

## 事实与处理

1. 业务文件已与远端完全一致。提交树差异仅为旧 .gkd/plan.md、plan-changes.md 和根级 progress.md、review.md。33 个旧提交的功能已通过 PR squash 交付；第 34 个是同步 merge。唯一未交付净内容包括 0.60.58 发布完成记录，另有未提交的旧任务归档。
2. 先在仓库外保存可恢复 Git bundle、全部 .gkd 与根级过程记录快照、原未提交补丁；已验证 bundle 完整且旧 stash 对象可从独立 bare 副本读取。保留两个既有 stash，不修改其引用或内容。
3. 从当前现场切出 docs/sync-local-history，再仅将这个新分支的 HEAD 用 soft reset 指向 origin/main，保留 index 和全部工作文件；原 main 保持不动直到交付成功。新提交因此只携带文档净差异，不把 34 个旧提交再次推入共享主线，不强推远端。
4. 将根级 plan/progress/review 迁入 .gkd/archive/release-0.60.58/2026-09-05-final/，保留本地发布完成事实；修正迁移后相对入口链接。旧 .gkd 活动计划与变更已在 #199/#196 归档中，以现有归档为准，原始版本留在 bundle。
5. 合批现有 model-price-rules、project-rules-gkd、provider-reliability、ci-test-initialization 的本地记录。将 closeout-preserved-before-sync 的 CI 调度摘要存为对应任务 final-observation.md，原 SHA 清单存入仓库外备份后移出活动目录。保留每份历史记录原观察时点及未完成的现场验收，不将归档写成新验收。
6. 本轮活动 PLAN/review 由一个 gkd_closeout 归档到 .gkd/archive/local-remote-history-sync/2026-09-12-r1/，与现有记录一次提交、一次推送。创建面向 main 的 PR，绑定已验证完整 head 正常 squash 合并。交付后清洁切换到远端提交并更新本地 main；原始 HEAD 可由仓库外 bundle 恢复。
7. 9 个已证实整树等价于已合并 PR 的旧本地分支在备份与交付成功后删除：chore/github-actions-governance、chore/pin-nanoid-3.3.18、chore/release-0.60.51、docs/record-nanoid-fix-merge、docs/record-release-0.60.51、docs/unify-agent-rules、docs/worktree-session-handoff、fix/estimated-rate-marker、test/token-speed-estimate-submit。同时清理本轮临时与交付分支，保留主工作树。
8. 远端 Dependabot 分支和 task/aio-gkd-bundle-adapter 没有本轮可删除的交付证据，保留。upstream、发布标签、stash、忽略文件与运行时引用不在删除范围。

## 验证与终点

- 本机实测为 Apple M4 MacBook Air、16 GB 内存，磁盘 228 GiB、可用 99 GiB。只运行轻量 Git/gh、归档与内容比较；不运行依赖安装、package scripts、测试、lint、构建或生成。
- 本地检查：备份可恢复、来源文件逐项保全、git diff --check、最终差异只含过程 Markdown、产品树与 c3097864 相同、9 个旧分支与对应 PR 整树相同且 PR merge 是 origin/main 祖先。
- PR 使用现有自动 Actions，required 为 ci-gate、pr-title。纯过程 Markdown 应只运行分类和门禁，重任务按既有合同跳过。合并后等待实际 merge SHA 的自动 ci-gate；不手动追加源码测试。
- 成功标准：PR MERGED，最终本地 main、origin/main、服务器 refs/heads/main SHA 一致，ahead/behind 为 0/0，工作区无未提交记录，旧分支按已验证清单清理，两个 stash 原 SHA 保持。
- 最终交付/CI/清理事实写到仓库外备份目录的 delivery.md 并在会话报告，不为回填自身结果另造提交或留下新的脏文件。

## 消融审查

只同步真实独有记录和 Git 历史归属，不改业务、测试、依赖、工作流或全局规则；不新增恢复工具、验证脚本或重复归档模板。
