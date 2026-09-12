# AIO Coding Hub Agent Rules

工作流统一以用户级 `$gkd-main` skill 为准。

## 项目信息

- AIO Coding Hub 是 Tauri 桌面应用与本地 AI 编码网关，前端使用 React/TypeScript，后端使用 Rust。
- 默认远端为 `origin`，GitHub 仓库为 `KNaiFen/aio-coding-hub`，`gh` 命令使用 `-R KNaiFen/aio-coding-hub`。
- [项目知识库](docs/README.md) 提供产品、架构、插件与运维资料；[模块合同](.trellis/spec/aio-coding-hub/) 记录项目行为和不变量。

## 工具与资源约束

- 禁止本地执行会生成大量文件产物或持续高 CPU 占用的检查。依赖安装、完整测试/覆盖率、编译、打包和长时性能检查使用 GitHub Actions。
- 必要的轻量、短时检查按 GKD 获批方案决定。
- 根包与 workspace 的 package scripts 仅在 GitHub Actions 中运行，不绕过环境 guard；工具、Tauri hook 与云端验证事实见[云端验证合同](.trellis/spec/aio-coding-hub/cross-layer/cloud-only-verification-contract.md)。
