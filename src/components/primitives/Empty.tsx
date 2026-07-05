/**
 * Empty state placeholder — used when lists have zero items.
 */

import { Inbox, AlertCircle, Ghost } from "lucide-react";
import { GlassCard } from "@/components/glass";

export function Empty({
  icon = "inbox",
  title,
  hint,
}: {
  icon?: "inbox" | "alert" | "ghost";
  title: string;
  hint?: string;
}) {
  const Icon = icon === "alert" ? AlertCircle : icon === "ghost" ? Ghost : Inbox;
  return (
    <GlassCard className="flex flex-col items-center justify-center py-12 px-6 text-center">
      <Icon className="size-8 text-white/20" />
      <h3 className="mt-3 text-sm font-medium text-white/70">{title}</h3>
      {hint && <p className="mt-1 text-xs text-white/40 max-w-xs">{hint}</p>}
    </GlassCard>
  );
}