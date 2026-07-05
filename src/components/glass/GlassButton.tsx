import { cn } from "@/lib/cn";
import {
  motion,
  type HTMLMotionProps,
  type Transition,
} from "motion/react";
import { forwardRef } from "react";

export type GlassButtonVariant = "primary" | "ghost" | "outline";
export type GlassButtonSize = "sm" | "md" | "lg";

export interface GlassButtonProps
  extends Omit<HTMLMotionProps<"button">, "ref"> {
  variant?: GlassButtonVariant;
  size?: GlassButtonSize;
  /** Stretch to fill parent width. */
  fullWidth?: boolean;
}

const SIZE_STYLES: Record<GlassButtonSize, string> = {
  sm: "h-8 px-3 text-xs rounded-lg gap-1.5",
  md: "h-10 px-4 text-sm rounded-xl gap-2",
  lg: "h-12 px-5 text-base rounded-2xl gap-2.5",
};

const VARIANT_STYLES: Record<GlassButtonVariant, string> = {
  primary:
    "bg-[#ef4444] text-white border border-[#ef4444] shadow-[0_8px_32px_rgba(239,68,68,0.35)] hover:bg-[#dc2626] hover:border-[#dc2626] disabled:opacity-50 disabled:cursor-not-allowed",
  ghost:
    "bg-transparent text-white/85 border border-transparent hover:bg-white/[0.05] disabled:opacity-50 disabled:cursor-not-allowed",
  outline:
    "bg-white/[0.04] text-white/90 border border-white/10 backdrop-blur-md hover:bg-white/[0.07] hover:border-white/20 disabled:opacity-50 disabled:cursor-not-allowed",
};

/**
 * Spring transition for hover/tap scale — snappy default per the
 * Task 2 spec ("1.0 ↔ 0.97, spring transition").
 */
const SPRING: Transition = {
  type: "spring",
  stiffness: 380,
  damping: 28,
  mass: 0.7,
};

/**
 * Primary / ghost / outline button built on a glass surface.
 * Uses motion's whileHover / whileTap for the 1.0 ↔ 0.97 scale;
 * the spring transition keeps it feeling premium without being
 * bouncy past the 200ms motion budget.
 *
 * Disabled state intentionally suppresses the scale animation by
 * passing `animate`/`whileHover` conditionally — simpler than
 * fighting motion's pointer-events.
 */
export const GlassButton = forwardRef<HTMLButtonElement, GlassButtonProps>(
  (
    {
      variant = "primary",
      size = "md",
      fullWidth = false,
      className,
      disabled,
      type,
      ...rest
    },
    ref,
  ) => {
    return (
      <motion.button
        ref={ref}
        type={type ?? "button"}
        disabled={disabled}
        whileHover={disabled ? undefined : { scale: 1.0 }}
        whileTap={disabled ? undefined : { scale: 0.97 }}
        transition={SPRING}
        className={cn(
          "inline-flex items-center justify-center font-medium select-none",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#ef4444]/60 focus-visible:ring-offset-2 focus-visible:ring-offset-[#0a0a0a]",
          "transition-colors",
          SIZE_STYLES[size],
          VARIANT_STYLES[variant],
          fullWidth && "w-full",
          className,
        )}
        {...rest}
      />
    );
  },
);

GlassButton.displayName = "GlassButton";
