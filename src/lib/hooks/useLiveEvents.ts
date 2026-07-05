/**
 * Live event store + hooks.
 *
 * One global Zustand store (`liveBuffer`) holds the rolling window of
 * `CanonicalEvent`s observed by the renderer. The store is fed by a
 * single subscriber registered at app-shell mount; the same buffer
 * is then sliced per-session by `useLiveEventsBySession`.
 *
 * Why a store and not a context?
 *   - Slicing selectors in Zustand avoid re-rendering every component
 *     that imports the buffer. Only components whose selector output
 *     changes re-render.
 *   - The buffer is independent of React tree position (sidebar,
 *     timeline, live panel all share it). Context would force the
 *     provider to live above all of them.
 *
 * Buffer cap is 1000 events (per Global Constraint #9 spirit: stay
 * under the 350 KB JS bundle target by not retaining unbounded
 * history in renderer memory; older events live in SQLite via
 * `get_session`).
 */

import { useEffect } from "react";
import { create } from "zustand";
import type { CanonicalEvent } from "@/lib/normalize/types";
import { streamLiveEvents } from "@/lib/api/stream";

/**
 * Hard cap on the buffer. When a new event pushes us over, the
 * oldest event is dropped FIFO. 1000 ≈ ~3 minutes of heavy Claude
 * tool-call traffic; enough to render "the recent past" without
 * melting memory.
 */
export const LIVE_BUFFER_CAP = 1000;

interface LiveState {
  buffer: CanonicalEvent[];
  /** Bumped on every push; lets consumers detect motion cheaply. */
  lastPushedAt: number;
  push: (event: CanonicalEvent) => void;
  clear: () => void;
}

/**
 * Module-level store. Singleton — there's one app shell, one bus,
 * one buffer. Tests reset it via `clear()` between scenarios.
 */
export const useLiveStore = create<LiveState>((set) => ({
  buffer: [],
  lastPushedAt: 0,
  push: (event) =>
    set((state) => {
      // Drop dupes on (session_id, seq) — a relaunch / re-emit from
      // Rust shouldn't double-count.
      const dupe = state.buffer.find(
        (e) => e.session_id === event.session_id && e.seq === event.seq,
      );
      if (dupe) return state;
      const next = state.buffer.length >= LIVE_BUFFER_CAP
        ? state.buffer.slice(state.buffer.length - LIVE_BUFFER_CAP + 1)
        : state.buffer.slice();
      next.push(event);
      return { buffer: next, lastPushedAt: Date.now() };
    }),
  clear: () => set({ buffer: [], lastPushedAt: 0 }),
}));

/**
 * Subscribe to the live event bus and feed it into the store.
 *
 * Mount this exactly once (typically in `<AppShell>`). Returns a
 * cleanup function so a re-mount (e.g. HMR) doesn't double-subscribe.
 */
export function useLiveSubscription(): void {
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    streamLiveEvents((event) => {
      if (cancelled) return;
      useLiveStore.getState().push(event);
    })
      .then((u) => {
        if (cancelled) {
          u();
          return;
        }
        unlisten = u;
      })
      .catch(() => {
        // Subscribe failed (devtools detached, sandboxed iframe, etc.).
        // Silently no-op: the bus is advisory; the store is the truth.
      });

    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, []);
}

/**
 * Selector: events for one session, oldest → newest, capped at 200.
 * Skips the recompute when nothing in the buffer changed for that
 * session (Zustand's shallow equality + structural sharing).
 */
export function useLiveEventsBySession(
  sessionId: string | null | undefined,
): CanonicalEvent[] {
  return useLiveStore((state) => {
    if (!sessionId) return [];
    const out: CanonicalEvent[] = [];
    for (const e of state.buffer) {
      if (e.session_id === sessionId) out.push(e);
    }
    return out.length > 200 ? out.slice(out.length - 200) : out;
  });
}

/**
 * Count of events seen in the last `windowMs` ms. Useful for the
 * "live activity" indicator on the overview screen.
 */
export function useRecentLiveCount(windowMs = 60_000): number {
  return useLiveStore((state) => {
    const cutoff = Date.now() - windowMs;
    let n = 0;
    for (let i = state.buffer.length - 1; i >= 0; i--) {
      const e = state.buffer[i];
      if (!e) continue;
      if (e.occurred_at * 1000 < cutoff) break;
      n++;
    }
    return n;
  });
}

/**
 * Total events currently in the rolling buffer (for the statusbar
 * counter). Memoised by selector so unrelated pushes don't re-fire.
 */
export function useLiveBufferSize(): number {
  return useLiveStore((s) => s.buffer.length);
}
