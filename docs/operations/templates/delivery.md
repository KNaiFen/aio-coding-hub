# 交付：{{任务名称}}

> 只记录实际实现；实时 PR/head/CI 以 GitHub 和任务 receipt 为准。

- 结果：{{完成 / 部分完成 / 失败 / 阻塞}}
- 实际修改：{{文件、符号和行为}}
- AC 证据：{{逐项结果}}
- 方案偏移：{{无 / 原因与影响}}
- 本地验证：`scripts/gkd-verify --base-sha <full-lowercase-sha>` {{结果}}
- 云端验证：{{检查名称与结果}}
- 剩余风险：{{无 / 描述}}

保持 worktree clean 后提交 delivery digest、candidate output bundle digest 和完整 head；随后停止写入。
