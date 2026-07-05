/**
 * Overview dashboard (Task 9).
 *
 * Composes KPIs, charts, live stream, and recent sessions.
 * Every number comes from TanStack Query against the typed `api`
 * client (Task 7). Charts use Recharts with `ResponsiveContainer`
 * wrappers + `style={{ overflow: 'visible' }}` to prevent clipping
 * (see AGENTS.md pitfalls).
 */

import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Activity, Zap, AlertTriangle, DollarSign } from "lucide-react";
import { motion } from "motion/react";
import { GlassCard, GlassPill } from "@/components/glass";
import { LiveStream } from "@/components/live/LiveStream";
import { CostSparkline } from "@/components/charts/CostSparkline";
import { ActivityHeatmap } from "@/components/charts/ActivityHeatmap";
import { SessionCard } from "@/components/session/SessionCard";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";
import { formatCost, formatTokens } from "@/lib/normalize/format";

export const Route = createFileRoute("/")({
  component: Overview,
});

function Overview() {
  // Last 24h window for "today" metrics.
  const since = Math.floor(Date.now() / 1000) - 86400;

  const statsQuery = useQuery({
    queryKey: keys.dashboard(since),
    queryFn: () => api.dashboardStats(since),
    refetchInterval: 30_000,
  });

  const sessionsQuery = useQuery({
    queryKey: keys.sessions.list({ since }),
    queryFn: () => api.listSessions({ since }, 20, 0),
    refetchInterval: 15_000,
  });

  const stats = statsQuery.data;
  const sessions = sessionsQuery.data ?? [];

  const avgCost =
    stats && stats.session_count > 0
      ? stats.total_cost_usd / stats.session_count
      : 0;

  return (
    <div className="p-8 space-y-6">
      <header className="flex items-baseline justify-between">
        <div>
          <h1 className="text-3xl font-medium tracking-tight text-white">
            Spire Bridge
          </h1>
          <p className="mt-1 text-sm text-white/50">
            Your AI agents, in one cockpit.
          </p>
        </div>
        <GlassPill tone="accent">
          <Activity className="size-3" /> Live
        </GlassPill>
      </header>

      {/* KPI row */}
      <section className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <KpiCard
          icon={<DollarSign className="size-4" />}
          label="Today's spend"
          value={formatCost(stats?.total_cost_usd)}
          tone="accent"
          loading={statsQuery.isPending}
        />
        <KpiCard
          icon={<Zap className="size-4" />}
          label="Sessions"
          value={String(stats?.session_count ?? 0)}
          loading={statsQuery.isPending}
        />
        <KpiCard
          icon={<AlertTriangle className="size-4" />}
          label="Errors"
          value={String(stats?.error_count ?? 0)}
          tone={stats && stats.error_count > 0 ? "warning" : "neutral"}
          loading={statsQuery.isPending}
        />
        <KpiCard
          icon={<Activity className="size-4" />}
          label="Avg cost / session"
          value={formatCost(avgCost)}
          loading={statsQuery.isPending}
        />
      </section>

      {/* Charts row */}
      <section className="grid grid-cols-1 lg:grid-cols-5 gap-4">
        <GlassCard className="lg:col-span-3 p-5">
          <h2 className="text-sm font-medium text-white/70 mb-4">
            Cost (last 24h)
          </h2>
          <div className="h-48" style={{ overflow: "visible" }}>
            <CostSparkline />
          </div>
        </GlassCard>
        <GlassCard className="lg:col-span-2 p-5">
          <h2 className="text-sm font-medium text-white/70 mb-4">Activity</h2>
          <ActivityHeatmap />
        </GlassCard>
      </section>

      {/* Live + sessions */}
      <section className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <GlassCard className="p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3">
            Live stream
          </h2>
          <LiveStream />
        </GlassCard>
        <GlassCard className="p-5">
          <h2 className="text-sm font-medium text-white/70 mb-3">
            Recent sessions
          </h2>
          <div className="space-y-2 max-h-96 overflow-y-auto">
            {sessionsQuery.isPending ? (
              Array.from({ length: 4 }).map((_, i) => (
                <div
                  key={i}
                  className="glass rounded-xl p-3 h-20 animate-pulse"
                />
              ))
            ) : sessions.length === 0 ? (
              <p className="text-sm text-white/40 py-8 text-center">
                No sessions in the last 24h. Start an agent run to see it
                here.
              </p>
            ) : (
              sessions.slice(0, 8).map((s) => (
                <SessionCard key={s.id} session={s} compact />
              ))
            )}
          </div>
        </GlassCard>
      </section>
    </div>
  );
}

function KpiCard({
  icon,
  label,
  value,
  tone = "neutral",
  loading,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  tone?: "neutral" | "accent" | "warning" | "success";
  loading?: boolean;
}) {
  const toneClass =
    tone === "accent"
      ? "text-red-300"
      : tone === "warning"
        ? "text-yellow-300"
        : tone === "success"
          ? "text-emerald-300"
          : "text-white/90";

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
    >
      <GlassCard className="p-4">
        <div className="flex items-center gap-2 text-xs text-white/50 uppercase tracking-wide">
          <span className="text-white/60">{icon}</span>
          {label}
        </div>
        <div
          className={`mt-2 text-2xl font-medium tabular-nums ${toneClass} ${loading ? "animate-pulse" : ""}`}
        >
          {value}
        </div>
      </GlassCard>
    </motion.div>
  );
}

// Helper to keep formatTokens reachable for tree-shaking visibility.
void formatTokens;