import { cn } from "@/lib/cn";
import type { HTMLMotionProps } from "motion/react";
import { forwardRef } from "react";
import { GlassPanel, type GlassPanelProps } from "./GlassPanel";

export type GlassCardSize = "sm" | "md" | "lg";

export interface GlassCardProps
  extends Omit<GlassPanelProps, "ref"> {
  /**
   * Inset padding. `sm` = p-4, `md` (default) = p-5, `lg` = p-6.
   * Use this instead of `className="p-X"` so card padding stays uniform.
   */
  size?: GlassCardSize;
}

const SIZE_PADDING: Record<GlassCardSize, string> = {
  sm: "p-4",
  md: "p-5",
  lg: "p-6",
};

/**
 * <GlassCard> = <GlassPanel> + responsive padding.
 * Use for content cards (session cards, KPI tiles, list items).
 * For chrome / dialogs / sidebars with no padding by default, use
 * <GlassPanel> directly.
 */
export const GlassCard = forwardRef<HTMLDivElement, GlassCardProps>(
  ({ size = "md", className, ...rest }, _ref) => {
    return (
      <GlassPanel
        ref={_ref}
        className={cn(SIZE_PADDING[size], className)}
        {...(rest as HTMLMotionProps<"div">)}
      />
    );
  },
);

GlassCard.displayName = "GlassCard";
