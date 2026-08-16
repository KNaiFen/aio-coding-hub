# 执行与交付

本页供独立执行 session 使用。先调用 `$gkd-execute`，再按 `execution.md` 给出的顺序读取任务材料；不要预加载 main 的验收和收尾流程。

## 开工核验

```bash
python3 .trellis/scripts/task.py status <task>
python3 .trellis/scripts/task.py doctor <task>
```

同时确认 `pwd -P`、当前分支、唯一写者和 `execution.md` 一致，`prd.md` 已授权实施，没有未关闭的材料性问题。任一项不成立即停止；不要从聊天猜测。

返工必须先读 `findings.md` 中当前未解决项。main 提交整改意见后运行 `task.py start <task> --writer <execution-session>`，把 `delivered` 转回 `implementing` 并显式交回写权；处于 `blocked` 时必须先由负责人执行 `resume`。

## 施工

- 尽早创建指向 `main` 的 Draft PR；按锁定范围实现、提交和推送。
- 同步任务要求的代码、测试、现行文档、机器合同和迁移材料。
- 可以修复自己 PR 的 CI/编译问题；疑似基础设施或 main 既有问题且无可靠任务内修法时保留证据并交 main。
- 不推送 `main`、不合并、不自动合并、不归档、不清理 worktree。
- 产品决定、公共接口、兼容性、安全边界、迁移或重要范围需要改变时，先暂停并由 main 获取用户决定。

## 阻塞

先在 `delivery.md` 的阻塞区记录当前证据、最后安全提交、工作树状态、受影响 AC、决定归属和恢复条件，再执行：

```bash
python3 .trellis/scripts/task.py block <task> \
  --reason "<reason>" \
  --resume-condition "<condition>" \
  --owner <main-or-other-owner>
```

提交可恢复的状态后暂停。没有可提交改动、PR 或 CI 时如实写“无/未触发及原因”，不制造空提交或虚构证据。恢复由负责人运行 `resume --writer <execution-session>`；涉及范围、行为或 AC 的变化必须先更新权威计划并重新获得所需确认。

## 交付报告

从[交付报告模板](../templates/delivery.md)创建 `delivery.md`，只写实际事实：

- 用户可见和内部行为。
- 关键文件、模块和符号。
- 每条 AC 的结果与证据。
- 与计划的偏移和原因。
- 实际运行的本地、云端和人工验证；未运行项及原因。
- 配置、数据、API、兼容性、安全、发布、回滚和剩余风险。

不要复制计划，不写候选 head/base/CI URL。GitHub PR 是这些实时事实的来源，提交 `delivery.md` 会产生新的 head。

## 交付转换

先提交并推送范围内实现、测试、文档和 `delivery.md`，保持 worktree 干净，然后运行：

```bash
python3 .trellis/scripts/task.py deliver <task>
git add .trellis/tasks/<task>/task.json
git commit -m "chore(workflow): 标记任务等待验收"
git push
```

等待这个最终 head 的必需 CI 和相关编译终态；按范围跳过的 job 在 `delivery.md` 解释。CI 绿色、需要的人工验证完成或明确交由 main 后，把 PR 标为可评审并暂停。通知 main 时只发送任务路径和 PR URL；main 从 GitHub 读取当前完整 head、base 和检查。

任何新 push 都会使先前的验收结论失效。main 验收期间不得继续写入，只有 main 明确恢复任务后才能返工。
