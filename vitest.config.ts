/// Vitest config — kept separate from vite.config.ts so the two
/// `defineConfig` types (Vite vs Vitest/UserConfig) don't conflict
/// during pnpm's tsc typecheck pass.
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./tests/setup.ts"],
  },
});
