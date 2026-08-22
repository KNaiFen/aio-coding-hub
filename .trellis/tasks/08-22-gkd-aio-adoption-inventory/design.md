# 设计：AIO GKD 接入边界

## 目标结构

后续接入以“固定 GKD bundle + 薄 AIO adapter/policy”为唯一方向。bundle 负责可跨仓库复用的任务机状态、角色路由、fixed-head monitor 与验收机制；adapter 只声明 AIO 的 origin、仓库身份、required checks、本地可运行的微型验证、CI 资源规则和 release 合同。

这使 AIO 不再复制通用逻辑，同时保证 `ci-gate`、`pr-title`、Rust/Tauri 及发布策略仍由唯一的项目所有者维护。

## 迁移安全门

1. bundle 版本、发行资产 SHA-256 和已验证 output digest 必须同时固定；canonical source 不是安装或验收证据。
2. 每个 adapter 都必须核验 repo/policy/origin 三方一致，任一不符即停止。
3. 状态迁移只可调用支持的 bundle/adapter 命令。旧任务 JSON、offer、claim、receipt 或历史交接不得手改或补造。
4. CI 与 release 改动必须固定 PR head，AIO 的 required checks 以实时项目策略为准。
5. 旧实现删除依赖于 fixture、adapter smoke、真实 canary 和独立验收；没有证据时保留旧路径并报告 GKD 核心缺陷。

## 本任务的变更面

仅新增本任务目录中的 `prd.md`、`inventory.md`、`design.md` 与 `implement.md`。这些文档不声明任何已完成的 runtime migration，也不改变 AIO 的现行执行合同。
