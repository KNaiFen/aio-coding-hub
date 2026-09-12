# 供应商可靠性 0.60.59 交付归档

- 观察日期：2026-09-08，Asia/Shanghai；本摘要由本轮唯一收尾角色记录，保留原作者 PLAN、execution、review 决定和 progress 历史事实。
- 本轮路线：用户新授权 PR、CI、合并、发版后的 r9-delivery 连续收尾。此前 r8 仅本地归档已完成，本轮未覆盖旧快照。
- 外部交付完成：PR #192 已 squash 合并，合并 SHA 的 main 自动 CI 与签名候选成功，0.60.59 Release 已 published，12 个预期资产完整且非零。
- 本机安装、三个 30 分钟观察、活动释放/休眠和用户主动真实生成尚未执行；本摘要不将云端通过写为现场通过。

## 来源记录

本目录保存五类实际 Markdown 的完整脱敏快照，原活动文件整理后仍可独立读取：

- [plan.md](plan.md)：主工作树 PLAN r9-delivery，包含原行为、范围、验收和新增外部交付授权。
- [plan-changes.md](plan-changes.md)：r9-delivery 授权变化。
- [execution.md](execution.md)：任务工作树 execution r9-exec11 及历史执行交接。
- [progress.md](progress.md)：原施工角色各轮实际结果，保留历史尚未运行 CI 的观察时点。
- [review.md](review.md)：main 于 2026-09-08 作出的最终 PASS 及历次复查/CI 证据。
- [r8 原归档](../2026-09-07-r8/summary.md)：继续表示 2026-09-07 的本地收尾时点，未回写为当时已发布。

## Git 与 PR

- 仓库：`KNaiFen/aio-coding-hub`；远端 `origin`。
- Baseline：`a35f2aa4ea469f6e4066582b2e969f1ec44fca2e`。
- 任务分支：`fix/provider-reliability`；最终已审且已推 PR HEAD：`845589f8bf72df70022a6c1e7a85132accd4b266`。
- [PR #192](https://github.com/KNaiFen/aio-coding-hub/pull/192) 于 `2026-09-07T16:26:25Z`（北京时间 00:26:25）合并，真实 squash SHA：`b147ced0f8cab342ee57925a5bd6b08d37e061db`。
- 实际合并使用 `gh pr merge 192 -R KNaiFen/aio-coding-hub --squash --match-head-commit 845589f8bf72df70022a6c1e7a85132accd4b266`，exit 0；未删除分支或使用管理员参数。
- 已 `git fetch origin main`，任务工作树成功 `checkout --detach` 到真实合并 SHA。保留主工作树 `main` 的独有历史，HEAD 仍为 `193767510ef647193ce5f16390bc1f663c3dffb0`；未 reset、推送 main 或把旧任务分支合回本地 main。
- PR 说明已用最终 PR CI 成功事实更新，更新 exit 0。复用已审 [PR CI run 34141052395](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34141052395)，未重复运行 PR CI。

## Main CI 与候选

- [main CI run 34143273546](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34143273546/attempts/1)，attempt 1，`ci.yml`，event=`push`，head_branch=`main`，head_sha=`b147ced0f8cab342ee57925a5bd6b08d37e061db`，completed/success。
- 唯一 commit watcher 于 `2026-09-07T17:11:41Z` 返回 success、exit 0；expected `ci-gate`、`contracts`、`frontend`、`rust`、`observer-macos`、`assemble-release-candidate` 全 success，无缺失、无失败。
- 同 run/attempt 的 change-scope、candidate-plan、两平台桌面签名候选、四平台 TUI 候选均 success；manual-dispatch-guard 为自动 push 下预期 skipped。main 不要求 pr-title，未手动 dispatch 常规 CI 或补建候选。
- Artifact ID：`10027849625`；名称：`release-candidate-b147ced0f8cab342ee57925a5bd6b08d37e061db-34143273546-1`；大小 `73931057` bytes；API 观察 `expired=false`。
- Artifact digest：`sha256:bdc9fa29e4ab2f0a994804d8e7de6438c6b220eaa65e3d641f0f18331cb9550c`；实际 API 到期时间 `2026-09-08T16:26:28Z`。归档不保证 GitHub 临时候选长期保留，正式 Release 是已发布制品来源。

## 标签与 Release

- 新标签：`aio-coding-hub-v0.60.59`；annotated tag object `ad8ea364eb56d66d215f947ad3a32c206deb708b`，远端 peeled commit 为 `b147ced0f8cab342ee57925a5bd6b08d37e061db`。
- 创建前本地标签、远端标签均不存在，Release 查询为 not found；普通 `git tag -a` 与 `git push origin refs/tags/aio-coding-hub-v0.60.59` 均 exit 0，未覆盖标签或资产。
- GitHub 接受普通推送时打印既有账号权限绕过 tag creation restriction 的提示；未使用 `--admin`、修改规则或授予权限。该返回原样性质如实保留，不将其表述为修改门禁。
- [release run 34146638896](https://github.com/KNaiFen/aio-coding-hub/actions/runs/34146638896)，attempt 1，event=`push`，branch=`aio-coding-hub-v0.60.59`，同合并 SHA，completed/success。唯一 run watcher 于 `2026-09-07T17:13:10Z` 返回 success、exit 0；随后 jobs/steps 明细核对全部 success。
- [Release 0.60.59](https://github.com/KNaiFen/aio-coding-hub/releases/tag/aio-coding-hub-v0.60.59) 的 `publishedAt=2026-09-07T17:12:58Z`（北京时间 01:12:58），`isDraft=false`，`isPrerelease=false`；targetCommitish 为 main，实际不可变标签解析到上述完整 SHA。
- 发布工作流成功执行来源/版本检查、选择成功 main 候选、下载准确 artifact、候选文件与 SHA256 校验、不可覆盖预检及无需重建的发布。
- 必要日志确认下载 artifact `10027849625`，下载摘要与候选 API digest 一致；SHA256SUMS 中 11 个 payload 文件全部显示 OK。Release API 的 11 个对应资产 digest 与 manifest 一致，checksum 文件本身亦非零。
- 仓库外只下载了 `latest.json` 和 `SHA256SUMS.txt` 小清单；未下载或本地构建/签名安装应用。latest 版本为 0.60.59，Windows x86_64 与 macOS arm64 URL 指向本标签对应 updater 资产，签名字段非空。
- Release 简短中文说明已更新成功，明确本次改动、真实测试用量、Codex OAuth 输出预算例外与现场待验证项；不修改已上传资产。

| 预期资产 | 字节数 |
| --- | ---: |
| aio-coding-hub-win64.msi | 17997824 |
| aio-coding-hub-win64.msi.sig | 428 |
| aio-coding-hub-win64-portable.zip | 18245778 |
| aio-coding-hub-macos-arm.tar.gz | 17448426 |
| aio-coding-hub-macos-arm.tar.gz.sig | 416 |
| aio-coding-hub-macos-arm.zip | 16917090 |
| aio-tui-win64.zip | 910053 |
| aio-tui-macos-intel.tar.gz | 914180 |
| aio-tui-macos-arm.tar.gz | 879552 |
| aio-tui-linux-x64.tar.gz | 948196 |
| latest.json | 1343 |
| SHA256SUMS.txt | 1018 |

## 记录整理与保留

- 五类实际活动记录已复制并脱敏到本目录，真实交付事实先保存在本摘要。
- 活动整理完成：快照与原始记录逐一核对完整一致后，按明确许可移除本任务独占主树 plan/plan-changes/review 与任务树 execution/progress 五个文件，未保留空模板。
- 本目录在主工作树且位于任务产品分支之外，沿交接保留为本地 Markdown 归档，不混入产品 PR/提交，不为归档重开 PR 或制造空提交。
- 保留本地/远端任务分支、任务工作树、main 独有历史、其他 gkd-project-rules 工作树以及来源不明文件。未获删除这些对象的许可。
- 监控脚本及其后台运行均已终止；不在归档保存会话 job 句柄、个人绝对路径、凭据或全量日志。
- 结束前一次集中核对确认：任务工作树 detached 发布 SHA 且无未提交改动；main HEAD 与本地/远端任务分支均保持上述值；原任务 PR HEAD 与 squash SHA 的完整文件树 diff exit 0、无差异；三个工作树仍保留。main status 仅本任务归档目录未跟踪，没有为归档创建产品提交。
- 同次核对确认五个活动路径均不存在、r9 六份归档及旧 r8 快照均可访问，个人路径/角色句柄筛查无匹配；summary 的 no-index 空白检查没有诊断输出（exit 1 表示与空文件有内容差异）。最终 Release 仍 published，12 个预期资产均 uploaded 且非零，中文说明已生效。

## 待现场验证

1. 新版本 AIO 供应商页可见、供应商页后台、关闭到托盘分别至少 30 分钟，按正常 TUI 节奏检查供应商和请求日志持续可用、无 1500 ms 整体 DB 超时或 OBS_BUSY；列表与账户用量分别记录。
2. macOS TUI 使用活动建立、全部 TUI 退出且无测试后 15 秒撤销，以及系统正常空闲休眠。
3. 用户主动进行真实合法模型成功验证；明确无效模型的失败按原 PLAN 和现有可临时配置能力验证，不能擅自修改持久化配置。
4. Codex OAuth 兼容路径不能承诺服务端 100 token 硬上限。云端 mock 与签名候选成功不代表真实供应商或全部用户现场已通过。

## 后续清理

- 观察时点：2026-09-08 09:18，Asia/Shanghai。用户在发布完成后明确授权“清理掉worktree和分支”；此为新增删除许可，上文未获许可而保留的记录仍表示原观察时点。
- main 交接已确认原 writer、验收及收尾角色全部停止；任务工作树 detached 于 squash SHA `b147ced0f8cab342ee57925a5bd6b08d37e061db`，无未提交或未跟踪文件。删除前补查 `git ls-files --others --ignored --exclude-standard`，exit 0、无输出，没有需保留的忽略文件。
- 复用 PR #192 的 MERGED、PR HEAD `845589f8bf72df70022a6c1e7a85132accd4b266`、上述 squash SHA 及两个完整 SHA 文件树 diff exit 0 的证据，确认任务成果已经合入远端 main。
- 从保留的主工作树执行普通 `git worktree remove`，已删除 `provider-reliability` 工作树，exit 0，未使用 `--force`；随后精确执行 `git branch -D -- fix/provider-reliability`，exit 0，删除原指向 PR HEAD 的本地任务分支。
- 删除前 `git ls-remote --heads origin refs/heads/fix/provider-reliability` exit 0、无输出，服务器任务分支已不存在，没有发送远端删除请求。陈旧跟踪引用仍指向上述 PR HEAD，已通过绑定该完整旧 SHA 的 `git update-ref -d refs/remotes/origin/fix/provider-reliability` 精确删除，exit 0。
- 保留主工作树及其独有历史、`gkd-project-rules` 工作树和分支、全部发布标签、r9 六份归档及 r8 历史归档；未触及其他或来源不明文件。本次仅进行 Git 清理与本摘要事实追加，没有新增产品提交、PR、CI 或本地测试/构建，原待现场验证事项继续保留。
