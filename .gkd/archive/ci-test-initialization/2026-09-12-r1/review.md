# CI 等待与测试初始化优化审查

- 日期: 2026-09-12；PLAN r1（已批准）、execution e1。
- Baseline: `b03b5ab4d57df5b39348744d5e65d6cc835fbe1b`。
- 实现 head: `94ec45e4efc5743500c4053b2d5627af0dc6e33b`。
- writer 已完成本地提交并停止；独立 `gkd_accept` 已于 2026-09-12 02:09:45 UTC+8 返回，工作树干净。
- 当前结论: 代码审查通过，无返工 findings；main 同意进入归档与最终自然 CI。最终 head、PR CI、main CI 仍待验证，当前不是合并许可条件已满足的声明。

## 审查结果

独立审查确认三个 Rust 文件的实现变更均在 tests：固定规则只读共享，每次服务/cfg/cache仍独立；7个低层及15个服务测试、既有输入/断言/ignore保持；真实加载、限额、预热裁剪和其他模块集成测试保留。网关原调用计数断言保持1，只补失败上下文。治理文档与现有workflow、分类器和watcher一致，不改门槛。

e1服务测试数字在实施定位时修正：13个packaged测试，12个复用、1个真实加载，另2个特殊测试；与低层7变1合计减少17次完整初始化。main已批准按实际计数继续并同步PLAN，属于事实纠正，未改变文件、行为、风险和验证范围，无需另立计划变更。

writer的 `git diff --check`、暂存区检查和低负载结构比较通过；独立审查复核已有证据，没有重跑测试。编译、Clippy、运行时和提速尚未证明；原网关非确定失败根因仍未知，本次仅诊断增强。

## 收尾决定

用户“开始执行此PLAN”已批准PLAN第6节交付和默认清理。使用同一个closeout先将已知plan/execution/progress/review与summary归档、迁出执行树活动记录，合批推送最终head；随后验证PR必要检查、绑定完整head正常squash合并、验证实际main merge SHA。最终事实保存主工作树归档，不递归补交CI结果。

正常PR required为ci-gate/pr-title，expected为contracts/rust/observer-macos；如必要机械生成修正使frontend被选中，按实际分类加入。main含代码时expected为ci-gate/contracts/frontend/rust/observer-macos，不加入不存在的pr-title。CodeQL非required且本次不改workflow，不额外等待。全程使用标准Actions，不在本地运行重检查，不额外dispatch/重跑/取消CI。

本任务允许清理已保存的活动plan/review、执行树execution/progress和新建任务worktree/本地远端分支；主树旧plan-changes删除和其他归档全部保留。若CI失败需要业务实现或超出获准操作，保留现场与证据交回main；不得由closeout擅自修代码或宣称已交付。
