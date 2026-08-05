# 交付证据

## 代码与 PR

- 功能分支：`codex/fix-observability-refresh-status`
- Ready PR：<https://github.com/KNaiFen/aio-coding-hub/pull/73>
- Squash merge：`2a79978c7170871acbbf5179ce6f35d5ec8128c3`
- PR 最终同步 main 后 CI：<https://github.com/KNaiFen/aio-coding-hub/actions/runs/31032716470>

实现阶段的逻辑提交：

- `fe9b0f24`：最终成功上游尝试吞吐。
- `ebf3486f`：请求日志离顶缓冲。
- `cb3091f4`：供应商状态格自然推进。
- `cf191eed`：版本提升到 0.60.49。
- `a14c1898`：应用云端原生格式、锁文件与绑定漂移补丁。
- `0dc75475`：满足云端 Clippy 计时归一化要求。
- `f8ddfdfa`：日聚合测试改用最终尝试耗时。

## 验证

- 本地：`pnpm lint`、`pnpm typecheck`、Vite build 通过。
- 本地前端测试：312 个文件、2812 项测试通过；排除用户的 ignored `.local/**` 测试目录。
- 未在本地运行 Cargo、rustfmt、Clippy、Rust 测试、Specta 或 Tauri。
- 精确合并 SHA main CI：<https://github.com/KNaiFen/aio-coding-hub/actions/runs/31036095938>
- main CI 的 frontend、Rust、Clippy、全量 Rust 测试、百万行 provider trend release 基准、依赖审计和最终 `ci-gate` 全部成功。

## 候选与发布

- 唯一候选制品 ID：`8944530737`
- 候选名称：`release-candidate-2a79978c7170871acbbf5179ce6f35d5ec8128c3-31036095938-1`
- Annotated tag：`aio-coding-hub-v0.60.49`
- Tag object：`12df7dab5fd125509dc7babcae6284a8b0929900`
- Tag 剥离后的提交：`2a79978c7170871acbbf5179ce6f35d5ec8128c3`
- Release workflow：<https://github.com/KNaiFen/aio-coding-hub/actions/runs/31041339318>
- Release：<https://github.com/KNaiFen/aio-coding-hub/releases/tag/aio-coding-hub-v0.60.49>

Release 是最新非草稿、非预发布版本。12 个资产全部上传成功，包括 Windows MSI、Windows portable、macOS ARM updater/portable、四个平台 TUI、两份 updater 签名、`SHA256SUMS.txt` 和 `latest.json`。`SHA256SUMS.txt` 覆盖其余 11 个资产；`latest.json` 的版本、Windows/macOS URL 和签名均已验证。
