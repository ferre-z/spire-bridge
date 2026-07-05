/**
 * TanStack Query key factory.
 *
 * Centralising keys in one module gives us three guarantees:
 *
 *   1. **No typos.** Every consumer imports a constant, so a renamed
 *      key fails TypeScript instead of silently desyncing the cache.
 *
 *   2. **Hierarchical invalidation.** TanStack Query invalidates
 *      every key that *starts with* a given prefix, so `keys.sessions.all`
 *      invalidates `keys.sessions.list(...)`, `keys.sessions.detail(...)`,
 *      and any future per-source variant in one call.
 *
 *   3. **Stable reference identity.** Factory functions build a new
 *      array per call (so different filter values land in different
 *      cache slots), but the `keys` object itself never changes
 *      — safe to put in dep arrays / contexts.
 *
 * Convention: every entry is a tuple. The first element is the
 * domain ("sessions"), the second the operation ("list", "detail",
 * "dashboard"), the rest are the inputs that distinguish cache slots.
 */

import type { SessionFilter } from "@/lib/normalize/types";

export const keys = {
  sessions: {
    all: ["sessions"] as const,
    list: (filter: SessionFilter, limit: number, offset: number) =>
      ["sessions", "list", { filter, limit, offset }] as const,
    detail: (id: string) => ["sessions", "detail", id] as const,
  },
  dashboard: {
    all: ["dashboard"] as const,
    stats: (since: number) => ["dashboard", "stats", since] as const,
  },
  settings: {
    all: ["settings"] as const,
    current: () => ["settings", "current"] as const,
  },
  live: {
    buffer: () => ["live", "buffer"] as const,
  },
} as const;

/** Type alias so callers can write `QueryKey = typeof keys.X.Y` if needed. */
export type SessionsListKey = ReturnType<typeof keys.sessions.list>;
export type SessionsDetailKey = ReturnType<typeof keys.sessions.detail>;
export type DashboardStatsKey = ReturnType<typeof keys.dashboard.stats>;
export type SettingsKey = ReturnType<typeof keys.settings.current>;
