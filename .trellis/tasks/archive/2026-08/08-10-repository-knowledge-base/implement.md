# 实施计划

## A. 建立知识入口与生命周期

- [x] 新建 `docs/README.md` 与 `docs/history/README.md`，定义事实源层级、文档状态和维护流程。
- [x] 更新 `AGENTS.md`、`README.md`、`README_EN.md`，加入知识库入口并补齐英文插件说明。
- [x] 调整 `.gitignore`，允许项目文档默认受跟踪，并忽略明确的本地工具产物。
- [x] 修复 `.trellis/workflow.md` 死链接与 `.trellis/workspace/index.md` 漂移。

## B. 迁移与历史化

- [x] 迁移产品基线、健康审计、上游审计、插件日期审计、被替代计划和两份工程笔记。
- [x] 给历史资料增加状态、证据日期和当前入口说明；删除旧 wiki 索引/日志。
- [x] 核对后移除重复的未跟踪健康审计截面。

## C. 修正现行说明

- [x] 统一修正插件平台兼容性、diagnostics API 和版本无关叙述。
- [x] 更新 `scripts/check-plugin-system-docs.mjs`，从锁定 `0.62` 文案改为锁定当前行为合同。
- [x] 修正 protocol bridge README 的本地 Cargo 指令边界。
- [x] 补齐 `CHANGELOG.md` 的 `0.60.33` 至 `0.60.50`。

## D. 清理任务状态

- [x] 解耦 Claude OAuth 候选与已完成父任务，并把候选基线更新为 `main`。
- [x] 归档 17 个已交付任务，保留原始计划、研究和验证记录。
- [x] 新建 `.trellis/tasks/README.md`，说明活跃列表、归档入口和证据维护规则。

## E. 验证与复核

- [x] `node --check scripts/check-plugin-system-docs.mjs`
- [x] `node scripts/check-plugin-system-docs.mjs`
- [x] `node scripts/check-plugin-api-contract.mjs`
- [x] `node scripts/check-cloud-only-verification.selftest.mjs`
- [x] `node scripts/check-cloud-only-verification.mjs`
- [x] 运行无依赖 Markdown 链接/锚点检查。
- [x] `python3 ./.trellis/scripts/task.py validate 08-10-repository-knowledge-base`
- [x] `git diff --check`
- [x] 审阅完整 diff、未跟踪文件和活跃任务列表。

云端保留门：依赖安装、前端完整质量门、Rust、生成绑定和构建仍由 GitHub Actions `ci` 负责，本地不运行。
