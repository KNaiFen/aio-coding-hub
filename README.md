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

本 Fork 的发布矩阵只覆盖上表 2 个目标。其他平台的开发制品由 `dev-build` GitHub Actions 工作流生成，不进入 Release 产物或 `latest.json`。

<details>
<summary>Linux Arch / Wayland 用户</summary>

**推荐：AUR 软件包**（使用系统库，兼容性最好）

```bash
paru -S aio-coding-hub-bin
# 或
yay -S aio-coding-hub-bin
```

**AppImage 用户**

应用在 Wayland 下启动时会自动检测并注入 `WEBKIT_DISABLE_COMPOSITING_MODE=1` 以避免 EGL 冲突崩溃（见 [issue #93](https://github.com/FingerCaster/aio-coding-hub/issues/93)）。
若仍遇到白屏，可改用 Release 中附带的 `*-wayland.AppImage`（已剥离内置 EGL/Mesa 库，使用系统版本）：

```bash
# 或者手动对已有 AppImage 进行重打包
./scripts/repack-linux-appimage-wayland.sh aio-coding-hub-linux-amd64.AppImage
```

</details>

<details>
<summary>macOS 安全提示</summary>

若遇到"无法打开 / 来源未验证"提示：

```bash
sudo xattr -cr /Applications/"AIO Coding Hub.app"
```

</details>

### 本地前端开发与云端桌面构建

本地前端需要 Node.js 22 与 pnpm；无需安装 Rust/Tauri 工具链。

```bash
git clone https://github.com/KNaiFen/aio-coding-hub.git
cd aio-coding-hub
pnpm install
pnpm dev
```

`pnpm dev` 只启动 Vite 前端。原生集成、Rust 校验和桌面打包均在 GitHub Actions 中完成；需要桌面制品时，在仓库 Actions 页面手动运行 `dev-build` 并选择目标。

<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->
| 分类 | 云端工作流目标 | 说明 |
| --- | --- | --- |
| 正式发布 / 开发制品 | Actions `dev-build`: `windows-x64` | Windows x64；`main` CI 生成签名候选；手动工作流生成无签名开发制品 |
| 开发制品 | Actions `dev-build`: `macos-x64` | macOS Intel；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
| 正式发布 / 开发制品 | Actions `dev-build`: `macos-arm64` | macOS Apple Silicon；`main` CI 生成签名候选；手动工作流生成无签名开发制品 |
| 开发制品 | Actions `dev-build`: `linux-x64` | Linux x64；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
| 开发制品 | Actions `dev-build`: `macos-universal` | macOS Universal；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
| 开发制品 | Actions `dev-build`: `windows-arm64` | Windows ARM64；手动工作流生成无签名开发制品；不进入 Release / updater 矩阵 |
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

```bash
pnpm check:precommit       # 快速 Node/TypeScript 检查
pnpm check:precommit:full  # 完整本地静态检查（仍为 Node/前端）
pnpm check:prepush         # 前端覆盖率、插件 SDK 与静态合同
pnpm test:unit             # 前端单元测试
pnpm build                 # TypeScript + Vite 前端构建
```

Rust 格式、`Cargo.lock`、生成绑定、Clippy、Rust 测试、audit 与 Tauri 打包全部由 GitHub Actions 负责。CI 检测到规范化漂移时，下载并应用它提供的补丁，不要在本地重新生成。

---

## 不适用场景

- 公网部署 / 远程访问 / 多租户
- 企业级 RBAC 权限管理

> 本项目定位为 **单机桌面工具 + 本地网关**，所有数据保存在本机。

---

## 参与贡献

欢迎提交 Issue 和 PR！采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

```bash
feat(ui): add usage heatmap
fix(gateway): handle timeout correctly
docs: update installation guide
```

---

## 许可证

[MIT License](LICENSE)

## Star History

[![Stargazers over time](https://starchart.cc/KNaiFen/aio-coding-hub.svg?variant=adaptive)](https://starchart.cc/KNaiFen/aio-coding-hub)
