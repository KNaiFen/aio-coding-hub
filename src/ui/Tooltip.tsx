import { cloneElement, isValidElement } from "react";
import { cn } from "../utils/cn";
import {
  Tooltip as TooltipRoot,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
  type TooltipSurface,
} from "@/ui/shadcn/tooltip";

export type TooltipProps = {
  content: React.ReactNode;
  children: React.ReactNode;
  className?: string;
  contentClassName?: string;
  placement?: "top" | "bottom";
  surface?: TooltipSurface;
  interactive?: boolean;
  collisionPadding?: number;
};

export function Tooltip({
  content,
  children,
  className,
  contentClassName,
  placement = "top",
  surface = "inverse",
  interactive = false,
  collisionPadding,
}: TooltipProps) {
  type ClassNameProp = { className?: string };
  const trigger = isValidElement<ClassNameProp>(children) ? (
    cloneElement(children, {
      className: cn(children.props.className, className),
    })
  ) : (
    // children 不是元素时包一层，保证 TooltipTrigger 能工作
    <span className={cn("inline-flex", className)}>{children}</span>
  );

  return (
    <TooltipProvider delayDuration={0}>
      <TooltipRoot disableHoverableContent={!interactive}>
        <TooltipTrigger asChild>{trigger}</TooltipTrigger>
        <TooltipContent
          side={placement}
          surface={surface}
          collisionPadding={collisionPadding}
          className={contentClassName}
        >
          {content}
        </TooltipContent>
      </TooltipRoot>
    </TooltipProvider>
  );
}
