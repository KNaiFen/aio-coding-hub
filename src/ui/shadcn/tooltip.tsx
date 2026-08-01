import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import { cn } from "@/ui/shadcn/utils";

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export type TooltipSurface = "inverse" | "panel";

type TooltipContentProps = React.ComponentPropsWithRef<typeof TooltipPrimitive.Content> & {
  surface?: TooltipSurface;
};

export function TooltipContent({
  className,
  sideOffset = 8,
  surface = "inverse",
  ref,
  ...props
}: TooltipContentProps) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        ref={ref}
        sideOffset={sideOffset}
        className={cn(
          "z-50 max-w-[280px] whitespace-normal rounded-lg text-xs leading-snug shadow-panel outline-none",
          surface === "panel"
            ? "border border-border bg-popover px-3 py-2 text-popover-foreground"
            : "bg-foreground px-2 py-1 text-background",
          className
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}
