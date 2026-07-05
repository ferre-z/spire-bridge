/**
 * Top-level app component.
 *
 * Task 1 (bootstrap) owns the real router shell — that agent will overwrite
 * this file once it lands. Until then, this minimal placeholder lets the
 * renderer boot so the Task 2 styles + glass primitives are exercisable
 * in dev and so `pnpm build` succeeds.
 */
export const App = (): JSX.Element => {
  return (
    <main className="min-h-screen bg-[#0a0a0a] text-[#f5f5f5] font-sans flex items-center justify-center">
      <h1 className="text-2xl font-medium tracking-tight">Spire Bridge</h1>
    </main>
  );
};
