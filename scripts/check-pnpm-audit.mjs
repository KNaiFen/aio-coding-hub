// npm 已于 2026-07 下线旧审计端点 /-/npm/v1/security/audits(/quick)，一律返回 410，
// pnpm 10.x 的 `pnpm audit` 因此不可用（修复只在 pnpm 11，大版本升级另行处理）。
// 这里改为直连 npm CLI / pnpm 11 使用的 bulk advisory 端点：提交 name -> versions 清单，
// 注册表按提交的版本过滤并返回命中的 advisory。
// ponytail: 端点写死 registry.npmjs.org；如迁移私有 registry，需要改为从配置推导。
import { spawnSync } from "node:child_process";
import { dirname } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const logger = {
  info(message, ...args) {
    console.error(message, ...args);
  },
  error(message, ...args) {
    console.error(message, ...args);
  },
};

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);
const AUDIT_SEVERITIES = Object.freeze(["info", "low", "moderate", "high", "critical"]);
const BLOCKING_SEVERITIES = Object.freeze(["high", "critical"]);
const AUDIT_EXCEPTIONS = Object.freeze([
  Object.freeze({
    packageName: "react-router",
    advisoryId: "GHSA-QWWW-VCR4-C8H2",
    expiresOn: "2026-10-27",
    reason:
      "This Tauri SPA uses only client-side routers and does not enable React Router RSC mode or Server Actions.",
  }),
]);
const pnpmCommand = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
const auditRegistry = process.env.PNPM_AUDIT_REGISTRY?.trim() || "https://registry.npmjs.org/";
const BULK_ADVISORY_ENDPOINT = new URL(
  "-/npm/v1/security/advisories/bulk",
  auditRegistry.endsWith("/") ? auditRegistry : `${auditRegistry}/`
).toString();

function pnpmListCommand() {
  const args = ["list", "-r", "--prod", "--depth", "Infinity", "--json"];
  if (process.platform !== "win32") {
    return { command: pnpmCommand, args };
  }

  return {
    command: process.env.ComSpec || "cmd.exe",
    args: ["/d", "/s", "/c", pnpmCommand, ...args],
  };
}

export function collectPackageVersions(projects) {
  const versionsByName = new Map();

  const visit = (dependencies) => {
    if (!dependencies || typeof dependencies !== "object") {
      return;
    }
    for (const [name, node] of Object.entries(dependencies)) {
      if (!node || typeof node !== "object") {
        continue;
      }
      // 只上报 registry 版本；跳过 link: / file: / workspace: 等本地依赖。
      if (typeof node.version === "string" && /^\d/.test(node.version)) {
        const versions = versionsByName.get(name) ?? new Set();
        versions.add(node.version);
        versionsByName.set(name, versions);
      }
      visit(node.dependencies);
      visit(node.optionalDependencies);
    }
  };

  for (const project of Array.isArray(projects) ? projects : []) {
    if (!project || typeof project !== "object") {
      continue;
    }
    visit(project.dependencies);
    visit(project.optionalDependencies);
  }

  return versionsByName;
}

export function validateAdvisoriesByPackage(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("[pnpm-audit] bulk advisory endpoint returned an unexpected payload shape.");
  }

  for (const advisories of Object.values(value)) {
    if (!Array.isArray(advisories)) {
      throw new Error("[pnpm-audit] bulk advisory endpoint returned an unexpected advisory list.");
    }
    for (const advisory of advisories) {
      if (!advisory || typeof advisory !== "object" || Array.isArray(advisory)) {
        throw new Error(
          "[pnpm-audit] bulk advisory endpoint returned an unexpected advisory entry."
        );
      }
      const severity = typeof advisory.severity === "string" ? advisory.severity.toLowerCase() : "";
      if (!AUDIT_SEVERITIES.includes(severity)) {
        throw new Error("[pnpm-audit] bulk advisory endpoint returned an unexpected severity.");
      }
    }
  }

  return value;
}

export function extractSeverityCounts(advisoriesByPackage) {
  validateAdvisoriesByPackage(advisoriesByPackage);
  const counts = {
    info: 0,
    low: 0,
    moderate: 0,
    high: 0,
    critical: 0,
  };

  for (const advisories of Object.values(advisoriesByPackage)) {
    for (const advisory of advisories) {
      const severity = advisory.severity.toLowerCase();
      counts[severity] += 1;
    }
  }

  return counts;
}

export function hasBlockingVulnerabilities(counts) {
  return BLOCKING_SEVERITIES.some((severity) => counts[severity] > 0);
}

export function formatCounts(counts) {
  return Object.entries(counts)
    .map(([severity, count]) => `${severity}=${count}`)
    .join(", ");
}

function blockingAdvisoryEntries(advisoriesByPackage) {
  validateAdvisoriesByPackage(advisoriesByPackage);
  const entries = [];
  for (const [name, advisories] of Object.entries(advisoriesByPackage)) {
    for (const advisory of advisories) {
      const severity = advisory.severity.toLowerCase();
      if (BLOCKING_SEVERITIES.includes(severity)) {
        entries.push({ name, advisory, severity });
      }
    }
  }
  return entries;
}

function advisoryIdFromUrl(advisory) {
  if (typeof advisory.url !== "string") {
    return null;
  }

  try {
    const url = new URL(advisory.url);
    if (url.protocol !== "https:" || url.hostname !== "github.com" || url.search || url.hash) {
      return null;
    }
    const match = /^\/advisories\/(GHSA-[a-z0-9-]+)\/?$/i.exec(url.pathname);
    return match?.[1].toUpperCase() ?? null;
  } catch {
    return null;
  }
}

function exceptionExpiryMs(exception) {
  if (typeof exception.expiresOn !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(exception.expiresOn)) {
    throw new Error(`[pnpm-audit] Invalid exception expiry: ${exception.expiresOn ?? "missing"}`);
  }
  const expiryMs = Date.parse(`${exception.expiresOn}T23:59:59.999Z`);
  if (
    !Number.isFinite(expiryMs) ||
    new Date(expiryMs).toISOString().slice(0, 10) !== exception.expiresOn
  ) {
    throw new Error(`[pnpm-audit] Invalid exception expiry: ${exception.expiresOn}`);
  }
  return expiryMs;
}

function exceptionMap(exceptions) {
  const byPackageAndId = new Map();
  for (const exception of exceptions) {
    if (
      !exception ||
      typeof exception.packageName !== "string" ||
      !/^GHSA-[A-Z0-9-]+$/.test(exception.advisoryId ?? "") ||
      typeof exception.reason !== "string" ||
      exception.reason.trim() === ""
    ) {
      throw new Error("[pnpm-audit] Invalid advisory exception configuration.");
    }
    exceptionExpiryMs(exception);
    const key = `${exception.packageName}\0${exception.advisoryId}`;
    if (byPackageAndId.has(key)) {
      throw new Error(`[pnpm-audit] Duplicate advisory exception: ${exception.advisoryId}`);
    }
    byPackageAndId.set(key, exception);
  }
  return byPackageAndId;
}

export function evaluateBlockingAdvisories(
  advisoriesByPackage,
  now = new Date(),
  exceptions = AUDIT_EXCEPTIONS
) {
  const nowMs = now instanceof Date ? now.getTime() : Number.NaN;
  if (!Number.isFinite(nowMs)) {
    throw new Error("[pnpm-audit] Invalid audit evaluation time.");
  }

  const configuredExceptions = exceptionMap(exceptions);
  const blocking = [];
  const exempted = [];
  for (const entry of blockingAdvisoryEntries(advisoriesByPackage)) {
    const advisoryId = advisoryIdFromUrl(entry.advisory);
    const exception = configuredExceptions.get(`${entry.name}\0${advisoryId}`);
    if (exception && nowMs <= exceptionExpiryMs(exception)) {
      exempted.push({ ...entry, advisoryId, exception });
    } else {
      blocking.push({ ...entry, advisoryId, expiredException: exception ?? null });
    }
  }

  return { blocking, exempted };
}

export function formatBlockingAdvisories(advisoriesByPackage) {
  return blockingAdvisoryEntries(advisoriesByPackage).map(
    ({ name, advisory, severity }) =>
      `[pnpm-audit] ${severity}: ${name} — ${advisory.title ?? "untitled advisory"} (${advisory.url ?? "no url"})`
  );
}

async function main() {
  /*
   * ============================================================================
   * 步骤1：执行 fail-close 的依赖审计（bulk advisory 端点）
   * ============================================================================
   * 目标：
   *   1) 只把 high / critical 视为阻断阈值
   *   2) 任何网络异常、命令异常、输出异常都按失败处理
   * 数据源：
   *   1) pnpm list -r --prod --depth Infinity --json（与旧 `pnpm audit --prod` 的
   *      workspace 生产依赖范围一致）
   *   2) npm bulk advisory 端点返回的 advisory 列表
   * 操作要点：
   *   1) 注册表按提交的版本做服务端过滤，返回即命中，无需本地 semver 比对
   *   2) 只有在响应可解析且 blocking 计数为 0 时才允许通过
   */
  logger.info("[pnpm-audit] 开始执行依赖审计...");

  // 1.1 枚举 workspace 全部生产依赖
  const listCommand = pnpmListCommand();
  const result = spawnSync(listCommand.command, listCommand.args, {
    cwd: repoRoot,
    encoding: "utf8",
    env: process.env,
    maxBuffer: 64 * 1024 * 1024,
  });

  if (result.error) {
    logger.error(result.stderr || result.stdout || "");
    throw result.error;
  }
  if (result.signal) {
    throw new Error(`pnpm list terminated by signal: ${result.signal}`);
  }
  if (result.status !== 0) {
    logger.error(result.stderr || result.stdout || "");
    throw new Error(
      `[pnpm-audit] pnpm list exited with status ${result.status}, refusing to fail-open.`
    );
  }

  const versionsByName = collectPackageVersions(JSON.parse(result.stdout));
  if (versionsByName.size === 0) {
    throw new Error("[pnpm-audit] pnpm list produced no auditable packages.");
  }
  logger.info("[pnpm-audit] 待审计包数量：%d", versionsByName.size);

  // 1.2 查询 bulk advisory 端点
  const requestBody = Object.fromEntries(
    [...versionsByName].map(([name, versions]) => [name, [...versions]])
  );
  const response = await fetch(BULK_ADVISORY_ENDPOINT, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(requestBody),
    signal: AbortSignal.timeout(60_000),
  });
  if (!response.ok) {
    const detail = (await response.text()).slice(0, 500);
    throw new Error(
      `[pnpm-audit] bulk advisory endpoint responded with ${response.status}: ${detail}`
    );
  }
  const advisoriesByPackage = validateAdvisoriesByPackage(await response.json());

  // 1.3 统计各级别命中数，只要出现 blocking 漏洞就直接失败
  const counts = extractSeverityCounts(advisoriesByPackage);
  logger.info("[pnpm-audit] 审计结果：%s", formatCounts(counts));

  const evaluation = evaluateBlockingAdvisories(advisoriesByPackage);
  for (const { name, advisoryId, exception } of evaluation.exempted) {
    logger.info(
      "[pnpm-audit] exception: %s/%s allowed through %s — %s",
      name,
      advisoryId,
      exception.expiresOn,
      exception.reason
    );
  }

  if (evaluation.blocking.length > 0) {
    for (const { name, advisory, severity, expiredException } of evaluation.blocking) {
      logger.error(
        `[pnpm-audit] ${severity}: ${name} — ${advisory.title ?? "untitled advisory"} (${advisory.url ?? "no url"})`
      );
      if (expiredException) {
        logger.error(
          `[pnpm-audit] exception expired on ${expiredException.expiresOn}: ${expiredException.advisoryId}`
        );
      }
    }
    throw new Error("[pnpm-audit] Detected blocking vulnerabilities (high/critical).");
  }

  logger.info("[pnpm-audit] 依赖审计通过。");
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
