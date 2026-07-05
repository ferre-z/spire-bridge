/**
 * Glass-style loading skeleton with shimmer animation.
 */

import { cn } from "@/lib/cn";

export function Skeleton({
  variant = "text-line",
  className,
}: {
  variant?: "text-line" | "card" | "chart";
  className?: string;
}) {
  const base = "glass animate-pulse";
  const variantClass =
    variant === "text-line"
      ? "h-3 rounded"
      : variant === "chart"
        ? "rounded-xl"
        : "rounded-2xl";

  return (
    <div
      className={cn(
        base,
        variantClass,
        variant === "text-line" && "w-full",
        variant === "chart" && "h-48",
        variant === "card" && "h-32",
        className,
      )}
      style={{
        background:
          "linear-gradient(90deg, rgba(255,255,255,0.04), rgba(255,255,255,0.08), rgba(255,255,255,0.04))",
        backgroundSize: "200% 100%",
        animation: "shimmer 1.5s ease-in-out infinite",
      }}
    />
  );
}

/**
 * Inline CSS keyframes. Imported once from globals; duplicated here
 * for component-local scoping so consumers don't need extra wiring.
 */
export const shimmerKeyframes = `
@keyframes shimmer {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}`;