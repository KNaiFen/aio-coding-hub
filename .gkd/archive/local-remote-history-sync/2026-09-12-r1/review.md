# 本地与远端历史同步审查

- 日期：2026-09-12；PLAN r1；路线 direct-main；结论：范围与保全方案通过，最终归档提交及自动 CI 待收尾执行。
- 原始 main：251dc18591c972d1c0ddede641741f93f728cd09；基线：c3097864ce54af7afff6252e1c558776b56c148a。
- 主代理已停止写入；收尾角色为后续唯一 writer，依 PLAN 归档、正常 PR 交付和有条件清理。无需业务实现或独立实施验收。

## 已核对证据

1. fetch origin --prune 成功；远端仍为 c3097864，本地旧 main 是其后代且独有 34 提交。原始 diff 只有 .gkd/plan.md、.gkd/plan-changes.md、progress.md、review.md，217 增/11 删；排除这四份记录后的整仓 diff 为空。
2. 复用本轮前序三个已完成历史调查：旧 GKD 清理 11 提交累计补丁进入 #187；Token 速度/迁移/版本通过 #185/#187/#188/#189 交付；规划和同步提交已由后续归档覆盖。0.60.58 完成记录仅本地，必须保存。
3. 主代理抽查：6770888e 与 #187 bf07ec66 的 tree 均为 1113ec035143213dabd8e54159b3bbb266ab065b；808d02f2 与 #189 ca17a2d0 的 tree 均为 5eea05061055ac33a8fdd4726603b856f16ad8ed。再次确认旧 Actions 治理、估算标记、TUI 估算三个分支与相应 PR 整树 diff 为空。
4. 已完成的旧引用调查覆盖全部 9 个待删本地分支，各 HEAD 与对应 PR merge 整树相同，且 merge 是 origin/main 祖先；收尾沿精确 HEAD 核对删除前条件。两个 stash 含旧任务与真实业务快照，保持原样；远端 task/aio-gkd-bundle-adapter 无 PR 对应，保留。
5. history.bundle 校验成功，记录完整历史且无 prerequisites。由其建立独立 bare 副本，cat-file 成功读出旧 stash@{1} 9a6b6f8b；所有 .gkd 和根级记录已另存 tar，tracked 未提交内容另存 binary patch，三项 SHA-256 保存到仓库外 README。
6. 主代理完整读取拟整理的根级发布记录、两份规则续办文档、供应商 r8/r9 PLAN、摘要、CI 最终观察和旧同步保全清单。记录保留历史授权与观察时点，供应商现场验收待验证标记必须保留。剩余归档进度/执行记录由收尾按原文保全与脱敏，不重新验收。
7. 已切到 docs/sync-local-history，再只对这个新分支 soft reset 到 c3097864；原 main 仍是 251dc185。index 与工作文件保留；活动 plan 已更新本轮，plan-changes 的既有删除保持。文档范围外 diff 为空，git diff --check 与 git diff --cached --check 均 exit 0。

## 收尾条件

- 将真实本地记录整理入 PLAN 指定归档；root plan/progress/review 迁入 0.60.58 归档并修复入口链接。清除已归档活动副本和仓库内临时保全目录，原内容由远端归档或外部备份持有。
- 对归档文件逐项核对来源，补做最终 diff --check；新提交只能修改过程 Markdown，产品树不变。自动 PR ci-gate/pr-title 与 merge 后 ci-gate 通过再宣布完成。
- PR 合并后保存 delivery 事实到仓库外备份目录；无未提交内容时 detached 切换实际远端提交，再将未被其他 worktree 使用的本地 main 更新至该提交并切回。既有 34 提交的历史已保存在 bundle，不再以普通 merge 接回主线。
- 9 个旧分支按已验证 SHA 精确清理；保存的 bundle 使原历史可恢复。两个 stash、远端未知分支、Dependabot、标签、upstream 与运行时引用保留。
- 最终集中核对本地/跟踪/服务器 main 同 SHA、0/0、工作区干净、保留 stash 完全相同；不得将后续远端新 tip 归为旧 CI 验证结果。

## 消融结论

没有业务变更、新增测试、抽象或流程工具。只把已有记录移到可长期访问的位置，用一次文档交付消除本地残留；恢复材料留在仓库外，不新增备份分支污染日常分支列表。
