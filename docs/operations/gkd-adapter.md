# GKD 项目 Adapter

`.gkd/policy.json` 是 AIO 的 GitHub identity、默认分支和必需检查的唯一项目 policy。`.gkd/bundle-pin.json` 固定已发布 GKD `v0.1.4` 的 source、asset 与 execution bundle digest；`.gkd/review-adapter.json` 是对应的 review adapter v1 事实，并绑定该 policy。`.gkd/resource-facts.json` 是 AIO 专有的 schema v1 事实，绑定 policy digest、默认分支和 required checks；它只确认公开 workflow 可证实的 GitHub-hosted Linux runner 来源，并将容量与账单保持为未验证的 `unknown`。

更新这些 adapter 文件时，必须保持 canonical JSON，并使用 `node scripts/check-gkd-adapter.mjs` 验证 pin、adapter digest、resource facts 与 policy binding。仓库的版本化 GKD 本地验证入口是 `scripts/gkd-verify --base-sha <full-lowercase-sha>`，它只委托既有的零依赖 local runner。

角色技能、claim receipt 和 runtime inventory 属于 project-local staging，不进入 Git；本仓库只保存可审查的 policy、pin、review adapter 和 project-only resource facts，不复制 GKD 通用生命周期或运行时状态。resource facts 不能充当实时资源扫描、CPU/内存/磁盘容量或价格/账单事实。
