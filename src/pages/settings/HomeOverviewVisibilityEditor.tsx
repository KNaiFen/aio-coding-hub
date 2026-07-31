import { useSyncExternalStore } from "react";
import { CLIS, type CliKey } from "../../constants/clis";
import {
  readHomeOverviewVisibilityFromStorage,
  subscribeHomeOverviewVisibility,
  writeHomeOverviewVisibilityToStorage,
  type HomeOverviewVisibility,
} from "../../services/home/homeOverviewVisibility";
import {
  HOME_OVERVIEW_TABS,
  type HomeOverviewTabKey,
} from "../../services/home/homeOverviewTabOrder";
import { Switch } from "../../ui/Switch";

type VisibilityEditorKind = "tabs" | "clis";

type VisibilityItem<K extends string> = {
  key: K;
  label: string;
};

const TAB_ITEMS: VisibilityItem<HomeOverviewTabKey>[] = HOME_OVERVIEW_TABS;
const CLI_ITEMS: VisibilityItem<CliKey>[] = CLIS.map((cli) => ({
  key: cli.key,
  label: cli.name,
}));

function updateHiddenKeys<K extends string>(current: readonly K[], key: K, visible: boolean): K[] {
  const hidden = new Set(current);
  if (visible) {
    hidden.delete(key);
  } else {
    hidden.add(key);
  }
  return Array.from(hidden);
}

function VisibilitySwitchGroup<K extends string>({
  ariaLabel,
  items,
  hiddenKeys,
  onChange,
}: {
  ariaLabel: string;
  items: VisibilityItem<K>[];
  hiddenKeys: readonly K[];
  onChange: (key: K, visible: boolean) => void;
}) {
  const hidden = new Set(hiddenKeys);
  const visibleCount = items.length - hidden.size;

  return (
    <div role="group" aria-label={ariaLabel} className="grid w-full gap-1 sm:grid-cols-2">
      {items.map((item) => {
        const visible = !hidden.has(item.key);
        const isLastVisible = visible && visibleCount === 1;
        const controlLabel = `${ariaLabel}：${item.label}`;

        return (
          <div
            key={item.key}
            className="flex min-h-10 items-center justify-between gap-3 rounded-lg border border-line-subtle bg-background/60 px-3 py-2"
          >
            <span className="min-w-0 truncate text-sm font-medium text-foreground">
              {item.label}
            </span>
            <Switch
              checked={visible}
              onCheckedChange={(next) => onChange(item.key, next)}
              aria-label={controlLabel}
              disabled={isLastVisible}
              title={isLastVisible ? "至少保留一个显示项" : undefined}
            />
          </div>
        );
      })}
    </div>
  );
}

export function HomeOverviewVisibilityEditor({ kind }: { kind: VisibilityEditorKind }) {
  const visibility = useSyncExternalStore(
    subscribeHomeOverviewVisibility,
    readHomeOverviewVisibilityFromStorage,
    () => readHomeOverviewVisibilityFromStorage()
  );

  function write(next: HomeOverviewVisibility) {
    writeHomeOverviewVisibilityToStorage(next);
  }

  if (kind === "tabs") {
    return (
      <VisibilitySwitchGroup
        ariaLabel="首页信息面板"
        items={TAB_ITEMS}
        hiddenKeys={visibility.hiddenTabs}
        onChange={(key, visible) => {
          write({
            ...visibility,
            hiddenTabs: updateHiddenKeys(visibility.hiddenTabs, key, visible),
          });
        }}
      />
    );
  }

  return (
    <VisibilitySwitchGroup
      ariaLabel="配置信息中显示的 CLI"
      items={CLI_ITEMS}
      hiddenKeys={visibility.hiddenCliKeys}
      onChange={(key, visible) => {
        write({
          ...visibility,
          hiddenCliKeys: updateHiddenKeys(visibility.hiddenCliKeys, key, visible),
        });
      }}
    />
  );
}
