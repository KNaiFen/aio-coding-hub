/**
 * Chart helper utilities
 * Shared functions for data transformation and formatting
 */

/**
 * Format tokens in millions/thousands for chart axes
 */
export function formatTokensMillions(value: number): string {
  if (!Number.isFinite(value) || value === 0) return "0";
  const billions = value / 1_000_000_000;
  if (billions >= 1) {
    return `${billions.toFixed(1)}B`;
  }
  const millions = value / 1_000_000;
  if (millions >= 1) {
    return `${millions.toFixed(1)}M`;
  }
  if (value >= 1000) {
    return `${(value / 1000).toFixed(1)}K`;
  }
  return String(Math.round(value));
}

export function computeYAxisWidth(
  tickValues: readonly number[],
  formatter: (value: number) => string = formatTokensMillions,
  minimumWidth = 45
): number {
  const longestLabelLength = tickValues.reduce(
    (longest, value) => Math.max(longest, formatter(value).length),
    0
  );
  return Math.max(minimumWidth, Math.ceil(longestLabelLength * 7 + 12));
}

/**
 * Compute nice Y-axis bounds with fixed interval
 * Returns { max, interval } to ensure Y-axis ticks are evenly distributed
 */
export function computeNiceYAxis(
  maxValue: number,
  tickCount = 5
): { max: number; interval: number } {
  if (maxValue <= 0) {
    return { max: 1_000_000, interval: 200_000 };
  }

  // Compute rough interval
  const roughInterval = maxValue / tickCount;

  // Compute magnitude (power of 10)
  const magnitude = Math.pow(10, Math.floor(Math.log10(roughInterval)));

  // Choose a "nice" multiplier: 1, 2, 2.5, 5, 10
  const normalized = roughInterval / magnitude;
  let niceMultiplier: number;
  if (normalized <= 1) {
    niceMultiplier = 1;
  } else if (normalized <= 2) {
    niceMultiplier = 2;
  } else if (normalized <= 2.5) {
    niceMultiplier = 2.5;
  } else if (normalized <= 5) {
    niceMultiplier = 5;
  } else {
    niceMultiplier = 10;
  }

  const niceInterval = niceMultiplier * magnitude;
  const niceMax = Math.ceil(maxValue / niceInterval) * niceInterval;

  return { max: niceMax, interval: niceInterval };
}

/**
 * Convert day key (YYYY-MM-DD) to MM/DD format
 */
export function toDateLabel(dayKey: string): string {
  const mmdd = dayKey.slice(5);
  return mmdd.replace("-", "/");
}

/**
 * Escape HTML in tooltip strings
 */
export function escapeHtml(input: string): string {
  return input
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * Pick top N slices from data, aggregating tail into "Other"
 */
export function pickTopSlices<T extends { cost_usd: number }>(
  rows: T[],
  topN: number
): { head: T[]; tailSum: number } {
  const sorted = rows.slice().sort((a, b) => b.cost_usd - a.cost_usd);
  const head = sorted.slice(0, Math.max(1, Math.floor(topN)));
  const tail = sorted.slice(head.length);
  const tailSum = tail.reduce((acc, cur) => acc + (Number(cur.cost_usd) || 0), 0);
  return { head, tailSum };
}
