# CI 调度与归档成本优化收尾事实

## 归档观察时点

- 时间：2026-09-08（本次收尾开始时）；依据已批准 PLAN r1、execution r2、progress 与代码审查记录。
- 本目录保存本任务实际的 plan、execution、progress、review 和本摘要；本轮没有 `plan-changes`，因此未创建空文件。
- 本机绝对路径与运行时角色标识已从归档副本脱敏；保留分支、提交、GitHub 对象和验证事实，未包含凭据或运行日志。
- baseline：`692da663ce0eaac2569e2c74eff78ee0c2317118`；已审查实现 HEAD：`82bc305834c601a3c2acb45cc61850d19cb02f34`。

## 已有结论与待验证项

- 实现覆盖 CodeQL 纯文档分类、主 CI contracts 前置和治理/云端合同同步；独立审查通过，原 F1 已由第二个实现提交解决。
- 本地实际完成 `git diff --check` 与两份 MJS 的 `node --check`，均成功；未安装依赖或执行包脚本、测试、构建或打包。
- 本归档将与两项已审查实现提交合批推送为最终待验证 PR head。此时所有实际 GitHub Actions、PR、合并和 main 验证均待定，不能用旧 HEAD 结果替代。
- 后续应以 PR 与 Actions 的真实记录确认 required `ci-gate`、`pr-title`，以及 contracts、frontend、rust、observer-macos、CodeQL JavaScript/TypeScript 与 Rust；CodeQL 两项必须实际执行成功。合并后还需确认 main merge SHA 的自动 CI 与双语言 CodeQL。

## 授权与清理边界

- 用户的 automatic 授权覆盖本地提交、推送、PR、正常 squash 合并、必要自然 CI 等待、归档和本任务默认清理；不包含业务实现、平台设置、手动 dispatch、retry 或 cancel CI、版本、tag 或 Release。
- 最终事实应按实际观察时点写入主工作树同目录，引用 PR 与 Actions；不为回填归档自身 SHA 或 CI 再创建提交。
- 只有成果真实进入 `main`、归档独立可访问且任务现场无待保留内容后，才可删除本任务执行 worktree 及本地/远端 `ci/scheduling-cost` 分支。主工作树既有历史与其他任务改动不属于本任务清理范围。
