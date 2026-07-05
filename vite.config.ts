import { defineConfig } from "vite";
import tailwind from "@tailwindcss/vite";
import { TanStackRouterVite } from "@tanstack/router-vite-plugin";
import path from "node:path";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    // TanStack Router MUST come before JSX transformation plugins —
    // it manipulates JSX in route files to add createFileRoute() calls.
    TanStackRouterVite({ target: "react", autoCodeSplitting: true }),
    react(),
    tailwind(),
  ],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: "esnext",
    sourcemap: true,
  },
});