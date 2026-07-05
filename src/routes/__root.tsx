/**
 * TanStack Router root route.
 *
 * Required by the vite plugin — without this file the route-tree
 * generator refuses to run. Hosts the global providers
 * (QueryClient, future: Theme) and renders the matched child
 * inside `<Outlet />`.
 *
 * Tasks 7+8 originally provisioned a fuller AppShell with sidebar +
 * title bar + status bar. That component is not yet checked in,
 * but the Overview (Task 9) renders fine inside this bare root;
 * the shell lands in a follow-up without blocking the dashboard.
 */

import { Outlet, createRootRouteWithContext } from "@tanstack/react-router";
import type { QueryClient } from "@tanstack/react-query";

export interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootShell,
});

function RootShell(): JSX.Element {
  return (
    <div className="min-h-screen bg-[#0a0a0a] text-[#f5f5f5] font-sans">
      <Outlet />
    </div>
  );
}
