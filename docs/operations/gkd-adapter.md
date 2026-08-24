# GKD 项目 Adapter

`.gkd/policy.json` 是 AIO 的 GitHub identity、默认分支和必需检查的唯一通用项目 policy。`.gkd/bundle-pin.json` 固定已发布 GKD `v0.1.5` 的 source、asset 与 execution bundle digest；`.gkd/review-adapter.json` 是对应的 review adapter v1 事实，并绑定该 policy。`.gkd/resource-facts.json` 是 AIO 专有的 schema v1 事实，绑定 policy digest、默认分支和 required checks；它只确认公开 workflow 可证实的 GitHub-hosted Linux runner 来源，并将容量与账单保持为未验证的 `unknown`。

`.gkd/adapter-policy.json` 是另一个 AIO 专有的 schema v1 事实，只声明仓库已经执行的 adapter 边界。它绑定零产物本地验证入口、完整小写 base SHA 与 cloud-owned 类别；GitHub-hosted runner、pnpm/Rust cache 和有界 artifact 名称/保留期；以及 tag、main 祖先、成功同 SHA main CI candidate、`SHA256SUMS.txt` 和既有 Release 等价资产不可覆盖的晋升合同。它不扩展 GKD 通用 policy schema，也不解析 workflow、查询 GitHub 或执行发布。

`.gkd/history-adapter.json` 是 AIO 专有的 tracked Trellis history 边界。`scripts/check-gkd-history.mjs` 只从 `git ls-files` 枚举 `.trellis/tasks` 下的 manifest：要求恰好一个 immediate-child active manifest，其 `worktree_path` 必须为 `null` 且不得使用 `coordination.version=1`；所有 archive descendant manifest 必须为 `completed`。archive 中历史 `worktree_path` 的值和存在性均被忽略，不作路径解析、不访问对应 filesystem 也不输出；未跟踪的 task 目录不是项目状态。当前 tracked inventory 为 1 个 active 和 107 个 archive manifest。

.gkd/ci-release-adapter.json binds the verified speed-first recommendation, Air-safe Node-only micro boundary, independent cloud job groups, ci-gate/pr-title required checks, redacted leak scanning, bounded artifact/cache retention, and same-source-SHA candidate/finalization rules. scripts/check-gkd-ci-release.mjs reads repository-relative paths and emits stable codes/digests; it does not dispatch workflows, query GitHub, write settings or Secrets, create tags, or publish Releases.

更新这些 adapter 文件时，必须保持 canonical JSON，并使用 `node scripts/check-gkd-adapter.mjs` 验证 pin、adapter digest、adapter policy、resource facts、history adapter 与通用 policy binding。仓库的版本化 GKD 本地验证入口是 `scripts/gkd-verify --base-sha <full-lowercase-sha>`，它只委托既有的零依赖 local runner，并在 history 表面变化时运行 read-only checker 及其 selftest。

角色技能、claim receipt 和 runtime inventory 属于 project-local staging，不进入 Git；本仓库只保存可审查的通用 policy、pin、review adapter、project-only adapter policy、resource facts 和 history adapter，不复制 GKD 通用生命周期或运行时状态。adapter policy 不是实时 workflow/API discovery，resource facts 不能充当实时资源扫描、CPU/内存/磁盘容量或价格/账单事实，history adapter 也不恢复旧 Trellis 协调或接受流程。
