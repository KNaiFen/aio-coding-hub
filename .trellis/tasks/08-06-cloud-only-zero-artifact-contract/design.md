# AUD-054 技术设计

## 合同模型

以一个无第三方依赖的 Node 检查器作为本地唯一入口。检查器结构化读取 JSON，并以受限的 YAML 缩进/key 解析器只检查工作流中的真实 job、steps 和 `run:` 字段；Markdown、Trellis 模板与脚本使用保守源码合同，统一验证：

1. 本地允许命令白名单；
2. 根/workspace scripts 的 Actions-only 所有权；
3. AGENTS、README 和活跃 Trellis 指引的一致性；
4. CI 原生与前端完整门真实接线；
5. `workflow_dispatch` 与按需 `dev-build` 保留。

检查器导出纯断言函数，self-test 使用内存 fixture 覆盖正例、遗漏和死文本绕过，不需要 `pnpm install`。

## 文件边界

修改仓库规则、README 中英文、根与两个 workspace package.json、检查聚合器、CI、Trellis agent 模板和仍活跃的 cross-layer 规范。新增独立 cloud-only contract 规范与零依赖 checker/self-test。历史任务、归档、release/dev-build 资产矩阵和依赖版本不变。

## 兼容与回滚

云端 job 仍可调用现有 scripts；脚本只移除“本地支持”语义，不删除 CI 能力。回滚本 PR 会恢复旧的本地入口，不产生数据迁移。
