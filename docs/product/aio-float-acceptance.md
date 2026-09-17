# AIO Float 验收记录

## 自动化覆盖

`float-build.yml` 验证 Windows AMD64 和 macOS ARM64 的共享 TUI 与悬浮窗，
`ci.yml` 验证配套 AIO、观察服务及生成绑定。以交付提交对应的 Actions 结果为准。

| 范围 | 覆盖 |
| --- | --- |
| 局域网生命周期 | 默认关闭，令牌隔离，未授权，端口冲突，重置，关闭，重启恢复，启动绑定失败 |
| 本机兼容 | 局域网变更期间本机健康接口、描述文件字节保持有效；多个端共享缓存读取 |
| 字符网格 | TUI/Float 相同状态的字符及宽字符坐标，窄窗口边界，中文及组合字符 |
| 客户端状态 | 断线保留快照，恢复清除错误，换连接清空数据，旧响应隔离，隐藏/恢复 |
| 共享导航 | 复用现有 TUI 详情、CLI 切换、选择超时和供应商测试回归用例 |
| WebView 视觉 | 280×640、160×300、640×480，1x/2x DPI，透明背景/不透明字形像素，缩放及设置 |

Playwright 使用合成快照与 IPC 桩，验证前端渲染和交互；它不代表原生窗口系统验收。
截图作为 `float-visual-*` 工件保留。原生可执行文件及校验和作为 `aio-float-*` 工件保留。

## 2026-09-17 云端验收

代码提交：`dfb3c902ab573047d6f290c37bc2301b99c00522`，版本 `0.60.61`。
PR 工作流检出的合并提交为 `e1c1e0168334ea7c65fe92963ae6678a6b642d75`；
配套 AIO 的手动构建直接检出上述代码提交，两者代码树一致。

| 检查 | 结果 |
| --- | --- |
| [主 CI](https://github.com/KNaiFen/aio-coding-hub/actions/runs/35205501973) | 通过：前端、Rust 测试、Clippy、依赖审计、格式和绑定无漂移、macOS 观察服务测试 |
| [Float 构建与视觉](https://github.com/KNaiFen/aio-coding-hub/actions/runs/35205501933) | 通过：Windows x64 EXE、macOS ARM64 app ZIP、共享 TUI 测试、Playwright |
| [CodeQL](https://github.com/KNaiFen/aio-coding-hub/actions/runs/35205501916) | 通过 |
| [配套 Windows AIO](https://github.com/KNaiFen/aio-coding-hub/actions/runs/35205537995) | EXE 与 MSI 构建成功 |
| [配套 macOS AIO](https://github.com/KNaiFen/aio-coding-hub/actions/runs/35205558312) | ARM64 app ZIP 构建成功 |

视觉回归另覆盖窄窗口的长离线错误提示，错误区域与 TUI 底部帮助行分开排版。
修复了审计报告的 `rustls` 问题；`Cargo.lock` 使用该提交前云端生成的
`rustls 0.23.45` / `rustls-webpki 0.103.15` 解析结果。

以上记录仅表示云端结果。Windows 10 的原生截图工具兼容性限制导致交互验收
未完成；已有窗口的只读截图和可访问性检查不作为本次提交的完整原生验收。

## 原生窗口验收

| 项目 | Windows 10/11 AMD64 | macOS ARM64 |
| --- | --- | --- |
| 跨电脑真实连接与供应商手动测试 | 待实机验收 | 待实机验收 |
| 无边框拖动及四边/四角缩放 | 待实机验收 | 待实机验收 |
| 背景透明、文字清晰及系统字体回退 | 待实机验收 | 待实机验收 |
| 置顶、穿透后托盘/菜单栏恢复 | 待实机验收 | 待实机验收 |
| 启动/首次设置/托盘恢复后均不占用任务栏 | 待实机验收 | 不适用 |
| 凭据保存、退出重启、第二次启动 | 待实机验收 | 待实机验收 |
| 多屏拔插、跨屏 DPI 和窗口恢复 | 待实机验收 | 待实机验收 |

当前开发环境没有 macOS ARM64 实机；云端构建成功不能替代上述验收。
Windows 构建机同样不能替代 Windows 10 实机以及多显示器验证。
