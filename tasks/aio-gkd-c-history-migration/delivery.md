# AIO GKD Historical Task Adapter Delivery

## 结果

- 结果：等待验收。
- PR：#179。
- AIO 现在以 read-only adapter 读取 tracked Trellis history；新的执行仍只使用 canonical GKD lifecycle。

## 实际实现

- 新增 canonical `.gkd/history-adapter.json`，固定 tracked active/archive roots、single-active 规则、active `worktree_path`/legacy coordination 禁止规则，以及 completed archive 与 archived-worktree ignore 规则。
- 新增 `scripts/check-gkd-history.mjs`，只从 `git ls-files -z` 枚举 manifest，拒绝非规范位置、symlink、路径逃逸、active 歧义、active legacy state 和非 completed archive，并只返回确定性的脱敏计数。
- 新增 isolated Git fixture selftest，覆盖当前 1/107 inventory、重复运行无写入、零/多 active、active Unix/Windows/relative path、legacy/malformed coordination、archive stale/missing path、未跟踪替代、malformed/non-completed archive、非规范位置和 symlink。
- 现有 adapter validator 严格校验 history declaration 的 canonical JSON、exact keys 和 exact values；版本化 local verifier 在相关表面或 `.trellis/tasks/**/task.json` 变更时运行 history selftest/checker，其他 Trellis 文档不触发。
- `AGENTS.md` 与 `docs/operations/gkd-adapter.md` 记录 tracked-only history 边界和当前 inventory。

## AC 证据

| AC | 结果 | 证据 |
|---|---|---|
| Canonical history declaration | 通过 | Adapter selftest 覆盖 non-canonical JSON、unknown fields 与 active/archive drift；adapter smoke 通过。 |
| Tracked-only 1/107 inventory | 通过 | History smoke 返回 `active_count=1`、`archived_count=107`；isolated fixture 证明未跟踪 manifest 被忽略。 |
| Active fail-closed rules | 通过 | Selftest 覆盖零/多 active、缺失或非 null `worktree_path`、`coordination.version=1` 和 malformed coordination。 |
| Archive compatibility | 通过 | Selftest 接受 stale Unix、Windows、relative、null 或 absent worktree facts，拒绝 malformed 或非 completed archive。 |
| Determinism and no writes | 通过 | Selftest 比较重复结果与前后 Git status；固定 base 到 implementation head 的 `.trellis/tasks/**` 路径集和内容无差异；local-verifier selftest 通过真实 Git 删除场景证明 active/archive manifest 触发、非 manifest 文档不触发。 |
| Canonical route and closeout | 部分完成 | clean-main bootstrap、portable locator、epoch 1 automatic offer/claim 与 fresh exact executor repair 已完成；本文件提交后的 canonical delivery、独立 acceptance 与 records-only closeout 仍由各自角色完成。 |
| Required verification | 部分完成 | 本地合同与 `git diff --check` 已通过；最终 fixed delivery head 的 `ci-gate`/`pr-title` 尚待 canonical monitor。 |

## 关键位置

| 文件或符号 | 实际变化 | 设计原因 |
|---|---|---|
| `.gkd/history-adapter.json` | 声明 AIO history policy | 让 project facts 可审查且与 checker 行为严格绑定。 |
| `scripts/check-gkd-history.mjs:verifyHistory` | tracked-only 分类、验证和计数 | 隔离 legacy history read 与 canonical GKD lifecycle。 |
| `scripts/check-gkd-history.selftest.mjs` | isolated Git 正反 fixture | 证明 tracked boundary、archive compatibility、determinism 与 fail-closed 行为。 |
| `scripts/check-gkd-adapter.mjs:validateHistoryAdapter` | canonical exact-field validation | 防止声明与实现静默漂移。 |
| `scripts/check-local-verification.mjs:shouldRunHistorySmoke` | 版本化触发与结果绑定，包括 active/archive manifest path | 保持单一批准的本地验证入口并关闭 manifest-only 漏检。 |

## 返工记录

- Epoch 0 fixed head `8be7e9cd676e0115acb05f3508cbdf095ee60c6d` 经独立审查被拒绝；HIGH finding 指出 manifest-only 变更不会触发 history smoke。
- Epoch 1 仅修复该 finding：`.trellis/tasks/**/task.json` 现在触发 history smoke，active/archive manifest 与非 manifest Trellis 文档的触发行为均由 selftest 固定。
- Epoch 2 仅修复同一 HIGH 的删除路径漏检：提交、暂存和 worktree 三类 diff filter 均从 `ACMR` 改为 `ACMRD`；selftest 以真实删除分别证明 active/archive manifest 触发 history smoke，并保持非 manifest Trellis 文档为 false。

## 验证

| 类型 | 命令或检查 | 结果 | 说明 |
|---|---|---|---|
| 本地 | `scripts/gkd-verify --base-sha 3f856c88749f4875889164fa72caeebc22143d98` | 通过 | 在 implementation head `b913fccbc877e5ac482869c745c98959f481de83` 上返回 `local_ready`；history smoke 返回 `active_count=1`、`archived_count=107`，并覆盖删除触发 selftest、adapter/history selftests 与 smoke、三层 diff、untracked whitespace 和六个 Node 文件语法。 |
| 本地 | `git diff --check` | 通过 | 无 whitespace error。 |
| 本地 | fixed base 对 `.trellis/tasks/**` 的 path/content diff | 通过 | 无 tracked 路径或内容差异。 |
| GitHub | `ci-gate` / `pr-title` | 等待 | 仅在 final fixed delivery head 上由 canonical monitor 观察一次。 |
| 云端所有 | dependencies、format、lint、typecheck、tests、coverage、build、generators、Rust/Tauri、signing/packaging | 未在本地运行 | 保持现有 cloud-owned 边界。 |

## 合同与影响

- 测试：新增 history checker selftest，并扩展 adapter/local-verifier selftest。
- 现行文档与机器合同：新增 history adapter；更新 AIO adapter operations 与根 facts inventory。
- API、兼容性与迁移：新增 AIO-only checker machine result；不修改 Trellis manifests、不恢复旧 coordination/acceptance，也不改变 canonical GKD API。
- 数据、配置、安全与隐私：只读取 Git tracked manifest；archive `worktree_path` 不作路径解析、filesystem 访问或输出。
- 发布与回滚：无发布或部署；回退 implementation commit 即可。

## 风险与审查重点

- 剩余风险：未来 tracked archive inventory 变化时，需要同步更新当前计数事实与 current-shape selftest；checker 的规则本身不固定 archive 数量。
- main 重点审查：`scripts/check-gkd-history.mjs:classifyManifestPaths` 的 tracked path 分类，以及 archive 验证不访问 `worktree_path` 的边界。
- 未完成项：final fixed-head CI、独立 acceptance、merge、records-only closeout 和 cleanup 均在 executor 边界之外。

## Candidate Output Bundle

- Implementation head：`b913fccbc877e5ac482869c745c98959f481de83`。
- Deterministic Git source archive SHA-256：`7121abfa2d7bb8eacd29c825a20409990f74eb9a4fe15c64109f22d0f71c90b0`。

## 阻塞快照

- 无。
