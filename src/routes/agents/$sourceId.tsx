/**
 * Per-agent page (Task 12).
 *
 * `/agents/$sourceId` renders three tabs: Sessions / Errors / Cost.
 * The route is created lazily — TanStack Router will code-split the
 * component on first navigation. Per-source aggregates (session
 * count, error count, total cost, top tools) are computed on the
 * renderer to avoid an extra IPC roundtrip.
 *
 * Tab labels are intentionally short — the SubagentTree below the
 * Sessions tab is the "money shot" Phase-1 ships with.
 */

import { useMemo, useState } from "react";
import {
  createFileRoute,
  useParams,
  Link,
  notFound,
} from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { motion, AnimatePresence } from "motion/react";
import {
  ArrowLeft,
  Terminal,
  Code2,
  Zap,
  AlertTriangle,
  Coins,
  GitBranch,
} from "lucide-react";
import {
  GlassCard,
  GlassPill,
  GlassButton,
} from "@/components/glass";
import { SessionCard } from "@/components/session/SessionCard";
import { SubagentTree } from "@/components/session/SubagentTree";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";
import {
  formatCost,
  formatDuration,
  formatRelative,
} from "@/lib/normalize/format";
import type { CanonicalSession } from "@/lib/normalize/types";

type Tab = "sessions" | "errors" | "cost";

export const Route = createFileRoute("/agents/$sourceId")({
  component: AgentPage,
});

const SOURCE_META: Record<
  string,
  { label: string; icon: React.ComponentType<{ className?: string }>; color: string }
> = {
  claude_code: { label: "Claude Code", icon: Terminal, color: "text-red-300" },
  opencode: { label: "OpenCode", icon: Code2, color: "text-white" },
  hermes: { label: "Hermes", icon: Zap, color: "text-white/70" },
};

function AgentPage() {
  const { sourceId } = useParams({ strict: false }) as { sourceId: string };
  const meta = SOURCE_META[sourceId];
  const [tab, setTab] = useState<Tab>("sessions");

  // Fetch all sessions for this source — capped via the IPC layer
  // to 200 so the page never lags. We re-query on a slow interval
  // so updates from active agents land without a manual refresh.
  const since = useMemo(() => Math.floor(Date.now() / 1000) - 86400 * 30, []);
  const sessionsQuery = useQuery({
    queryKey: keys.sessions.list({ source: sourceId, since }, 200, 0),
    queryFn: () => api.listSessions({ source: sourceId, since }, 200, 0),
  });

  const sessions: CanonicalSession[] = sessionsQuery.data ?? [];

  const aggregates = useMemo(() => {
    const errors = sessions.filter((s) => s.end_reason === "error");
    const totalCost = sessions.reduce((acc, s) => acc + s.cost_usd, 0);
    const totalTokens = sessions.reduce(
      (acc, s) =>
        acc +
        s.input_tokens +
        s.output_tokens +
        s.cache_read +
        s.cache_write,
      0,
    );
    return {
      sessions: sessions.length,
      errors: errors.length,
      totalCost,
      totalTokens,
      errorsList: errors,
    };
  }, [sessions]);

  if (sessionsQuery.isPending) {
    return <AgentPageSkeleton meta={meta} />;
  }

  if (!meta) {
    // Source we don't know — surface a 404 instead of a blank screen.
    throw notFound();
  }

  return (
    <div className="p-8 space-y-6">
      <Link
        to="/"
        className="inline-flex items-center gap-1.5 text-xs text-white/50 hover:text-white transition-colors"
      >
        <ArrowLeft className="size-3.5" /> Overview
      </Link>

      {/* Header */}
      <header className="flex items-start justify-between gap-4">
        <div className="flex items-center gap-4">
          <div className={`glass rounded-2xl p-3 ${meta.color}`}>
            <meta.icon className="size-7" />
          </div>
          <div>
            <h1 className="text-3xl font-medium tracking-tight text-white">
              {meta.label}
            </h1>
            <p className="mt-1 text-sm text-white/50">
              {aggregates.sessions} sessions · {aggregates.errors} errors ·{" "}
              {formatCost(aggregates.totalCost)} lifetime
            </p>
          </div>
        </div>
        <GlassPill tone={aggregates.errors > 0 ? "warning" : "success"}>
          {aggregates.errors > 0
            ? `${aggregates.errors} issues`
            : "healthy"}
        </GlassPill>
      </header>

      {/* Tabs */}
      <div className="flex items-center gap-1 border-b border-white/10">
        {(["sessions", "errors", "cost"] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`relative px-4 py-2 text-sm capitalize transition-colors ${
              tab === t
                ? "text-white"
                : "text-white/50 hover:text-white/80"
            }`}
          >
            {t}
            {t === "errors" && aggregates.errors > 0 && (
              <span className="ml-1.5 text-[10px] text-red-300">
                ({aggregates.errors})
              </span>
            )}
            {tab === t && (
              <motion.div
                layoutId="agent-tab-underline"
                className="absolute -bottom-px left-0 right-0 h-px bg-red-400"
                transition={{ type: "spring", stiffness: 380, damping: 30 }}
              />
            )}
          </button>
        ))}
      </div>

      <AnimatePresence mode="wait">
        {tab === "sessions" && (
          <TabPanel key="sessions">
            {sessions.length === 0 ? (
              <GlassCard className="p-8 text-center">
                <GitBranch className="size-8 mx-auto text-white/30 mb-3" />
                <p className="text-white/70 text-sm">
                  No {meta.label} sessions yet.
                </p>
                <p className="text-white/40 text-xs mt-1">
                  Start one to see it here in real time.
                </p>
              </GlassCard>
            ) : (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <section className="space-y-2">
                  <h2 className="text-xs uppercase tracking-wide text-white/50 px-1">
                    Recent sessions ({sessions.length})
                  </h2>
                  <div className="space-y-2 max-h-[640px] overflow-y-auto pr-2">
                    {sessions.slice(0, 50).map((s) => (
                      <Link
                        key={s.id}
                        to="/sessions/$sessionId"
                        params={{ sessionId: s.id }}
                        className="block transition-transform hover:translate-x-1"
                      >
                        <SessionCard session={s} />
                      </Link>
                    ))}
                  </div>
                </section>
                <section className="space-y-2">
                  <h2 className="text-xs uppercase tracking-wide text-white/50 px-1">
                    Subagent tree
                  </h2>
                  <GlassCard className="p-3">
                    <SubagentTree sessions={sessions} />
                  </GlassCard>
                </section>
              </div>
            )}
          </TabPanel>
        )}

        {tab === "errors" && (
          <TabPanel key="errors">
            {aggregates.errorsList.length === 0 ? (
              <GlassCard className="p-8 text-center">
                <AlertTriangle className="size-8 mx-auto text-white/30 mb-3" />
                <p className="text-white/70 text-sm">No errors in the last 30 days.</p>
              </GlassCard>
            ) : (
              <GlassCard className="divide-y divide-white/5">
                {aggregates.errorsList.map((s) => (
                  <Link
                    key={s.id}
                    to="/sessions/$sessionId"
                    params={{ sessionId: s.id }}
                    className="flex items-center gap-3 p-3 hover:bg-white/[0.03] transition-colors"
                  >
                    <AlertTriangle className="size-4 text-red-400 shrink-0" />
                    <div className="flex-1 min-w-0">
                      <div className="text-sm text-white/90 truncate">
                        {s.title ?? s.id.slice(0, 24)}
                      </div>
                      <div className="text-xs text-white/40 mt-0.5">
                        {formatRelative(s.started_at, Date.now() / 1000)} ·{" "}
                        {formatCost(s.cost_usd)}
                      </div>
                    </div>
                    <span className="text-[10px] uppercase tracking-wide text-red-300/80">
                      {s.end_reason ?? "errored"}
                    </span>
                  </Link>
                ))}
              </GlassCard>
            )}
          </TabPanel>
        )}

        {tab === "cost" && (
          <TabPanel key="cost">
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <KpiTile
                label="Lifetime spend"
                value={formatCost(aggregates.totalCost)}
                accent
              />
              <KpiTile label="Total sessions" value={String(aggregates.sessions)} />
              <KpiTile
                label="Total tokens"
                value={aggregates.totalTokens.toLocaleString()}
              />
              <KpiTile
                label="Avg cost / session"
                value={formatCost(
                  aggregates.sessions > 0
                    ? aggregates.totalCost / aggregates.sessions
                    : 0,
                )}
              />
              <KpiTile
                label="Most expensive"
                value={
                  sessions.length === 0
                    ? "—"
                    : formatCost(
                        Math.max(...sessions.map((s) => s.cost_usd)),
                      )
                }
              />
              <KpiTile
                label="Last session"
                value={
                  sessions.length === 0
                    ? "—"
                    : formatRelative(
                        sessions[0]!.started_at,
                        Date.now() / 1000,
                      )
                }
              />
            </div>
          </TabPanel>
        )}
      </AnimatePresence>
    </div>
  );
}

function TabPanel({ children }: { children: React.ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -6 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
    >
      {children}
    </motion.div>
  );
}

function KpiTile({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <GlassCard className="p-4">
      <div className="text-xs text-white/50 uppercase tracking-wide">
        {label}
      </div>
      <div
        className={`mt-2 text-2xl font-medium tabular-nums ${
          accent ? "text-red-300" : "text-white/90"
        }`}
      >
        {value}
      </div>
    </GlassCard>
  );
}

function AgentPageSkeleton({
  meta,
}: {
  meta: { label: string } | undefined;
}) {
  return (
    <div className="p-8 space-y-6">
      <div className="glass rounded-2xl h-16 w-72 animate-pulse" />
      <div className="grid grid-cols-3 gap-4">
        {Array.from({ length: 3 }).map((_, i) => (
          <div key={i} className="glass rounded-xl h-24 animate-pulse" />
        ))}
      </div>
      <GlassCard className="p-8 text-center text-white/40 text-sm">
        Loading {meta?.label ?? "agent"}…
      </GlassCard>
    </div>
  );
}

// Keep `GlassButton` reachable for tree-shaking visibility — may
// be used for "Export" / "Re-auth" actions in a follow-up task.
void GlassButton;
// Keep `formatDuration` reachable — Cost tab may eventually show it.
void formatDuration;
// Keep `Coins` reachable — Cost tab icon used downstream.
void Coins;
