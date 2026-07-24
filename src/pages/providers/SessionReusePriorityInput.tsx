import { useEffect, useState } from "react";
import { MAX_SESSION_REUSE_PRIORITY } from "../../services/providers/providers";
import { Input } from "../../ui/Input";

type SessionReusePriorityInputProps = {
  value: number;
  providerLabel: string;
  disabled: boolean;
  onCommit: (sessionReusePriority: number) => void;
};

export function SessionReusePriorityInput({
  value,
  providerLabel,
  disabled,
  onCommit,
}: SessionReusePriorityInputProps) {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  function commit() {
    const nextValue = Number(draft);
    if (
      !Number.isSafeInteger(nextValue) ||
      nextValue < 0 ||
      nextValue > MAX_SESSION_REUSE_PRIORITY
    ) {
      setDraft(String(value));
      return;
    }
    if (nextValue !== value) {
      onCommit(nextValue);
    }
  }

  return (
    <label
      className="flex shrink-0 items-center gap-1"
      onPointerDown={(event) => event.stopPropagation()}
      title="会话复用优先级：数值越大越优先"
    >
      <span className="text-[10px] text-muted-foreground">复用</span>
      <Input
        type="number"
        min={0}
        max={MAX_SESSION_REUSE_PRIORITY}
        step={1}
        inputMode="numeric"
        value={draft}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.currentTarget.blur();
          }
          if (event.key === "Escape") {
            setDraft(String(value));
            event.currentTarget.blur();
          }
        }}
        disabled={disabled}
        aria-label={`${providerLabel} 的会话复用优先级`}
        className="h-7 w-16 px-1.5 text-center text-xs"
      />
    </label>
  );
}
