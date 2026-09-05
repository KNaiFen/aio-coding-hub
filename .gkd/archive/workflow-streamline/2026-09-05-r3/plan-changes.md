# 方案变更

## r4：首轮 CI 修复

- PR #190、run `33960979398` 的 frontend、rust、pr-title 成功；sync policy selftest 在 Linux 重新打开 Node 管道对应的 `/dev/stdout` 失败。
- 测试 summary 改为直接重定向到已有 stdout 描述符；继续验证实际 workflow 步骤及原有成功/失败断言，无文件或网络操作。
- 外部监控 `--pr` 等待合并而非 CI，已终止；后续用同一 Skill 的 `--run <id> --interval 30 --timeout 3600` 跟踪本 PR 新 head 的明确 CI run。
- 用户的推送、合并、清理授权持续有效；修复仍在原 selftest 范围，不修改外部 Skill。

## r2：实现授权

- 将 r1 只读调查落为现行 GKD 交接、文档 CI 分类、提交/发版说明和 upstream 状态语义四项实现。
- DIRTY/UNKNOWN 成功并警告，空状态及真实命令错误继续失败；保留签名候选晋升、分支保护与本地零产物规则。
- main 单写者在独立 worktree 完成 execution r2，本地中文提交按用户规则执行。

## r3：交付授权

- 用户明确授权推送、合并和清理；实现范围未改变。
- 增加任务 PR 的只读 CI 监控、脱敏归档、cleanup commit 和已合并任务分支/worktree 清理。
- main 同步远端合并结果时保留原有独有历史；不推送远端 main。
