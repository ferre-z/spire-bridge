/**
 * Session detail page — header + cost sparkline + virtualised timeline.
 */

import { useEffect } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Clock, Coins, MessageSquare, Cpu } from "lucide-react";
import { GlassCard, GlassPill } from "@/components/glass";
import { SessionTimeline } from "@/components/timeline/SessionTimeline";
import { SessionCostChart } from "@/components/charts/SessionCostChart";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";
import { formatCost, formatDuration, formatRelative } from "@/lib/normalize/format";

export const Route = createFileRoute("/sessions/$sessionId")({
  component: SessionDetail,
});

function SessionDetail() {
  const { sessionId } = Route.useParams();

  const query = useQuery({
    queryKey: keys.sessions.detail(sessionId),
    queryFn: () => api.getSession(sessionId),
  });

  useEffect(() => {
    document.title = query.data?.session.title ?? `Session ${sessionId}`;
  }, [query.data, sessionId]);

  if (query.isPending) {
    return (
      <div className="p-8">
        <div className="glass rounded-2xl p-6 h-32 animate-pulse" />
      </div>
    );
  }

  if (query.isError || !query.data) {
    return (
      <div className="p-8">
        <GlassCard className="p-6">
          <p className="text-white/60">
            Failed to load session: {String(query.error)}
          </p>
        </GlassCard>
      </div>
    );
  }

  const { session, events } = query.data;
  const duration = session.ended_at
    ? session.ended_at - session.started_at
    : Date.now() / 1000 - session.started_at;

  return (
    <div className="p-8 space-y-6">
      {/* Header */}
      <GlassCard className="p-6">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <GlassPill tone="accent">{session.source_id}</GlassPill>
              {session.model && (
                <GlassPill tone="neutral">
                  <Cpu className="size-3" />
                  <span className="font-mono">{session.model}</span>
                </GlassPill>
              )}
            </div>
            <h1 className="text-2xl font-medium text-white truncate">
              {session.title ?? session.id}
            </h1>
            <p className="mt-1 text-sm text-white/40 font-mono truncate">
              {session.cwd ?? session.project_dir ?? session.id}
            </p>
          </div>

          <div className="text-right shrink-0">
            <div className="text-3xl font-medium tabular-nums text-white">
              {formatCost(session.cost_usd)}
            </div>
            <div className="text-xs text-white/40 mt-1">
              {formatRelative(session.started_at, Date.now() / 1000)}
            </div>
          </div>
        </div>

        <div className="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
          <Stat icon={<Clock className="size-3" />} label="Duration" value={formatDuration(duration)} />
          <Stat icon={<MessageSquare className="size-3" />} label="Messages" value={String(session.message_count)} />
          <Stat
            icon={<Coins className="size-3" />}
            label="Tokens"
            value={(session.input_tokens + session.output_tokens).toLocaleString()}
          />
          <Stat
            icon={<Cpu className="size-3" />}
            label="Tool calls"
            value={String(session.tool_call_count)}
          />
        </div>
      </GlassCard>

      {/* Cost curve */}
      <GlassCard className="p-5">
        <h2 className="text-sm font-medium text-white/70 mb-3">Cost over time</h2>
        <div className="h-32" style={{ overflow: "visible" }}>
          <SessionCostChart events={events} />
        </div>
      </GlassCard>

      {/* Timeline */}
      <GlassCard className="p-5">
        <h2 className="text-sm font-medium text-white/70 mb-3">
          Timeline · {events.length} events
        </h2>
        <SessionTimeline events={events} session={session} />
      </GlassCard>
    </div>
  );
}

function Stat({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div>
      <div className="flex items-center gap-1 text-xs text-white/40 uppercase tracking-wide">
        {icon}
        {label}
      </div>
      <div className="mt-1 tabular-nums text-white/90">{value}</div>
    </div>
  );
}