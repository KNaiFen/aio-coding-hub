import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);
const legacyCreateAioPluginExecCommand =
  "pnpm --filter create-aio-plugin exec " + "create-aio-plugin";

const requiredDocs = [
  {
    path: "docs/plugin-system-rfc.md",
    phrases: [
      "短期不执行任意 JavaScript/TypeScript",
      "提示词优化只能在网关请求阶段可靠实现",
      "第三方代码不得直接进入主进程或 WebView",
      "Skill 市场",
      "gateway.request.afterBodyRead",
      "Final Gateway Hook Chain",
      "gateway.request.beforeSend (active upstream header/body mutation)",
      "gateway.response.chunk (active SSE chunk inspect/modify/block)",
      "log.beforePersist (active request log redaction before enqueue)",
      "WASM",
      "public manifests use `capabilities`",
      "Extension Host public manifest 不支持 top-level `permissions`",
      "capability changes",
    ],
    forbiddenPhrases: [
      "granted permissions",
      "body-read permissions",
      "write permissions",
      "add permissions",
      "permission changes",
    ],
  },
  {
    path: "docs/plugin-manifest-v1.md",
    phrases: [
      "publisher.plugin-name",
      "SemVer",
      "apiVersion",
      "hostCompatibility",
      "main",
      'runtime.kind = "extensionHost"',
      "contributes.gatewayHooks",
      'capabilities: ["gateway.hooks"]',
      "api.gateway.registerHook",
      "commands -> commands.execute",
      "providers / provider UI -> provider.extensionValues",
      "gatewayHooks -> gateway.hooks",
      "protocolBridges -> protocol.bridge",
      "Protocol bridge MVP skeleton",
      "gateway.response.chunk",
      "Active hooks in plugin API v1",
      "Reserved hooks for future host integration",
      "Reserved permissions for future host-mediated APIs",
      "request.header.readSensitive",
      "official.privacy-filter",
      "acme.prompt-helper",
      "quarantined",
      "PLUGIN_INCOMPATIBLE_PLATFORM",
      "Extension Host public manifest 不支持 top-level `permissions`",
      "High-risk 和 critical labels",
    ],
    forbiddenPhrases: [
      '"kind": "wasm"',
      "0.62.x 只支持 Plugin API major",
      "当前代码实际阻断的是 `hostCompatibility.app` 和 `hostCompatibility.pluginApi`",
    ],
  },
  {
    path: "docs/plugins/README.md",
    phrases: [
      "插件开发手册",
      "插件开发总指南",
      "按目标查找",
      "插件 API 参考",
      "Privacy Filter 示例",
      "Manifest",
      "Hooks",
      "Permissions",
      "diagnostics.read",
      "api.diagnostics.getRuntimeReports",
      "强制执行的平台白名单",
    ],
    forbiddenPhrases: ["不参与本地安装阻断或市场兼容性筛选"],
  },
  {
    path: "docs/plugins/developer-guide.md",
    phrases: [
      "插件开发总指南",
      "plugin.json",
      "Extension Host",
      "doctor -> validate --strict -> pack -> publish-check -> install/update -> export replay fixture -> fix -> reinstall",
      "plugin_export_replay_fixture",
      "publish-check",
      "dist/extension.js",
      'runtime.kind = "extensionHost"',
      "contributes.gatewayHooks",
      "capabilities",
      "api.gateway.registerHook",
      "gateway.request.beforeSend",
      "request.normalizedMessages",
      "configSchema",
      "x-aio-ui",
      "仓库本地 checkout 不得运行任何 `pnpm` 命令",
      "GitHub Actions 中的 monorepo 工具合同",
      "GitHub Actions 在仓库 CI 中运行",
      "create-aio-plugin cli validate",
      "create-aio-plugin cli pack",
      "PLUGIN_REPLAY_UNSUPPORTED",
      ".aio-plugin",
      "official.privacy-filter",
      "精选插件",
      "高级来源",
      "example:prompt-helper",
      "example:redactor",
      "example:response-guard",
      "示例是开发模板，不是默认可安装市场包",
      "安装、更新、重验和启用时的强制白名单",
    ],
    forbiddenPhrases: [
      "WASM 适合需要确定性代码逻辑的插件",
      "pnpm plugin-wasm-sdk:test",
      "最小声明式规则插件",
      "pnpm --filter create-aio-plugin cli replay",
      legacyCreateAioPluginExecCommand,
      "Warnings do not fail the command in 0.62.1",
      "platforms` 当前只作为元数据展示",
    ],
  },
  {
    path: "docs/plugins/runtime/wasm.md",
    phrases: [
      "unsupported pre-release legacy runtime",
      "not part of the public Plugin API v1 community runtime surface",
      "community plugins must migrate to Extension Host",
      'runtime.kind = "extensionHost"',
    ],
    forbiddenPhrases: [
      "WASM packages are installable only when host policy enables execution",
      "WASM 只用于宿主策略启用后",
      "插件作者应使用",
    ],
  },
  {
    path: "docs/plugins/runtime/process-poc.md",
    phrases: [
      "unsupported pre-release legacy runtime",
      "not part of the public Plugin API v1 community runtime surface",
      "Extension Host",
      "JSON-RPC over stdio",
      "disabled by default",
    ],
    forbiddenPhrases: ["服务于未来无法放进 WASM ABI"],
  },
  {
    path: "docs/plugins/developer-guide.md",
    phrases: [
      "create-aio-plugin",
      "create-aio-plugin cli",
      "create-aio-plugin cli validate",
      "create-aio-plugin cli pack",
      "create-aio-plugin cli publish-check",
      "Plugins 页面执行本地包导入",
      "Claude 和 Codex request shapes",
      "@aio-coding-hub/plugin-sdk",
      "plugin.json",
      "最小 Extension Host 插件",
      "Extension Host",
    ],
  },
  {
    path: "docs/plugins/reference/sdk.md",
    phrases: [
      "@aio-coding-hub/plugin-sdk",
      "PluginManifest",
      "validateManifest",
      "permissionRisk",
      "Extension Host",
      'runtime: { kind: "extensionHost"',
      "api.gateway.registerHook",
      "SDK 边界",
    ],
    forbiddenPhrases: ["aio-plugin-wasm-sdk"],
  },
  {
    path: "docs/plugins/examples/privacy-filter.md",
    phrases: [
      "official.privacy-filter",
      "packyme/privacy-filter",
      "Extension Host",
      "privacy.redact",
      "api.privacy.redactRequestBody",
      "已移除的内置示例",
      "仓库本地 checkout 不得运行 package-manager 脚本",
      "GitHub Actions 或仓库外独立插件工作区",
    ],
    forbiddenPhrases: ["native:privacyFilter", "host-owned built-in"],
  },
  {
    path: "docs/plugins/examples/README.md",
    phrases: [
      "example:prompt-helper",
      "example:redactor",
      "example:response-guard",
      "fixtures/claude-request.json",
      "fixtures/response-warn.json",
      "不是默认可安装市场包",
      "checksum",
      "signature",
      "托管",
      "市场索引流程",
    ],
  },
  {
    path: "docs/plugins/architecture/audit.md",
    phrases: [
      "official.privacy-filter",
      "Extension Host",
      "gatewayHooks",
      "protocolBridges",
      "unsupported pre-release legacy runtime",
      "信任边界",
      "性能与稳定性建议",
      "The current host does not expose public provider plugin APIs",
    ],
    caseInsensitivePhrases: ["provider adapter facades remain internal"],
    forbiddenPhrases: ["## 0.62 Platform Kernel Decision"],
  },
  {
    path: "docs/plugins/reference/manifest.md",
    phrases: [
      "apiVersion",
      "hostCompatibility",
      'runtime.kind = "extensionHost"',
      "main",
      "contributes.gatewayHooks",
      "capabilities",
      "Protocol bridge MVP skeleton",
      "PLUGIN_INCOMPATIBLE_PLATFORM",
    ],
    forbiddenPhrases: [
      '{ "kind": "wasm"',
      "platforms` 当前是解析和展示元数据",
    ],
  },
  {
    path: "docs/plugins/reference/hooks.md",
    phrases: [
      "gateway.request.afterBodyRead",
      "gateway.response.chunk",
      "log.beforePersist",
      "plugin_hook_execution_reports",
      "plugin_export_replay_fixture",
      "默认 vNext hook timeout: 5000 ms",
    ],
  },
  {
    path: "docs/plugins/reference/permissions.md",
    phrases: ["request.body.read", "secret.read", "critical", "新增 capability 需要用户重新确认"],
  },
  {
    path: "docs/plugins/reference/config-schema.md",
    phrases: [
      "string",
      "number",
      "boolean",
      "password",
      "enum is supported as a keyword",
      "vNext does not provide host-managed secret storage",
    ],
  },
  {
    path: "docs/plugins/architecture/security.md",
    phrases: [
      "fail-closed",
      "quarantined",
      "Extension Host",
      "不在 Rust 主进程或 Tauri WebView 执行第三方插件代码",
      "默认 vNext hook timeout: 5000 ms",
    ],
  },
  {
    path: "docs/plugins/runtime/streaming.md",
    phrases: ["sliding window", "gateway.response.chunk", "stream.modify"],
  },
  {
    path: "docs/plugins/reference/publishing.md",
    phrases: [
      ".aio-plugin",
      "sha256",
      "Ed25519",
      "rollback",
      "publish-check",
      "market index URL",
      "trusted public key",
      "revoked / incompatible install blocks",
      "plugin_export_replay_fixture",
      "默认市场视图",
      "自定义 market index 属于高级来源",
      "GitHub Actions 可以对示例模板运行 publish-check",
      "不代表示例已经被上传、签名、加入默认 market index",
      "hostCompatibility.platforms",
      "阻断安装",
      "仓库贡献者不得在本地 checkout 运行 package-manager 脚本",
      "GitHub Actions 所有的 CI 合同",
    ],
    forbiddenPhrases: ["## 0.62.2 生命周期行为", "当前只作为元数据展示"],
  },
  {
    path: "docs/plugins/runtime/README.md",
    phrases: [
      "Host Runtime Lifecycle",
      "plugin_hook_execution_reports",
      "host-owned lifecycle",
      "Dispose",
      "diagnostics.read",
      "api.diagnostics.getRuntimeReports",
      "1..100",
    ],
    forbiddenPhrases: [
      "without exposing a new plugin-callable diagnostics API",
      "0.62.3 treats runtime lifecycle",
    ],
  },
  {
    path: "docs/plugins/reference/compatibility.md",
    phrases: [
      "SemVer",
      "pluginApi",
      "platforms",
      "Plugin API v1 remains externally compatible with the current host",
      "The current host does not expose public provider plugin APIs",
      "Extension Host is the only community runtime",
      "unsupported pre-release legacy runtime",
      "PLUGIN_INCOMPATIBLE_PLATFORM",
      "api.diagnostics.getRuntimeReports",
    ],
    forbiddenPhrases: [
      '{ "kind": "wasm"',
      "0.62",
      "当前不会因为缺少当前桌面平台而阻断",
    ],
  },
  {
    path: "docs/plugins/plugin-api-v1-contract.json",
    phrases: [
      '"diagnostics.read"',
      '"api.diagnostics.getRuntimeReports"',
      '"diagnosticsReadBoundary"',
      '"defaultLimit": 20',
      '"limitRange": [1, 100]',
      "PLUGIN_INCOMPATIBLE_PLATFORM",
    ],
    forbiddenPhrases: ["只作为元数据和展示字段"],
  },
];

const failures = [];

const requiredSourceContracts = [
  {
    path: "src-tauri/src/domain/plugins.rs",
    checks: [
      {
        label: "platform compatibility rejects an undeclared current platform",
        pattern:
          /fn validate_host_compatibility[\s\S]*?current_plugin_platform\(\)[\s\S]*?compatibility\.platforms[\s\S]*?PLUGIN_INCOMPATIBLE_PLATFORM/,
      },
      {
        label: "current platform is derived from the host OS",
        pattern: /fn current_plugin_platform\(\)[\s\S]*?std::env::consts::OS/,
      },
    ],
  },
  {
    path: "src-tauri/src/app/plugins/extension_host.rs",
    checks: [
      {
        label: "diagnostics API requires diagnostics.read and scopes reports to the owning plugin",
        pattern:
          /fn diagnostics_get_runtime_reports[\s\S]*?require_capability\("diagnostics\.read"\)\?[\s\S]*?host_api_plugin_id\(&params\)\?[\s\S]*?list_extension_execution_reports\([\s\S]*?Some\(plugin_id\)/,
      },
      {
        label: "diagnostics API defaults to 20 and clamps the requested limit to 1..100",
        pattern: /fn diagnostics_get_runtime_reports[\s\S]*?unwrap_or\(20\)[\s\S]*?\.clamp\(1, 100\)/,
      },
      {
        label: "host API rejects a pluginId that differs from the Extension Host owner",
        pattern:
          /fn host_api_plugin_id[\s\S]*?plugin_id != self\.plugin_id[\s\S]*?PLUGIN_EXTENSION_HOST_FORBIDDEN/,
      },
    ],
  },
  {
    path: "packages/plugin-sdk/src/index.ts",
    checks: [
      {
        label: "SDK exposes diagnostics only through the optional capability-gated namespace",
        pattern:
          /Available only when the plugin declares the `diagnostics\.read` capability[\s\S]*?getRuntimeReports\(limit\?: number\)[\s\S]*?diagnostics\?: DiagnosticsApi/,
      },
    ],
  },
];

const versionIndependentCurrentDocs = [
  "docs/plugin-manifest-v1.md",
  "docs/plugins/README.md",
  "docs/plugins/developer-guide.md",
  "docs/plugins/reference/compatibility.md",
  "docs/plugins/reference/manifest.md",
  "docs/plugins/reference/publishing.md",
  "docs/plugins/runtime/README.md",
  "docs/plugins/architecture/README.md",
  "docs/plugins/architecture/audit.md",
];

const localReplayBoundaryFiles = [
  "docs/plugins/README.md",
  "docs/plugins/developer-guide.md",
  "docs/plugins/reference/sdk.md",
  "docs/plugins/reference/compatibility.md",
  "docs/plugins/architecture/audit.md",
  "docs/plugins/examples/README.md",
  "docs/plugins/examples/privacy-filter.md",
  "docs/plugins/reference/publishing.md",
  "packages/create-aio-plugin/src/scaffold.ts",
  "packages/create-aio-plugin/src/scaffold.test.ts",
  "packages/create-aio-plugin/src/devtools.ts",
];

const replaySuccessPatterns = [
  /pnpm --filter create-aio-plugin cli replay/,
  /\bcreate-aio-plugin\s+replay\b/,
  /\breplay --explain\b/,
  /validate[\s\S]{0,80}replay[\s\S]{0,80}pack/,
];

const supersededHistoricalDocsFallback = [
  "docs/superpowers/plans/2026-06-22-aio-coding-hub-0-62-1-plugin-developer-loop.md",
  "docs/superpowers/plans/2026-06-22-aio-coding-hub-0-62-gateway-first-plugin-kernel.md",
  "docs/superpowers/plans/2026-06-25-aio-coding-hub-plugin-observability-replay-publishing.md",
  "docs/superpowers/plans/2026-06-26-aio-coding-hub-plugin-example-developer-loop-phase-1.md",
  "docs/superpowers/specs/2026-06-21-aio-coding-hub-0-62-plugin-platform-kernel-design.md",
  "docs/superpowers/specs/2026-06-22-aio-coding-hub-0-62-1-plugin-developer-loop-design.md",
  "docs/superpowers/specs/2026-06-22-aio-coding-hub-0-62-gateway-first-plugin-kernel-design.md",
  "docs/superpowers/specs/2026-06-25-aio-coding-hub-plugin-observability-replay-publishing-design.md",
  "docs/superpowers/specs/2026-06-26-aio-coding-hub-plugin-example-developer-loop-phase-1-design.md",
  "docs/superpowers/specs/2026-06-27-aio-coding-hub-plugin-runtime-lifecycle-registry-design.md",
];

function lineExplainsReplayUnsupported(line) {
  return (
    line.includes("PLUGIN_REPLAY_UNSUPPORTED") ||
    line.includes("unsupported for Extension Host") ||
    line.includes("当前不执行 Extension Host gateway hooks") ||
    line.includes("不在本地执行 Extension Host gateway hooks") ||
    line.includes("not local `create-aio-plugin replay` execution") ||
    line.includes("not.toContain")
  );
}

function trackedSuperpowersMarkdownDocs() {
  const result = spawnSync(
    "git",
    ["ls-files", "docs/superpowers/plans", "docs/superpowers/specs"],
    {
      cwd: repoRoot,
      encoding: "utf8",
    }
  );
  if (result.status !== 0) {
    return supersededHistoricalDocsFallback;
  }
  return result.stdout.split(/\r?\n/).filter((path) => path.endsWith(".md"));
}

function hasReplaySuccessPath(text) {
  return replaySuccessPatterns.some((pattern) => pattern.test(text));
}

function hasSupersededHistoricalSuccessPath(text) {
  return hasReplaySuccessPath(text);
}

for (const doc of requiredDocs) {
  const fullPath = join(repoRoot, doc.path);
  if (!existsSync(fullPath)) {
    failures.push(`${doc.path}: missing required document`);
    continue;
  }

  const text = readFileSync(fullPath, "utf8");
  for (const phrase of doc.phrases) {
    if (!text.includes(phrase)) {
      failures.push(`${doc.path}: missing required phrase "${phrase}"`);
    }
  }

  const normalizedText = text.toLowerCase();
  for (const phrase of doc.caseInsensitivePhrases ?? []) {
    if (!normalizedText.includes(phrase.toLowerCase())) {
      failures.push(`${doc.path}: missing required phrase "${phrase}"`);
    }
  }

  for (const phrase of doc.forbiddenPhrases ?? []) {
    if (text.includes(phrase)) {
      failures.push(`${doc.path}: forbidden phrase "${phrase}"`);
    }
  }
}

for (const sourceContract of requiredSourceContracts) {
  const fullPath = join(repoRoot, sourceContract.path);
  if (!existsSync(fullPath)) {
    failures.push(`${sourceContract.path}: missing required source contract`);
    continue;
  }
  const source = readFileSync(fullPath, "utf8");
  for (const check of sourceContract.checks) {
    if (!check.pattern.test(source)) {
      failures.push(`${sourceContract.path}: missing source contract for ${check.label}`);
    }
  }
}

for (const path of versionIndependentCurrentDocs) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path}: missing current plugin document`);
    continue;
  }
  if (/\b0\.62(?:\.\d+|\.x)?\b/.test(readFileSync(fullPath, "utf8"))) {
    failures.push(`${path}: current plugin documentation must not bind Plugin API v1 to 0.62`);
  }
}

for (const path of localReplayBoundaryFiles) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path}: missing local replay boundary file`);
    continue;
  }
  const lines = readFileSync(fullPath, "utf8").split(/\r?\n/);
  lines.forEach((line, index) => {
    if (lineExplainsReplayUnsupported(line)) return;
    if (replaySuccessPatterns.some((pattern) => pattern.test(line))) {
      failures.push(
        `${path}:${index + 1}: local create-aio-plugin replay must not be documented as a successful Extension Host hook path`
      );
    }
  });
}

for (const path of trackedSuperpowersMarkdownDocs()) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path}: missing superseded historical document`);
    continue;
  }
  const text = readFileSync(fullPath, "utf8");
  if (!hasSupersededHistoricalSuccessPath(text)) continue;
  const head = text.split(/\r?\n/).slice(0, 16).join("\n");
  if (!head.includes("Status: Superseded.") || !head.includes("MUST NOT be executed")) {
    failures.push(`${path}: historical local replay public runtime plan must be marked superseded`);
  }
}

if (failures.length > 0) {
  console.error("Plugin system documentation contract failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
