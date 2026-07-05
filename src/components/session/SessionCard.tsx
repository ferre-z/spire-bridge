/**
 * Compact session card — title, source, model, started, cost.
 */

import { motion } from "motion/react";
import { Clock, Coins, MessageSquare } from "lucide-react";
import { GlassPill } from "@/components/glass";
import { formatCost, formatDuration, formatRelative } from "@/lib/normalize/format";
import type { CanonicalSession } from "@/lib/normalize/types";

export function SessionCard({
  session,
  compact = false,
}: {
  session: CanonicalSession;
  compact?: boolean;
}) {
  const startRel = formatRelative(session.started_at, Date.now() / 1000);

  return (
    <motion.div
      whileHover={{ x: 2 }}
      transition={{ duration: 0.15 }}
      className={`glass rounded-xl p-3 ${compact ? "" : "p-4"} flex flex-col gap-1.5`}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="font-medium text-white/95 truncate text-sm">
          {session.title ?? session.id}
        </span>
        <GlassPill tone="neutral" className="shrink-0">
          {session.source_id}
        </GlassPill>
      </div>

      <div className="flex items-center gap-3 text-xs text-white/50">
        <span className="flex items-center gap-1">
          <Clock className="size-3" />
          {startRel}
        </span>
        {session.model && (
          <span className="truncate font-mono">{session.model}</span>
        )}
      </div>

      <div className="flex items-center gap-3 text-xs text-white/60">
        <span className="flex items-center gap-1">
          <Coins className="size-3" />
          {formatCost(session.cost_usd)}
        </span>
        <span className="flex items-center gap-1">
          <MessageSquare className="size-3" />
          {session.message_count}
        </span>
        {session.ended_at && (
          <span>{formatDuration(session.ended_at - session.started_at)}</span>
        )}
      </div>
    </motion.div>
  );
}