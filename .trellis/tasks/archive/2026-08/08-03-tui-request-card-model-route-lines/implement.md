# 实施计划

1. 增加请求卡语义行类型与专用模型行格式化 helper。
2. 将普通卡构造改为动态 Vec，并为实际模型变化插入目标续行。
3. 将 UI 配色改为按语义 kind 选择，保持选中背景和分隔线。
4. 补充精确文本、Unicode 宽度、普通/路由混排和配色测试。
5. 更新本地 Observer TUI 合同；Rust 格式和测试只交由 GitHub Actions。

## 实施记录（2026-08-03）

- `request_card_lines` 改为携带 `RequestCardLineKind` 的动态行集合；实际模型路由变化新增源模型箭头行和 Unicode 显示宽度右对齐的目标行，普通/同模型路由保持单模型行。
- 请求卡使用紧凑 `·effort`，Statusline 与详情继续由 `request_model` 保有旧的 `·思考` 语义；UI 按语义 kind 配色并支持 5/6 行卡片混排。
- 对旧观察器或异常可选路由字段做显示宽度、思考强度和策略来源校验，无效数据回退原始请求模型。
- 已更新本地 Observer TUI 跨层合同，并补充格式、极窄宽、无效路由、混排和语义配色回归测试；未修改 Observer 协议或网关路由逻辑。

## 独立检查记录（2026-08-03）

- 检查代理复核 PRD、设计、实现清单及引用规范；发现并修复无效 `configured_model_route` 回退缺口，无开放阻塞项。
- `git diff --check` 与 Trellis 全量上下文校验通过；Rust/Cargo、rustfmt、Clippy 和 Tauri 命令按仓库规则留给 GitHub Actions。

## 交付证据（2026-08-03）

- 实现提交 `cd8af240`，后续格式与 Clippy 修正提交 `25fe7681`、`c5b5c354`。
- 功能 PR #24 合并提交 `52aca8daf4f8480c22db3033683afee1abe2efe1`，完整 CI `30795974057` 成功。
- 已随 `aio-coding-hub-v0.60.45` 发布；精确 main 候选 CI `30799895208`、发布工作流 `30802809954` 与 Release 资产校验全部成功。
