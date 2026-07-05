import { cn } from "@/lib/cn";
import { motion, type HTMLMotionProps } from "motion/react";
import { forwardRef } from "react";

export interface GlassPanelProps
  extends Omit<HTMLMotionProps<"div">, "ref"> {
  /** Use the stronger blur (40px / 160% saturate) for elevated surfaces. */
  strong?: boolean;
  /** Adds hover affordance + pointer cursor. For clickable panels. */
  interactive?: boolean;
}

/**
 * Foundational glass surface — transparent panel with backdrop blur,
 * light border, and soft inset highlight. This is the atom; higher-level
 * compositions (cards, dialogs, sidebars) wrap it.
 *
 * Built on `motion.div` so children get free layout / spring
 * animations when this component is unmounted or re-keyed.
 */
export const GlassPanel = forwardRef<HTMLDivElement, GlassPanelProps>(
  ({ strong = false, interactive = false, className, ...rest }, ref) => {
    return (
      <motion.div
        ref={ref}
        className={cn(
          "glass",
          strong && "glass-strong",
          interactive && "glass-hover cursor-pointer",
          className,
        )}
        {...rest}
      />
    );
  },
);

GlassPanel.displayName = "GlassPanel";
