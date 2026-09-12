# CI 等待与测试初始化优化执行记录

- 日期：2026-09-12；依据：PLAN r1 已获批，execution e1。
- 工作树：`<任务工作树>`。
- 分支：`ci/test-initialization`；baseline：`b03b5ab4d57df5b39348744d5e65d6cc835fbe1b`。
- 环境：交接记录为 M4 MacBook Air / 16GB、约 228 GiB 磁盘；本次轻量结构比较实际使用 Node.js v26.5.1 和 Git，通过 FastCtx 非交互 Bash 执行，所有命令 cwd 显式指定上述工作树。
- 初始 `git status --short --branch`：目标分支正确，仅 main 创建的 `.gkd/execution.md` 未跟踪；未发现嵌套 AGENTS 或 `.codegraph` 文件，未创建索引。

## 实现事实

1. `privacy_filter.rs::tests` 添加私有 `LazyLock<PrivacyFilter>`；原 `include_str!` 路径、真实解析及 `expect` 保留，`filter()` 返回静态只读引用，移除相应多余借用。
2. `privacy_redaction_service.rs::tests` 添加独立的私有 `LazyLock<Arc<PrivacyFilter>>`，由 `privacy_filter_detail(json!({}))` 与真实 `load_privacy_filter` 构建 packaged 规则。重复装配 helper 每次创建独立 service / Mutex / HashMap，以实际 plugin 的既有 cache key 预填 Arc clone。
3. 5 个 `execute_privacy_filter_request` 调用测试及 7 个直接装配测试使用共享规则。`redacts_before_send_request_bodies` 的 Default/真实加载、临时规则文件限额测试、`retain_prewarms_and_prunes` 逐字保留。
4. `routes.rs::tests::run_codex_error_response_rule_route` 仅给原上游计数断言增加 status、response、provider_id、终态 log Debug 及上游任务结束状态；期望计数仍为 1。
5. 治理文档补充 PR watcher 的 required/expected、真实 job 名称、按选择域调整、失败和待定语义、四类退出结果、240 秒/6 小时及 run 模式差异。既有归档合批顺序保留。

## 交接计数纠正

e1 的服务层计数多算 1 个：实际服务模块共 15 个测试，其中 2 个为临时规则/缓存生命周期测试，13 个使用 packaged 规则。后者由 5 个请求 helper 调用测试和 8 个直接 Default 测试组成，保留真实加载的 1 个后，共 12 个复用共享规则。

已向 main 返回源文件计数事实；main 明确回复按实际 12 个复用测试继续，数字纠正不改变文件、行为、验证或授权范围，无需修改 execution。低层初始化 7 次变 1 次，服务 packaged 初始化 13 次变共享 1 次 + 真实加载 1 次，合计结构减少 17 次。该计数针对完整执行这些测试的一次测试进程，不是耗时测量或提速承诺。

## 本地检查

- `git diff --check`：退出 0，无输出。
- `git diff -- <四个实现文件>`：已逐项检查；`git diff --name-only` 恰为交接的四个实现文件。
- 下列无依赖 Node stdin 结构比较：最终退出 0。三个源文件 tests 模块前内容完全一致；低层 7 个、服务层 15 个测试正文在仅归一化获准装配/借用变化后逐字一致，测试属性保持；3 个特殊服务测试逐字一致；routes 全文件仅新增断言失败消息。
- 检查脚本前两次未通过：首次多写右括号导致 SyntaxError；第二次 marker 假定 `#[cfg(test)]` 紧邻 `mod tests`，遇到 routes 已有 Clippy 属性失败。仅修正脚本括号和定位为 `mod tests`，未改变源代码、删除断言或放宽比较标准。最终仍逐字比较 routes 整文件的唯一获准替换。
- 早期尝试通过 `functions.exec` 批量调用 FastCtx 时运行时未暴露相应方法，调用失败且未执行命令或写入；后续使用 FastCtx 直接接口。

可复用的结构比较命令如下，cwd 为本工作树；比较固定 baseline 与当前文件，不运行项目测试、不生成文件：

```bash
node <<'NODE'
const assert = require('node:assert/strict');
const fs = require('node:fs');
const cp = require('node:child_process');
const path = require('node:path');
const root = '<任务工作树>';
const baseline = 'b03b5ab4d57df5b39348744d5e65d6cc835fbe1b';
const git = (...args) => cp.execFileSync('git', args, { cwd: root, encoding: 'utf8' });
const files = ['src-tauri/src/app/plugins/privacy_filter.rs', 'src-tauri/src/app/plugins/privacy_redaction_service.rs', 'src-tauri/src/gateway/routes.rs'];
const sources = files.map(file => ({ file, before: git('show', `${baseline}:${file}`), after: fs.readFileSync(path.join(root, file), 'utf8') }));
for (const { file, before, after } of sources) {
  const marker = '\nmod tests {';
  assert.ok(before.includes(marker), file);
  assert.equal(after.slice(0, after.indexOf(marker)), before.slice(0, before.indexOf(marker)), `${file}: code outside tests unchanged`);
  assert.deepEqual(after.match(/^\s*#\[(?:tokio::)?test[^\n]*\]|^\s*#\[ignore[^\n]*\]/gm), before.match(/^\s*#\[(?:tokio::)?test[^\n]*\]|^\s*#\[ignore[^\n]*\]/gm), `${file}: test attributes unchanged`);
}
const [low, service, routes] = sources;
const tests = source => source.slice(source.indexOf('    #[test]')).split(/(?=    #\[test\]\n)/).filter(Boolean);
const lowBefore = tests(low.before);
const lowAfter = tests(low.after);
assert.equal(lowBefore.length, 7);
const adjustedLow = lowBefore.map(test => test.replaceAll('redact(&filter,', 'redact(filter,').replace('            &filter,', '            filter,'));
assert.deepEqual(lowAfter, adjustedLow);
const serviceBefore = tests(service.before);
const serviceAfter = tests(service.after);
assert.equal(serviceBefore.length, 15);
const oldAssembly = '        let service = PrivacyRedactionService::default();\n        let plugin = privacy_filter_detail(json!({}));';
const newAssembly = '        let plugin = privacy_filter_detail(json!({}));\n        let service = privacy_redaction_service(&plugin);';
assert.deepEqual(serviceAfter.map(test => test.replace(newAssembly, oldAssembly)), serviceBefore);
const viaRequest = serviceBefore.filter(test => test.includes('execute_privacy_filter_request(')).length;
const directBefore = serviceBefore.filter(test => test.includes(oldAssembly)).length;
const directAfter = serviceAfter.filter(test => test.includes(newAssembly)).length;
assert.equal(viaRequest, 5);
assert.equal(directBefore, 8);
assert.equal(directAfter, 7);
for (const name of ['rejects_rule_file_over_byte_limit', 'retain_prewarms_and_prunes_privacy_redact_plugins', 'redacts_before_send_request_bodies']) {
  const original = serviceBefore.find(test => test.includes(`fn privacy_redaction_service_${name}()`));
  assert.ok(original, name);
  assert.equal(serviceAfter.find(test => test.includes(`fn privacy_redaction_service_${name}()`)), original, name);
}
const diagnostic = '        assert_eq!(\n            call_count.load(std::sync::atomic::Ordering::SeqCst),\n            1,\n            "status={status} response={response:?} provider_id={provider_id} log={log:?} upstream_task_finished={}",\n            upstream_task.is_finished()\n        );';
assert.ok(routes.after.includes(diagnostic));
assert.equal(routes.after.replace(diagnostic, '        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);'), routes.before);
console.log(JSON.stringify({ baseline, sourceFiles: 3, productionUnchanged: true, testAttributesUnchanged: true, lowTests: lowAfter.length, serviceTests: serviceAfter.length, serviceSharedTests: viaRequest + directAfter, servicePackagedBefore: viaRequest + directBefore, preservedSpecialTests: 3, routeChange: 'assertion message only', estimatedInitializationReduction: (7 - 1) + ((viaRequest + directBefore) - 2) }, null, 2));
NODE
```

实际执行时还在读取源文件前以 `git rev-parse HEAD` 断言 HEAD 等于 baseline，通过；上述可复用版本省略这一提交后不再适用的前置断言，其余结构比较一致。

## 消融审查与剩余风险

- 代码变化均位于 tests，未增加生产入口。两个静态分别保留 include_str fixture 与 packaged 加载来源；共享对象仅通过 `&self` 使用，结果、plugin 配置和缓存仍分别装配。
- helper 被多处重复使用，没有仅为一次性操作增加抽象，没有全局 service、任意输入缓存、新依赖或新增测试框架。现有测试/断言/ignore 状态、官方及 runtime/plugin_service 集成装配保留。
- 文档只说明现行监控交接，不改变 workflow、分类器、required gate 或 watcher/用户级 skills。
- 本地未执行依赖安装、包脚本、rustfmt、生成绑定、Clippy、Rust 测试、build/打包、benchmark 或服务器。结构比较不证明 Rust 编译、Clippy 或运行时通过。
- **最终 CI 待定**：由审查后 closeout 合批推送最终 head 自然触发 Actions。本阶段未推送或触发云端运行。后续需要既有 contracts、Rust 格式/绑定生成检查、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`CARGO_BUILD_JOBS=1 cargo test --workspace --locked -- --test-threads=1` 和 macOS observer 测试证据。
- 原上游计数偶发 0 的根因仍未知，新增诊断不等于根因修复；17 次是结构初始化差额，未测量实际耗时。

## 提交信息

本地提交已获 e1 明确许可；提交说明为 `test: 复用隐私规则测试初始化并补充CI诊断`。范围为四个实现文件、此 progress 与 main 创建的 execution 原文；完整 SHA 随执行返回交给 main，包含本文件的实现提交可由 `git log -1 --format=%H -- .gkd/progress.md` 定位，避免递归回填本文件所属 SHA。提交后停止，审查、推送、PR、合并、发布、归档和清理由 main 接续。
