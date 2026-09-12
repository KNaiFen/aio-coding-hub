# CI 调度与归档成本优化 Review

- 当前结论：代码审查通过，可进入收尾准备；F1 已解决，最终云端验证待定，不代表整体交付完成。
- PLAN：r1，已批准 automatic；execution：r2。
- baseline：`692da663ce0eaac2569e2c74eff78ee0c2317118`。
- reviewed HEAD：`82bc305834c601a3c2acb45cc61850d19cb02f34`。
- 2026-09-08 18:58（北京时间），两轮执行均停止；独立 gkd_accept 首轮检查六文件，次轮只复查 F1 的两文件差异，未运行测试。main 接受复查，无新范围或授权决定。

## Finding F1

P2：`scripts/check-ci-quality-gates.mjs:586` 只检查 analyze 的 action 列表，其 checkout 未检查无条件执行/不可忽略失败。加 `if: false` 或 `continue-on-error: true` 不会被合同拒绝。当前 workflow 正确，但 PLAN 3.2 明确要求三个步骤都必须执行。

历史 r1 结论暂不通过，已由 r2 取代：复用既有 `stepRunsUnconditionally` 约束 analyze 的首个 checkout，现有 selftest 添加两个负例。仅改原允许的 checker/selftest，14 行新增、1 行删除；workflow、解析器、PLAN 目标和授权未改变。独立复查确认先验证 action 序列再访问首步，两个负例准确针对 analyze checkout；F1 已解决，无新增 findings。

## 已有证据与未完成项

- 实现的两份 MJS `node --check`、`git diff --check` 成功；checker/selftest 本体与实际 Actions 尚未执行。
- 独立审查其余范围未发现问题：分类输入与双语言选择、三个重任务合同依赖、原 gate 闭包、归档顺序符合 PLAN。
- 收尾合批后的最终 head 仍需自然运行全量主 CI、双语言 CodeQL、required ci-gate/pr-title；main merge SHA 的验证、交付、归档和清理尚未完成。

## 收尾决定

批准按 PLAN 第 6 节一次交接收尾：先归档当前已知材料，再合批推送最终 head 并创建 PR；等待 required 与 expected 检查通过，绑定已验证完整 head 正常 squash 合并，再等待 main 自动 CI/CodeQL。只归档与清理本任务，其他任务脏文件和本地主干独有历史保留。最终事实在保留的主工作树归档补记，不递归提交。
