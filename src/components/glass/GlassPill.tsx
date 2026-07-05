import { cn } from "@/lib/cn";
import { forwardRef, type HTMLAttributes, type ReactNode } from "react";

export type GlassPillTone =
  | "neutral"
  | "success"
  | "warning"
  | "error"
  | "info"
  | "accent";

export interface GlassPillProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, "children"> {
  /** Visual tone — controls bg / text / border color. */
  tone?: GlassPillTone;
  /** Extra classes to merge on the pill. */
  className?: string;
  children: ReactNode;
}

/**
 * Tone → className table. Each entry is a triple (bg / text / border)
 * using locked color tokens from globals.css.
 *
 * The TONE_STYLES object MUST stay exhaustive over `GlassPillTone`
 * — the `Record<GlassPillTone, string>` enforces that at compile time.
 */
const TONE_STYLES: Record<GlassPillTone, string> = {
  neutral: "bg-white/[0.06] text-white/80 border-white/10",
  success: "bg-emerald-500/15 text-emerald-300 border-emerald-400/30",
  warning: "bg-yellow-500/15 text-yellow-300 border-yellow-400/30",
  error: "bg-red-500/15 text-red-300 border-red-400/30",
  info: "bg-blue-500/15 text-blue-300 border-blue-400/30",
  accent: "bg-red-500/20 text-red-200 border-red-400/40",
};

/**
 * Small inline chip — used for status badges, role tags, cost numbers,
 * "LIVE" indicators, etc. Radius is locked to 9999px (pill).
 *
 * Any extra HTML span props (data-*, aria-*, onClick, etc.) forward
 * to the underlying element so the pill composes in forms, tooltips,
 * and screen-reader-friendly contexts.
 */
export const GlassPill = forwardRef<HTMLSpanElement, GlassPillProps>(
  ({ tone = "neutral", className, children, ...rest }, ref) => {
    return (
      <span
        ref={ref}
        className={cn(
          "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium backdrop-blur-md",
          TONE_STYLES[tone],
          className,
        )}
        {...rest}
      >
        {children}
      </span>
    );
  },
);

GlassPill.displayName = "GlassPill";
