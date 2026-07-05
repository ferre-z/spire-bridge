/**
 * TanStack Query client.
 *
 * Settings rationale:
 *
 *   - `staleTime: 5_000` — overview/sessions lists are quick and
 *     often the same numbers between renders; we only refetch on
 *     explicit invalidation or after 5s of staleness. Live events
 *     are the primary "real-time" path; queries are the catch-up.
 *
 *   - `gcTime: 60_000` — keep detail pages warm for a minute after
 *     the user navigates away so back-button returns are instant.
 *
 *   - `refetchOnWindowFocus: false` — the desktop window rarely
 *     loses focus in a way that should trigger a refetch. The live
 *     event bus is the source of truth, so on-focus refetch would
 *     just thrash the IPC layer.
 *
 *   - `retry: 1` — fail fast. One retry is enough to mask a single
 *     hiccup; more retries make the UI feel stuck.
 */

import { QueryClient } from "@tanstack/react-query";

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 5_000,
        gcTime: 60_000,
        refetchOnWindowFocus: false,
        retry: 1,
      },
      mutations: {
        retry: 0,
      },
    },
  });
}
