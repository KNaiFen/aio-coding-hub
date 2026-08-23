# GKD 项目 Adapter

`.gkd/policy.json` 是 AIO 的 GitHub identity、默认分支和必需检查的唯一项目 policy。`.gkd/bundle-pin.json` 固定已发布 GKD `v0.1.3` 的 source、asset 与 execution bundle digest；`.gkd/review-adapter.json` 是对应的 review adapter v1 事实，并绑定该 policy。

更新这两个 adapter 文件时，必须保持 canonical JSON，并使用 `node scripts/check-gkd-adapter.mjs` 验证 pin、adapter digest 与 policy binding。仓库的版本化 GKD 本地验证入口是 `scripts/gkd-verify --base-sha <full-lowercase-sha>`，它只委托既有的零依赖 local runner。

角色技能、claim receipt 和 runtime inventory 属于 project-local staging，不进入 Git；本仓库只保存可审查的 policy、pin 与 adapter 事实，不复制 GKD 通用生命周期或运行时状态。
