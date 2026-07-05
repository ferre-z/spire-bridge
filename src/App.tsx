/**
 * Top-level app component.
 *
 * Wires up the global providers (TanStack QueryClient, live-event
 * subscriber) and instantiates the TanStack Router with our route
 * tree. The Overview (Task 9) renders directly inside the root
 * route — the AppShell with sidebar / title bar / status bar was
 * the responsibility of Task 8 (already marked complete in the
 * ledger) but its components weren't checked in. The shell lands
 * in a separate task; meanwhile the Overview composition fills
 * the viewport so all of Tasks 9-11 are exercisable.
 */

import { useMemo } from "react";
import {
  QueryClientProvider,
  type QueryClient,
} from "@tanstack/react-query";
import {
  RouterProvider,
  createRouter,
  type RouterContext,
} from "@tanstack/react-router";
import { createQueryClient } from "@/lib/query/client";
import { useLiveSubscription } from "@/lib/hooks/useLiveEvents";

import { Route as RootRoute } from "@/routes/__root";

/**
 * Pre-create the router at module scope using the exported `Route`
 * constant. TanStack Router requires a single `Router` instance;
 * the QueryClient is injected via `RouterContext` so route
 * loaders can query through the same client as the React tree.
 *
 * The cast keeps `Route` typed as `AnyRoute` even though it's the
 * concrete root route — the router only reads `useRouterContext`
 * off it, which is structurally available on any Route.
 */
function buildRouter(queryClient: QueryClient) {
  const ctx: RouterContext = { queryClient };
  const router = createRouter({
    routeTree: RootRoute,
    context: ctx,
    defaultPreload: "intent",
  });
  return router;
}

export const App = (): JSX.Element => {
  const queryClient = useMemo(() => createQueryClient(), []);
  const router = useMemo(() => buildRouter(queryClient), [queryClient]);

  // Subscribe to the live event bus. Mounted once at app boot per
  // `useLiveSubscription` contract. No UI side effects — the store
  // is the only thing it touches.
  useLiveSubscription();

  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
};
