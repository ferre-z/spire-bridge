/**
 * Helpers for the live event bus.
 *
 * The Rust core publishes `CanonicalEvent`s to every subscriber via
 * `tauri::Window::emit("live_event", event)`. The renderer subscribes
 * with `listen<CanonicalEvent>("live_event", cb)` and gets a typed
 * payload directly — no JSON parsing on the hot path.
 *
 * Two surfaces live here:
 *
 *   - `streamLiveEvents(onEvent)` — low-level: returns the unlisten
 *     function so callers can wire it into React `useEffect`
 *     cleanup. Used by `useLiveEvents` and tests.
 *
 *   - `useLiveEvents()` — high-level: subscribes once at the app
 *     shell level, pushes every event into a Zustand store, exposes
 *     selectors. See `src/lib/hooks/useLiveEvents.ts`.
 *
 * Both surfaces are designed so the renderer can drop and re-attach
 * the listener without affecting the upstream broadcast (Tauri's
 * event bus is independent of subscriber count).
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CanonicalEvent } from "@/lib/normalize/types";

/** Event name the Rust core emits on. Keep in sync with `events.rs`. */
export const LIVE_EVENT_NAME = "live_event" as const;

/**
 * Subscribe to live events. The returned `Promise<UnlistenFn>` is
 * the disposable — call it to detach the listener (typically from
 * a React effect's cleanup).
 *
 * `onEvent` runs once per published event. Throws from `onEvent`
 * are swallowed by Tauri (so a single bad event can't kill the
 * subscriber); if you need error handling, wrap the body in
 * try/catch yourself.
 */
export function streamLiveEvents(
  onEvent: (event: CanonicalEvent) => void,
): Promise<UnlistenFn> {
  return listen<CanonicalEvent>(LIVE_EVENT_NAME, (msg) => {
    onEvent(msg.payload);
  });
}
