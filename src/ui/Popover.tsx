import { useCallback, useEffect, useState, type ReactNode } from "react";
import { cn } from "../utils/cn";
import { Popover as PopoverRoot, PopoverContent, PopoverTrigger } from "@/ui/shadcn/popover";

export type PopoverProps = {
  trigger: ReactNode;
  children: ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  placement?: "top" | "bottom";
  align?: "start" | "center" | "end";
  className?: string;
  contentClassName?: string;
  portalled?: boolean;
  disabled?: boolean;
};

export function Popover({
  trigger,
  children,
  open: controlledOpen,
  onOpenChange,
  placement = "bottom",
  align = "end",
  className,
  contentClassName,
  portalled = true,
  disabled = false,
}: PopoverProps) {
  const [internalOpen, setInternalOpen] = useState(false);

  const isControlled = controlledOpen !== undefined;
  const open = isControlled ? controlledOpen : internalOpen;

  const setOpen = useCallback(
    (next: boolean) => {
      if (disabled && next) return;
      if (!isControlled) setInternalOpen(next);
      onOpenChange?.(next);
    },
    [disabled, isControlled, onOpenChange]
  );

  useEffect(() => {
    if (!disabled || !open) return;
    if (!isControlled) setInternalOpen(false);
    onOpenChange?.(false);
  }, [disabled, isControlled, onOpenChange, open]);

  return (
    <PopoverRoot open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <button type="button" className={cn("inline-flex", className)} disabled={disabled}>
          {trigger}
        </button>
      </PopoverTrigger>
      <PopoverContent
        side={placement}
        align={align}
        className={contentClassName}
        portalled={portalled}
      >
        {children}
      </PopoverContent>
    </PopoverRoot>
  );
}
