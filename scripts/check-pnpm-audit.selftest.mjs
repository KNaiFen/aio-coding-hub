// check-pnpm-audit.mjs 纯逻辑自检：树遍历收集、级别计数、阻断判定、阻断明细。
import assert from "node:assert/strict";

import {
  collectPackageVersions,
  evaluateBlockingAdvisories,
  extractSeverityCounts,
  formatBlockingAdvisories,
  hasBlockingVulnerabilities,
} from "./check-pnpm-audit.mjs";

// 正常路径：跨项目递归收集 name -> versions，去重且覆盖 optionalDependencies。
{
  const projects = [
    {
      dependencies: {
        foo: { version: "1.0.0", dependencies: { bar: { version: "2.0.0" } } },
        dup: { version: "3.0.0" },
      },
      optionalDependencies: {
        opt: { version: "4.0.0" },
      },
    },
    {
      dependencies: {
        dup: { version: "3.0.0" },
        linked: { version: "link:packages/x" },
        workspacePkg: { version: "workspace:*" },
      },
    },
  ];

  const collected = collectPackageVersions(projects);
  assert.deepEqual(
    Object.fromEntries([...collected].map(([name, versions]) => [name, [...versions].sort()])),
    {
      foo: ["1.0.0"],
      bar: ["2.0.0"],
      dup: ["3.0.0"],
      opt: ["4.0.0"],
    }
  );
}

// 精确豁免：仅匹配 package + GHSA，且到期后自动恢复阻断。
{
  const advisory = {
    severity: "high",
    title: "RSC-only issue",
    url: "https://github.com/advisories/GHSA-qwww-vcr4-c8h2",
  };
  const exceptions = [
    {
      packageName: "react-router",
      advisoryId: "GHSA-QWWW-VCR4-C8H2",
      expiresOn: "2026-10-27",
      reason: "RSC is not enabled.",
    },
  ];

  const active = evaluateBlockingAdvisories(
    { "react-router": [advisory] },
    new Date("2026-10-27T23:59:59.999Z"),
    exceptions
  );
  assert.equal(active.blocking.length, 0);
  assert.equal(active.exempted.length, 1);

  const expired = evaluateBlockingAdvisories(
    { "react-router": [advisory] },
    new Date("2026-10-28T00:00:00.000Z"),
    exceptions
  );
  assert.equal(expired.blocking.length, 1);
  assert.equal(expired.exempted.length, 0);
  assert.equal(expired.blocking[0].expiredException, exceptions[0]);

  const wrongPackage = evaluateBlockingAdvisories(
    { "another-package": [advisory] },
    new Date("2026-07-27T00:00:00.000Z"),
    exceptions
  );
  assert.equal(wrongPackage.blocking.length, 1);
  assert.equal(wrongPackage.exempted.length, 0);

  const wrongAdvisory = evaluateBlockingAdvisories(
    {
      "react-router": [
        {
          ...advisory,
          url: "https://github.com/advisories/GHSA-chx6-hx7r-mcp5",
        },
      ],
    },
    new Date("2026-07-27T00:00:00.000Z"),
    exceptions
  );
  assert.equal(wrongAdvisory.blocking.length, 1);
  assert.equal(wrongAdvisory.exempted.length, 0);

  assert.throws(
    () =>
      evaluateBlockingAdvisories(
        { "react-router": [advisory] },
        new Date("2026-07-27T00:00:00.000Z"),
        [{ ...exceptions[0], expiresOn: "2026-02-31" }]
      ),
    /Invalid exception expiry/
  );
}

// 边界：空输入、非法节点、缺失 version 都不产出也不抛错。
{
  assert.equal(collectPackageVersions([]).size, 0);
  assert.equal(collectPackageVersions(null).size, 0);
  assert.equal(collectPackageVersions([{ dependencies: { bad: null, noVersion: {} } }]).size, 0);
}

// 正常路径：按 advisory 逐条计数，并归一已知级别的大小写。
{
  const advisoriesByPackage = {
    lodash: [{ severity: "high" }, { severity: "moderate" }],
    minimatch: [{ severity: "HIGH" }],
  };
  const counts = extractSeverityCounts(advisoriesByPackage);
  assert.deepEqual(counts, { info: 0, low: 0, moderate: 1, high: 2, critical: 0 });
  assert.equal(hasBlockingVulnerabilities(counts), true);
}

// 安全边界：不能解释的成功响应必须 fail closed，不能静默计为零。
{
  assert.throws(() => extractSeverityCounts({ error: "registry unavailable" }), /unexpected/i);
  assert.throws(() => extractSeverityCounts({ lodash: { severity: "critical" } }), /unexpected/i);
  assert.throws(
    () => extractSeverityCounts({ lodash: [{ severity: "future-critical" }] }),
    /unexpected/i
  );
  assert.throws(() => extractSeverityCounts({ lodash: [null] }), /unexpected/i);
  assert.throws(() => extractSeverityCounts({ lodash: [{}] }), /unexpected/i);
}

// 失败路径反例：只有低危不阻断。
{
  const counts = extractSeverityCounts({ lodash: [{ severity: "low" }] });
  assert.deepEqual(counts, { info: 0, low: 1, moderate: 0, high: 0, critical: 0 });
  assert.equal(hasBlockingVulnerabilities(counts), false);
}

// 阻断明细：只列出 high / critical，带标题与链接。
{
  const lines = formatBlockingAdvisories({
    lodash: [
      { severity: "critical", title: "RCE", url: "https://example.test/a" },
      { severity: "low", title: "noise", url: "https://example.test/b" },
    ],
  });
  assert.deepEqual(lines, ["[pnpm-audit] critical: lodash — RCE (https://example.test/a)"]);
}

console.error("[pnpm-audit:selftest] 全部断言通过。");
