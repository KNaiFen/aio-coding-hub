<div align="center">
  <img src="public/logo.jpg" width="120" alt="AIO Coding Hub Logo" />

# AIO Coding Hub

**本地 AI CLI 统一网关** — 让 Claude Code / Codex / Gemini CLI 请求走同一个入口

[![Release](https://img.shields.io/github/v/release/KNaiFen/aio-coding-hub?style=flat-square)](https://github.com/KNaiFen/aio-coding-hub/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey?style=flat-square)](#安装)

简体中文 | [English](./README_EN.md)

</div>

> **致谢** — 本项目借鉴了 [cc-switch](https://github.com/farion1231/cc-switch)、[claude-code-hub](https://github.com/ding113/claude-code-hub)、[code-switch-R](https://github.com/Rogers-F/code-switch-R) 等优秀开源项目。

> **Fork 说明** — 本仓库是个人 fork，主要用于 `vibe coding`、试验和随手折腾。代码可能随时改动，**不保证任何可用性、稳定性或兼容性**，也不适合默认用于生产环境；如需参考原始能力，请以 upstream 仓库为准。
>
---

## 为什么需要它？

| 痛点 | AIO Coding Hub 的解决方案 |
|------|--------------------------|
| 每个 CLI 都要单独配置 API | **统一网关** — 所有 CLI 走 `127.0.0.1` 本机入口 |
| 上游不稳定时请求失败 | **智能 Failover** — 自动切换供应商，熔断保护 |
| 不同场景需要不同的供应商组合 | **排序模板** — 多套组合按 CLI 激活，一键切换 |
| 不知道用了多少 Token 和花了多少钱 | **全链路可观测** — Trace 追踪、用量统计、花费估算 |
| 不同项目需要不同的 Prompts / MCP 配置 | **工作区隔离** — 按项目管理 CLI 配置，一键切换 |

---

## 产品截图

### 首页 — 热力图、用量趋势、活跃 Session、请求日志

![首页](public/screenshots/home.png)

### 用量 — Token 统计、缓存命中率、耗时、花费排行

![用量](public/screenshots/usage.png)

### 模型验证 — 多维度渠道鉴别与供应商验证

![模型验证](public/screenshots/modelValidate.png)

---

## 核心功能

### 网关代理

- 单一入口代理 Claude Code / Codex / Gemini CLI 请求
- 首页每个 CLI 独立代理开关，一键启停
- 自定义模型名称映射
- SSE / JSON 响应自动修复

### 智能路由与容错

- 多供应商优先级排序 + 自动故障转移
- 熔断器模式（可配置阈值与恢复时间）
- Sticky Session 保持会话粘滞
- 排序模板：多套供应商组合，三个 CLI 各自激活
- 模板内拖拽排序、独立 enabled 开关、切换即时生效

### 用量与可观测

- Token 用量统计（按 CLI / 供应商 / 模型维度）
- 花费估算 + 模型价格自动同步
- 请求 Trace 与实时控制台日志
- 请求热力图（按时段分布）
- 缓存走势图：分供应商命中率折线，60% 预警线
- 可用率：供应商时间线点阵，15s 自动刷新

### 工作区管理

- 按项目隔离 Prompts、MCP、Skill 配置
- 工作区对比、克隆、切换与回滚
- 配置自动同步到各 CLI

### Skill 市场

- 从 Git 仓库发现并安装 Skill
- 仓库管理、过滤、排序
- 关联工作区批量管理

### 插件系统

- 官方内置插件：Privacy Filter
- Extension Host 插件：命令、Provider 扩展值、网关 hook、协议桥骨架、宿主渲染 UI
- 插件权限、配置 schema、审计日志、启用 / 禁用 / 卸载
- SDK 与脚手架：`@aio-coding-hub/plugin-sdk`、`create-aio-plugin`

插件作者应从 [插件开发手册](docs/plugins/README.md) 开始。社区插件统一使用 Extension Host；旧的预发布规则 / WASM / 进程运行时只作为不支持的迁移历史处理。

### CLI 管理

- Claude Code 设置直接编辑
- Codex config.toml 代码编辑器
- 环境变量冲突检测
- 本地 Session 历史浏览（项目 → 会话 → 消息）

### 模型验证

- 多维度验证模板（Token 截断、Extended Thinking 等）
- 跨供应商签名验证
- 批量验证 + 历史记录

### 其他

- 自动更新、开机自启、单实例
- 数据导入 / 导出 / 清空
- WSL 环境支持

---

## 安装

### 从 Release 下载（推荐）

前往 [Releases](https://github.com/KNaiFen/aio-coding-hub/releases) 下载对应平台安装包。
本 Fork 的标签发布当前只提供 Windows x64 与 macOS Apple Silicon；其他目标通过手动云端开发构建提供，或使用 upstream 发布：

<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->
| 平台 | 官方发布安装包 |
| --- | --- |
| Windows x64 | `.msi` / `-portable.zip` |
| macOS Apple Silicon | `.zip` |
<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->

桌面安装器的自动更新矩阵仍只覆盖上表 2 个目标；每个正式 Release 还会附带四个平台的独立 `aio-tui` 终端信息面板压缩包。

### SSH / Codex CLI 终端信息面板

桌面 AIO 运行后会在本机回环地址提供只读观测服务。它不会改变网关转发，也不会让 TUI 启动桌面应用。根据 Release 中的 `aio-tui-*` 制品选择平台并加入 `PATH`：

| 平台 | TUI 制品 |
| --- | --- |
| Windows x64 | `aio-tui-win64.zip` |
| macOS Intel | `aio-tui-macos-intel.tar.gz` |
| macOS Apple Silicon | `aio-tui-macos-arm.tar.gz` |
| Linux x64 | `aio-tui-linux-x64.tar.gz` |

```bash
# macOS / Linux
tar -xzf aio-tui-macos-arm.tar.gz   # Intel 使用 aio-tui-macos-intel.tar.gz
chmod +x aio-tui
sudo install -m 0755 aio-tui /usr/local/bin/aio-tui

# 默认进入请求/供应商面板；status 显示状态栏
aio-tui
aio-tui status
aio-tui status --once --cli codex
aio-tui status --items preferred-provider,last-request,concurrency,today-cost
aio-tui statusline
```

Windows 解压 `aio-tui-win64.zip` 后，把目录加入 `PATH`。默认面板用 `←/→` 在最近 50 条请求和供应商状态之间切换；供应商按当前路由顺序显示，每张卡固定五行，`Enter` 查看只读详情。支持 `--cli claude|codex|grok|gemini|all`；`all` 的状态栏会按最近一条终态模型推理请求选择 CLI。`aio-tui statusline` 中使用空格启用项目、`←/→` 调整顺序、`c` 切换颜色并按 Enter 保存；`--items` 只覆盖当前运行。默认显示首选供应商、上次请求、近 10 次主供应商、并发、今日费用和今日 Token，设置 `NO_COLOR` 可强制禁用颜色。状态栏中的并发是全局活跃模型推理请求数，同一会话和子代理的每个请求都计 1。观测服务离线或短暂繁忙时 TUI 保留最后快照并显示陈旧标记，不会崩溃或自动启动 AIO。

独立 TUI 资产不会进入桌面 updater 的 `latest.json`，并由发布中的 `SHA256SUMS.txt` 校验。

<details>
<summary>Linux Arch / Wayland 用户</summary>

**推荐：AUR 软件包**（使用系统库，兼容性最好）

```bash
paru -S aio-coding-hub-bin
# 或
yay -S aio-coding-hub-bin
```

**AppImage 用户**

应用在 Wayland 下启动时会自动检测并注入 `WEBKIT_DISABLE_COMPOSITING_MODE=1` 以避免 EGL 冲突崩溃（见 [issue #93](https://github.com/FingerCaster/aio-coding-hub/issues/93)）。仓库不提供本地原生重打包入口；需要桌面制品时请使用 `dev-build` 云端工作流。

</details>

<details>
<summary>macOS 安全提示</summary>

若遇到"无法打开 / 来源未验证"提示：

```bash
sudo xattr -cr /Applications/"AIO Coding Hub.app"
```

</details>

### 本地零产物与云端验证

仓库不在本地安装依赖、启动开发服务，也不在本地运行格式化、类型检查、Lint、测试或构建。任务流程以 `$gkd-main` skill 为准，[AGENTS.md](AGENTS.md) 补充 AIO 的环境与 Git 约束。main 在 `.gkd/plan.md` 记录获批方案，并按 GKD 选择 direct-main 或 delegated；仅 delegated 生成执行 worktree 的 `.gkd/execution.md` 和 `.gkd/progress.md`。资料按受影响行为读取，历史记录不指导新任务。

提交前只执行计划批准的零依赖、无产物检查。合并前等待自动 CI 按改动分类选中的质量门：普通 PR 与受保护分支推送自动触发 `ci`，PR 需要对应的 `ci-gate` 和独立 `pr-title`；常规 PR 验证不重复触发手动 `ci`。`workflow_dispatch` 仅用于 `main` 的恢复或候选构建，Provider trend release benchmark 由相关自动 CI 路径或独立 `performance` 工作流执行；需要桌面集成制品时，在 Actions 页面按需运行 `dev-build` 并选择目标。

纯文档 PR 与纯文档主干推送保留轻量分类；`.gkd/` Markdown 不触发前端或 Rust 构建，README、AGENTS 和规范变更仍执行文档合同。纯前端或纯 Rust PR 只运行相应域，shared、混合或未知路径运行两端；含代码或未知文件的主干推送仍跑完整 CI，CodeQL 保持独立运行。任务从 `origin/main` 建分支，包括 direct-main；squash 合并后同步远端实际结果。版本标签只指向已生成成功签名候选的实际 main 合并提交。详见[提交与发版流程](docs/operations/github-actions-governance.md#提交与发版)。

<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->
| 分类 | 云端工作流目标 | 说明 |
| --- | --- | --- |
| 正式发布 / 开发制品 | Actions `dev-build`: `windows-x64` | Windows x64；`main` CI 生成签名候选；手动工作流生成无签名开发制品 |
| 开发制品 | Actions `dev-build`: `macos-x64` | macOS Intel；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
| 正式发布 / 开发制品 | Actions `dev-build`: `macos-arm64` | macOS Apple Silicon；`main` CI 生成签名候选；手动工作流生成无签名开发制品 |
| 开发制品 | Actions `dev-build`: `linux-x64` | Linux x64；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->

手动云端制品均不签名且不会被 Release 晋升。正式 Release 只晋升成功 `main` CI 为 Windows x64 与 macOS Apple Silicon 生成的签名候选。

---

## 快速开始

```
1. 供应商页 → 添加上游（官方 API / 自建代理 / 公司网关）
2. 首页 → 打开目标 CLI 的"代理"开关
3. 终端发起请求 → 在控制台 / 用量页查看 Trace 与统计
```

验证网关运行：

```bash
curl http://127.0.0.1:37123/health
# {"status":"ok"}
```

### 插件开发文档

插件系统面向社区扩展，社区插件统一使用 Extension Host。开发入口：

- [插件开发总览](docs/plugins/README.md)
- [插件开发总指南](docs/plugins/developer-guide.md)
- [Plugin SDK](docs/plugins/reference/sdk.md)
- [官方示例插件](docs/plugins/examples/privacy-filter.md)
- [插件 API 参考](docs/plugins/reference/README.md)
- [Manifest v1 规范](docs/plugin-manifest-v1.md)

---

## 项目文档与维护

- [项目知识库入口](docs/README.md)：产品、架构、插件、运维、任务和历史资料的权威导航。
- [待处理事项](PENDING.md) 与 [已完成事项](PENDING_COMPLETED.md)：延后工作和交付记录。
- 任务计划、交接、进度、审查与归档按 GKD 路线维护，不在仓库维护第二套生命周期事实。

现行实现优先于历史审计、旧计划和会话日志；完整文档维护规则见 [知识库维护规则](docs/README.md#维护规则)。

## 技术栈

| 层级 | 技术 |
|------|------|
| **前端** | React 19 · TypeScript · Tailwind CSS · Vite |
| **状态管理** | TanStack Query · React Hooks |
| **桌面框架** | Tauri 2 |
| **后端** | Rust · Axum (HTTP Gateway) |
| **数据库** | SQLite (rusqlite) |
| **测试** | Vitest · Testing Library · MSW · Cargo Test |

---

## 质量保证

GitHub Actions 按改动分类运行相应质量门：前端包括依赖审计、Lint、TypeScript、插件 SDK/脚手架测试、E2E、覆盖率和 Vite build；Rust 包括格式、`Cargo.lock`、生成绑定、Clippy、测试与 audit。`ci-gate` 要求所选 job 成功、未选 job 为 `skipped`；跨平台桌面打包仍是 main 候选或按需 `dev-build`，不是每个 PR 的必需任务。

CI 检测到格式、锁文件或生成绑定漂移时，下载并审查它提供的有界补丁，不要在本地重新生成。

---

## 不适用场景

- 公网部署 / 远程访问 / 多租户
- 企业级 RBAC 权限管理

> 本项目定位为 **单机桌面工具 + 本地网关**，所有数据保存在本机。

---

## 参与贡献

欢迎提交 Issue 和 PR！开发流程与授权遵循 `$gkd-main`，实施约束见 [AGENTS.md](AGENTS.md)。每个完成的任务通过任务分支 PR 集成，使用简短中文 [Conventional Commit](https://www.conventionalcommits.org/)。

```bash
feat(ui): 添加用量热力图
fix(gateway): 修正超时处理
docs: 更新安装指南
```

---

## 许可证

[MIT License](LICENSE)

## Star History

[![Stargazers over time](https://starchart.cc/KNaiFen/aio-coding-hub.svg?variant=adaptive)](https://starchart.cc/KNaiFen/aio-coding-hub)
