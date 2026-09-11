# CI 等待与测试初始化优化执行交接

- PLAN revision: r1，已由用户“开始执行此PLAN”批准。
- Execution revision: e1，2026-09-12。
- 执行 worktree: `/Users/knaifen/Documents/Codex/aio-coding-hub/ci-test-initialization`。
- 分支: `ci/test-initialization`；baseline: `b03b5ab4d57df5b39348744d5e65d6cc835fbe1b`。
- 角色: `gkd_execute`，唯一 writer。主工作树和其他任务有用户改动，不还原、不提交、不清理他人内容。禁止派生子代理。
- 只读执行交接和适用 AGENTS 后读取必要源码；无需读取其他 PLAN/历史 session。若有 `.codegraph` 先用其定位；本工作树未索引则不新建索引。

## 1. 目标与允许修改

优化测试固定规则重复初始化、增强既有偶发失败诊断、补充项目内监控交接说明。只允许以下实现文件及本任务 progress；本 execution 为 main 所有，writer 不修改：

1. `src-tauri/src/app/plugins/privacy_filter.rs`：仅 `#[cfg(test)] mod tests`。
2. `src-tauri/src/app/plugins/privacy_redaction_service.rs`：仅 `#[cfg(test)] mod tests`。
3. `src-tauri/src/gateway/routes.rs`：仅 `run_codex_error_response_rule_route` 测试 helper 的现有计数断言失败消息。
4. `docs/operations/github-actions-governance.md`：普通 PR 监控交接说明。
5. `.gkd/progress.md`：实施事实、检查、消融审查、剩余风险和提交信息。

产品实现、工作流、分类器、测试线程、构建并发、benchmark、依赖/版本/缓存键、用户级 skills 均不修改。现行官方/运行时集成装配不变。不开展规则算法或安全专项调查。

## 2. 实现要求

### 两个模块内复用固定编译结果

- `privacy_filter.rs::tests::filter`（约702行）当前7个测试每次解析同一 include_str fixture。增加模块私有 `LazyLock<PrivacyFilter>`，保留原解析及 expect，filter 返回静态只读引用。相应调整多余借用，所有测试输入、断言、名称、数量/ignore 状态保持。真实加载统计仍验证共享实例由原加载入口构建。
- `privacy_redaction_service.rs::tests` 增加模块私有 `LazyLock<Arc<PrivacyFilter>>`，使用现有 `privacy_filter_detail(json!({}))` 和真实 `load_privacy_filter` 加载固定 packaged 规则；初始化失败直接报错。与低层 include_str fixture 保留独立静态来源。
- 同一 tests 模块增加重复使用的 service 装配 helper，接受 `&PluginDetail`，每次创建独立 `PrivacyRedactionService`、Mutex/HashMap，通过既有 `privacy_filter_cache_key(plugin)` 预填共享规则的 Arc clone。tests 子模块可访问父模块私有字段，无需任何生产构造器/跨模块注入入口。
- 13个仅检查输出/配置的 packaged 规则测试改用该 helper，包括 `execute_privacy_filter_request` 及其他直接 Default 装配。
- 明确保留 `privacy_redaction_service_redacts_before_send_request_bodies` 的 Default/真实加载；临时规则文件限额、`retain_prewarms_and_prunes` 原样；官方插件/runtime/plugin_service 集成测试原样。每测例的服务缓存、plugin配置、请求结果仍独立。
- 预期低层7次变1次，服务14次变共享1次+真实加载1次，结构减少约18次初始化。只共享 `&self` 使用的不可变规则，不能全局共享服务或缓存任意输入。不新增测试框架或重复测试。

### 原断言补上下文

`routes.rs` 约1777行 `assert_eq!(call_count.load(...), 1)` 增加失败格式消息，使用已存在的 status、response、provider_id、终态请求 log（Debug，含 error_code/attempts_json/provider_chain_json）及 `upstream_task.is_finished()`。

保持所有行为断言，不加 sleep/retry/超时放宽/新请求/mock 状态机。历史同SHA首次上游计数0、预期1，rerun通过，但根因未知；交付只称诊断增强。

### 治理文档

现行文档已含归档合批顺序，保留不复制。补充紧凑段落：

- 普通 PR 使用现有 watcher PR 模式，required `ci-gate`/`pr-title`；任务必要 job 可加入 expected 以提前观察失败。名称从目标 workflow 和 `gh pr checks --json name,bucket,state,workflow` 的实际 name 获取，不能拼 UI 展示名。
- expected 随真实选择域：本任务正常 PR contracts/rust/observer-macos；frontend skipped不加入；若合法生成绑定漂移令其被选中再加入。main 含代码时两端执行，commit 检查包含 ci-gate/contracts/frontend/rust/observer-macos，不添加 main 不存在的 pr-title。
- 所需失败在下一查询被观察后返回，缺失/pending继续待定；分别报告 GitHub终态、脚本timeout、调用错误、代理异常。默认240秒/6小时不变，run模式等待run终态，不等同PR逐job失败返回。
- 不修改 watcher/用户skills，不因为缺失名称删期望、不因代理异常重触发CI。

## 3. 验证与权限

本机M4 MacBook Air/16GB、磁盘约228GiB，项目禁止本地重检查。只运行 `git diff --check`、Git/gh只读及必要低负载结构比较；不得安装依赖、运行项目包脚本、rustfmt、生成绑定、clippy/test/build/打包或新增环境绕过。不要启动服务器。

完整验证由审查后closeout归档合批推送最终head所自然触发的GitHub Actions承担。此阶段无需触发任何云端运行，progress明确“最终CI待定”。云端保留 `cargo clippy --workspace --all-targets --locked -- -D warnings`、`CARGO_BUILD_JOBS=1 cargo test --workspace --locked -- --test-threads=1`，以及macOS既有observer测试、contracts和生成检查。

交付给main前做消融审查：代码不越出tests、共享规则只读/缓存独立、所有原测试/加载路径/断言保留、无不必要抽象、文档不制造新门槛。允许中文本地实现提交，只暂存自己范围内文件和progress；main创建的execution可随本任务记录提交但不能改正文。不推送、不创建/编辑PR、不合并、不发布、不归档或清理。提交后停止。

## 4. 返回

返回完整实现commit、修改文件、检查与结论、测试结构计数、待云端事项及真实风险。progress保持事实：新增诊断不等于根因修复；结构减少18次初始化不等于承诺提速。若发现超范围/方案不能成立，保留现场返回main，不私自放宽或改路线。
