import { cn } from "../../utils/cn";

export type ProviderTrendLegendItem = {
  key: string;
  name: string;
  color: string;
};

export function ProviderTrendLegend({
  providers,
  hiddenProviders,
  onToggle,
}: {
  providers: readonly ProviderTrendLegendItem[];
  hiddenProviders: ReadonlySet<string>;
  onToggle: (providerKey: string) => void;
}) {
  return (
    <div
      className="flex h-9 shrink-0 items-center gap-1.5 overflow-x-auto pb-1"
      role="group"
      aria-label="供应商图例"
    >
      {providers.map((provider) => {
        const visible = !hiddenProviders.has(provider.key);
        return (
          <button
            key={provider.key}
            type="button"
            aria-pressed={visible}
            onClick={() => onToggle(provider.key)}
            className={cn(
              "inline-flex h-7 max-w-48 shrink-0 items-center gap-1.5 rounded-md border px-2 text-xs transition-colors",
              visible
                ? "border-border bg-background text-foreground"
                : "border-transparent bg-secondary text-muted-foreground"
            )}
          >
            <span
              aria-hidden="true"
              className="h-2 w-2 shrink-0 rounded-full"
              style={{ backgroundColor: provider.color, opacity: visible ? 1 : 0.35 }}
            />
            <span className="truncate">{provider.name}</span>
          </button>
        );
      })}
    </div>
  );
}
