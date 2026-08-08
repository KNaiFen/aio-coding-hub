# 最终修复与本地零产物：总体设计

## 交付拓扑

既定顺序用于约束依赖和历史证据，但不再要求八个实现 PR 严格串行。AUD-054、AUD-056、AUD-016、AUD-008 已分别进入主线；AUD-055、AUD-002、AUD-035、AUD-033 在最新 `origin/main` 上汇集到 `codex/final-hardening-unified`，由一个 PR 作为唯一云端验证、审查和合并表面。报告、PENDING 与 Trellis 拓扑随该 PR 更新，不再另建纯文档 PR。

## 共同边界

- `origin/main` 是唯一仓库操作基准，不读取或写入 `upstream`。
- 本地验证只执行不会安装依赖或生成产物的 Node 源码合同、解析和 Git 差异检查。
- 所有原生与前端完整验证均由精确候选 SHA 的 GitHub Actions 完成。
- 兼容主线漂移时重放并重验；无法共存的语义冲突保留候选并登记为待决策。
- 报告、PENDING 和 Trellis 任务是同一交付链的持久事实源。

## 依赖关系

- AUD-054 是后续所有任务的验证前置。
- AUD-008 先提供应用级维护态，AUD-002 再复用它执行恢复 journal 对账。
- AUD-056 保留 usage ledger 的永久统计语义，不与 AUD-002 的文件系统 journal 合并。
- AUD-016 的 gateway token、AUD-035 的 Observer 查询和 AUD-033 的插件生命周期相互独立。

## 回滚

已合并项目保留各自的主线回滚边界；剩余四项的统一 PR 是一个集成回滚单元。若统一候选无法通过 CI 或出现不可兼容主线漂移，不合并；保留统一分支、证据和明确决策点。旧 PR 只在统一 PR 已创建且精确 head 的 Actions 已启动后关闭。
