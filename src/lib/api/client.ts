/**
 * Typed wrappers around Tauri IPC commands.
 *
 * Each function delegates to `@tauri-apps/api/core::invoke` with a
 * literal command name. Type signatures mirror the Rust handlers in
 * `src-tauri/src/ipc/commands.rs` and the structs in
 * `src-tauri/src/ipc/mod.rs`.
 *
 * Two responsibilities:
 *
 *   1. Hide the `"snake_case"` argument-name mismatch between Rust
 *      and JS. `invoke<T>("list_sessions", { filter, limit, offset })`
 *      works because Tauri serialises both sides identically, but
 *      keeping this surface small means future renames in Rust only
 *      require changes here, not in every component.
 *
 *   2. Return narrow, well-typed `Promise<T>`s so React Query
 *      callers never need `unknown` casts.
 *
 * Tests: see `tests/ts/api.test.ts` (mocks `@tauri-apps/api/core`).
 */

import { invoke } from "@tauri-apps/api/core";
import type {
  CanonicalSession,
  CanonicalEvent,
  DashboardStats,
  SessionDetail,
  SessionFilter,
  Settings,
} from "@/lib/normalize/types";

/**
 * One module-level `api` object. Using a plain object (not a class)
 * keeps React Query's `queryFn: () => api.listSessions(...)` calls
 * referentially stable across re-renders, which matters for
 * dependency arrays.
 */
export const api = {
  /**
   * List sessions, newest first. `filter.source` is the canonical
   * way to scope to one agent; `since` / `until` are Unix seconds.
   */
  listSessions: (
    filter: SessionFilter = {},
    limit = 50,
    offset = 0,
  ): Promise<CanonicalSession[]> =>
    invoke<CanonicalSession[]>("list_sessions", { filter, limit, offset }),

  /**
   * Fetch one session and its events. Throws on IPC failure; the
   * React layer wraps this in `useQuery` so a missing session
   * surfaces as `query.error` rather than an unhandled rejection.
   */
  getSession: (id: string): Promise<SessionDetail> =>
    invoke<SessionDetail>("get_session", { id }),

  /**
   * Aggregate roll-up for the overview dashboard. `since` is Unix
   * seconds; the caller typically passes `Date.now()/1000 - 86400`.
   */
  dashboardStats: (since: number): Promise<DashboardStats> =>
    invoke<DashboardStats>("dashboard_stats", { since }),

  /**
   * Read all settings (mostly: which secrets are set).
   * Backend expects an explicit empty arg object so the IPC layer
   * surfaces a typed `Settings` rather than a positional nothing.
   */
  getSettings: (): Promise<Settings> => invoke<Settings>("get_settings", {}),

  /**
   * Store the Hermes basic-auth password in the OS keychain.
   * Returns nothing on success.
   */
  setHermesPassword: (password: string): Promise<void> =>
    invoke<void>("set_hermes_password", { password }),
} as const;

export type Api = typeof api;

// --- Live event type re-export ------------------------------------
// The renderer subscribes to events via the `listen` helper, which
// doesn't go through `invoke`. We re-export the type here so callers
// can `import { type CanonicalEvent, streamLiveEvents } from "@/lib/api"`.
export type { CanonicalEvent };
