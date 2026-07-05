/**
 * Cost analytics page (Task 13).
 *
 * Date-range-aware roll-up of cost: stacked area by source, top
 * expensive sessions table, naive linear forecast.
 */

import { useMemo, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { motion } from "motion/react";
import { GlassCard, GlassPill } from "@/components/glass";
import { CostBreakdown } from "@/components/charts/CostBreakdown";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";
import { formatCost } from "@/lib/normalize/format";

const RANGES = [
  { label: "24h", seconds: 86400 },
  { label: "7d", seconds: 7 * 86400 },
  { label: "30d", seconds: 30 * 86400 },
] as const;

export const Route = createFileRoute("/cost")({
  component: CostPage,
});

function CostPage() {
  const [rangeIdx, setRangeIdx] = useState(1); // default: 7d
  const since = useMemo(
    () => Math.floor(Date.now() / 1000) - RANGES[rangeIdx].seconds,
    [rangeIdx],
  );

  const statsQuery = useQuery({
    queryKey: keys.dashboard(since),
    queryFn: () => api.dashboardStats(since),
    refetchInterval: 60_000,
  });

  const sessionsQuery = useQuery({
    queryKey: keys.sessions.list({ since }),
    queryFn: () => api.listSessions({ since }, 100, 0),
    refetchInterval: 60_000,
  });

  const sessions = sessionsQuery.data ?? [];
  const top = useMemo(
    () =>
      [...sessions]
        .sort((a, b) => b.cost_usd - a.cost_usd)
        .slice(0, 10),
    [sessions],
  );

  // Naive forecast: average of last 7 daily buckets × remaining days in month.
  const forecast = useMemo(() => {
    const daysInRange = RANGES[rangeIdx].seconds / 86400;
    if (daysInRange <= 0) return 0;
    const total = statsQuery.data?.total_cost_usd ?? 0;
    const perDay = total / daysInRange;
    const remaining = Math.max(0, 30 - daysInRange);
    return perDay * remaining + total;
  }, [statsQuery.data, rangeIdx]);

  return (
    <div className="p-8 space-y-6">
      <header className="flex items-baseline justify-between">
        <div>
          <h1 className="text-2xl font-medium tracking-tight text-white">Cost</h1>
          <p className="mt-1 text-sm text-white/50">
            Spend across all agents in the selected window.
          </p>
        </div>
        <div className="flex gap-1">
          {RANGES.map((r, i) => (
            <button key={r.label} onClick={() => setRangeIdx(i)}>
              <GlassPill tone={i === rangeIdx ? "accent" : "neutral"}>
                {r.label}
              </GlassPill>
            </button>
          ))}
        </div>
      </header>

      {/* KPIs */}
      <section className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <KpiBig
          label="Spent this window"
          value={formatCost(statsQuery.data?.total_cost_usd)}
          loading={statsQuery.isPending}
        />
        <KpiBig
          label="Sessions"
          value={String(sessions.length)}
          loading={sessionsQuery.isPending}
        />
        <KpiBig
          label="Forecast (30d)"
          value={formatCost(forecast)}
          hint="naive linear extrapolation"
          loading={statsQuery.isPending}
        />
      </section>

      {/* Breakdown chart */}
      <GlassCard className="p-5">
        <h2 className="text-sm font-medium text-white/70 mb-3">
          Cost by source
        </h2>
        <div className="h-56" style={{ overflow: "visible" }}>
          <CostBreakdown sessions={sessions} />
        </div>
      </GlassCard>

      {/* Top sessions table */}
      <GlassCard className="p-5">
        <h2 className="text-sm font-medium text-white/70 mb-3">
          Top 10 most expensive sessions
        </h2>
        <div className="divide-y divide-white/5">
          {top.length === 0 ? (
            <p className="text-sm text-white/40 py-8 text-center">
              No sessions in this window.
            </p>
          ) : (
            top.map((s, i) => (
              <motion.div
                key={s.id}
                initial={{ opacity: 0, x: -8 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: i * 0.02 }}
                className="grid grid-cols-[2rem_1fr_auto_auto_auto] items-center gap-3 py-2.5 text-sm"
              >
                <span className="text-white/30 tabular-nums">
                  {String(i + 1).padStart(2, "0")}
                </span>
                <span className="text-white/90 truncate">
                  {s.title ?? s.id}
                </span>
                <GlassPill tone="neutral">{s.source_id}</GlassPill>
                <span className="text-white/50 text-xs font-mono">
                  {s.model ?? "—"}
                </span>
                <span className="text-white tabular-nums font-medium">
                  {formatCost(s.cost_usd)}
                </span>
              </motion.div>
            ))
          )}
        </div>
      </GlassCard>
    </div>
  );
}

function KpiBig({
  label,
  value,
  hint,
  loading,
}: {
  label: string;
  value: string;
  hint?: string;
  loading?: boolean;
}) {
  return (
    <GlassCard className="p-5">
      <div className="text-xs text-white/50 uppercase tracking-wide">
        {label}
      </div>
      <div
        className={`mt-2 text-3xl font-medium tabular-nums text-white ${loading ? "animate-pulse" : ""}`}
      >
        {value}
      </div>
      {hint && (
        <div className="mt-1 text-xs text-white/40">{hint}</div>
      )}
    </GlassCard>
  );
}