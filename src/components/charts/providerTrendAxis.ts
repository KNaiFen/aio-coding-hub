export type ProviderTrendBucketRow = {
  day: string;
  hour: number | null;
  granularity: "hour" | "day" | "week" | "month" | "year";
};

export type ProviderTrendLabelContext = {
  hourlyDayCount: number;
  yearCount: number;
};

export function providerTrendBucketKey(row: ProviderTrendBucketRow): string {
  if (row.granularity === "hour" && row.hour != null) {
    return `${row.day}T${String(row.hour).padStart(2, "0")}`;
  }
  return row.day;
}

export function providerTrendLabelContext(
  rows: readonly ProviderTrendBucketRow[]
): ProviderTrendLabelContext {
  return {
    hourlyDayCount: new Set(rows.filter((row) => row.granularity === "hour").map((row) => row.day))
      .size,
    yearCount: new Set(rows.map((row) => row.day.slice(0, 4))).size,
  };
}

export function providerTrendBucketLabel(
  row: ProviderTrendBucketRow,
  context: ProviderTrendLabelContext
): string {
  if (row.granularity === "hour" && row.hour != null) {
    const hour = `${String(row.hour).padStart(2, "0")}:00`;
    if (context.hourlyDayCount <= 1) return hour;
    const day = context.yearCount > 1 ? row.day : row.day.slice(5).replace("-", "/");
    return `${day} ${hour}`;
  }
  if (row.granularity === "day" || row.granularity === "week") {
    return context.yearCount > 1 ? row.day : row.day.slice(5).replace("-", "/");
  }
  return row.day;
}

export function buildProviderTrendTicks(labels: readonly string[], maxTicks = 8): string[] {
  if (labels.length <= maxTicks) return [...labels];
  const step = Math.ceil(labels.length / maxTicks);
  const ticks = labels.filter((_, index) => index % step === 0);
  const last = labels[labels.length - 1];
  if (last && ticks[ticks.length - 1] !== last) ticks.push(last);
  return ticks;
}
