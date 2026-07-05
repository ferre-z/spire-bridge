/**
 * Sessions list page — filter chips + debounced search + paginated table.
 */

import { useMemo, useState } from "react";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useInfiniteQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { GlassCard, GlassPill } from "@/components/glass";
import { SessionCard } from "@/components/session/SessionCard";
import { api } from "@/lib/api/client";
import { keys } from "@/lib/query/keys";

const SOURCES = ["claude", "opencode", "hermes"] as const;
const PAGE_SIZE = 50;

export const Route = createFileRoute("/sessions/")({
  component: SessionsList,
});

function SessionsList() {
  const navigate = useNavigate();
  const [activeSources, setActiveSources] = useState<Set<string>>(
    new Set(SOURCES),
  );
  const [search, setSearch] = useState("");

  const filter = useMemo(
    () => ({
      source: activeSources.size === 1 ? Array.from(activeSources)[0] : undefined,
    }),
    [activeSources],
  );

  const query = useInfiniteQuery({
    queryKey: keys.sessions.list(filter),
    queryFn: ({ pageParam = 0 }) => api.listSessions(filter, PAGE_SIZE, pageParam),
    initialPageParam: 0,
    getNextPageParam: (last, _, lastPageParam) =>
      last.length < PAGE_SIZE ? undefined : (lastPageParam as number) + PAGE_SIZE,
  });

  const sessions = query.data?.pages.flat() ?? [];
  const filtered = search
    ? sessions.filter((s) =>
        (s.title ?? s.id).toLowerCase().includes(search.toLowerCase()),
      )
    : sessions;

  return (
    <div className="p-8 space-y-4">
      <header>
        <h1 className="text-2xl font-medium tracking-tight text-white">Sessions</h1>
        <p className="mt-1 text-sm text-white/50">
          {query.data?.pages.reduce((acc, p) => acc + p.length, 0) ?? 0} total
        </p>
      </header>

      {/* Filters */}
      <GlassCard className="p-3 flex items-center gap-3">
        <div className="flex items-center gap-1.5">
          {SOURCES.map((s) => (
            <button
              key={s}
              onClick={() => {
                setActiveSources((prev) => {
                  const next = new Set(prev);
                  if (next.has(s)) next.delete(s);
                  else next.add(s);
                  return next;
                });
              }}
              className="transition-opacity"
              style={{ opacity: activeSources.has(s) ? 1 : 0.4 }}
            >
              <GlassPill
                tone={activeSources.has(s) ? "accent" : "neutral"}
              >
                {s}
              </GlassPill>
            </button>
          ))}
        </div>

        <div className="flex-1 relative">
          <Search className="size-4 absolute left-3 top-1/2 -translate-y-1/2 text-white/40" />
          <input
            type="search"
            placeholder="Search sessions…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full glass rounded-lg pl-9 pr-3 py-1.5 text-sm bg-white/[0.04] border-white/10 focus:border-red-400/40 focus:outline-none transition-colors"
          />
        </div>
      </GlassCard>

      {/* List */}
      <div className="space-y-2">
        {query.isPending ? (
          Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="glass rounded-xl p-4 h-20 animate-pulse" />
          ))
        ) : filtered.length === 0 ? (
          <p className="text-sm text-white/40 py-12 text-center">
            No sessions match the current filters.
          </p>
        ) : (
          filtered.map((s) => (
            <button
              key={s.id}
              onClick={() => navigate({ to: "/sessions/$sessionId", params: { sessionId: s.id } })}
              className="block w-full text-left transition-transform hover:translate-x-1"
            >
              <SessionCard session={s} />
            </button>
          ))
        )}
      </div>

      {query.hasNextPage && (
        <div className="flex justify-center pt-2">
          <button
            onClick={() => query.fetchNextPage()}
            disabled={query.isFetchingNextPage}
            className="glass rounded-lg px-4 py-1.5 text-sm hover:bg-white/[0.06] disabled:opacity-40 transition-colors"
          >
            {query.isFetchingNextPage ? "Loading…" : "Load more"}
          </button>
        </div>
      )}
    </div>
  );
}