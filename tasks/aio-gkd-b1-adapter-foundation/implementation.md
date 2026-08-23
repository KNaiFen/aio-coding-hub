# AIO GKD Bundle And Review Adapter Foundation Implementation

## Internal Design

- `.gkd/policy.json` 继续作为 GKD CI monitor、bootstrap、route 与 project staging 的唯一 repository policy。
- `.gkd/bundle-pin.json` 是 AIO 对已发布 bundle 的只读 pin，字段只表达 version、release source SHA、execution bundle digest 与 asset SHA-256。
- `.gkd/review-adapter.json` 采用已发布 review adapter v1：单个 AIO repository record 指向 `.gkd/policy.json`，并列出 diff/check/pull request/artifact capability。
- `scripts/check-gkd-adapter.mjs` 仅验证 AIO adapter 文件的字节 canonicality、结构、digest 与跨文件 identity binding；不实现 task、route、claim 或 GitHub API 行为。

## Execution Details

1. 添加 pin、review adapter 与 adapter 文档，写入已验收 `v0.1.3` 的固定发行事实。
2. 添加 smoke 与 selftest，并在 local runner 中以相关 changed paths 为条件调用；补充 runner selftest 覆盖触发和非触发路径。
3. 在 `AGENTS.md` 以最小规则说明：版本化 policy/adapter 在 Git 内，角色与 runtime inventory 是 project-local staging，且 runtime inventory 不能提交。
4. 执行现有允许的 local verification；由 PR 的标准 GitHub Actions 验证剩余 cloud-owned 检查。
