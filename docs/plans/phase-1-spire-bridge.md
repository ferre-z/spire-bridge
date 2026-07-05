# Spire Bridge — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a premium, real-time, **liquid-glass** desktop cockpit that unifies live data from Claude Code, OpenCode, and Hermes Agent into a single beautiful dashboard — designed to scale to 5+ hosts, 20+ agents, and 200+ users (telemetry-only path).

**Architecture:** Tauri 2 (Rust core + system webview) hosts a React 19 renderer. Three pluggable source adapters (Claude OTel/jsonl, OpenCode HTTP+SSE, Hermes HTTP+WS) feed a normalized event store (SQLite). Live SSE/WS bridges surface events to the renderer as a single typed stream. UI is glassmorphic, black/white with red accents, motion-driven but performance-bounded.

**Tech Stack (pinned):**

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2.1+** | <600 KB binary, system WebView, Rust SSE/file-watcher perf |
| Renderer | **React 19 + TypeScript 5.7 strict** | Concurrent rendering, use() hook |
| Build | **Vite 6** | Fast HMR, tree-shaking |
| Routing | **TanStack Router v1** (file-based, type-safe) | Typed params, code-splitting |
| Server state | **TanStack Query v5** | Caching, optimistic updates, streaming |
| Client state | **Zustand 5** | Tiny, no boilerplate |
| UI primitives | **Radix Themes** + custom glass layer | Accessible, themable |
| Styling | **Tailwind CSS 4** + CSS custom properties | Speed, theming, glass tokens |
| Animations | **Motion (formerly Framer Motion) v12** | Best React DX, `layout` animations |
| Virtualization | **react-virtuoso** | Variable-height, dynamic lists |
| Charts | **Recharts 2.x** | Practical default, good enough perf |
| Code blocks | **shiki** (build-time) + **react-syntax-highlighter** (runtime small) | Premium look |
| Forms | **react-hook-form + zod** | Typed forms, validation |
| Backend | **Rust** (Tokio, reqwest, rusqlite bundled, eventsource-client, notify) | Async, fast, embedded |
| SQL | **rusqlite** (bundled feature) + handwritten migrations | Embedded, no system dep |
| Auth | **keyring** crate (OS keychain) | Password / OAuth secret storage |
| Auto-update | **tauri-plugin-updater** | Signed updates |
| Tray | **tauri-plugin-system-tray** | Phase 1 nicety |
| Logging | **tracing** + **tracing-subscriber** | Structured logs |

---

## Global Constraints

These apply to every task. Copy-paste verbatim where it binds.

1. **Repo:** all code lives under `/home/ubuntu/spire-bridge/`; remote is `https://github.com/ferre-z/spire-bridge` (main branch).
2. **License:** MIT (matches Vision + Anthropic ecosystem norms).
3. **Node:** 20 LTS. **Rust:** 1.83 stable. **OS:** development on Linux (this box); CI matrix later (Win/macOS).
4. **TypeScript:** `strict: true`, `noUncheckedIndexedAccess: true`, `exactOptionalPropertyTypes: true`. No `any` on boundary types; `unknown` only at parse sites.
5. **Rust:** `clippy::all = deny` in CI, no `unwrap()` outside tests, all public APIs documented.
6. **No network calls to third-party analytics, telemetry, or crash-reporting services.** Period. Privacy is the product.
7. **All agent integration endpoints default to `127.0.0.1` only.** No `0.0.0.0` binding on Hermes/OpenCode listeners.
8. **Visual design tokens** — locked, not configurable per-component:
   - **Background:** `#0a0a0a` (near-black, 95% L*)
   - **Surface:** `rgba(255, 255, 255, 0.04)` with `backdrop-filter: blur(24px) saturate(140%)`
   - **Border:** `rgba(255, 255, 255, 0.08)`
   - **Text primary:** `#f5f5f5`, **text muted:** `#a3a3a3`, **text faint:** `#525252`
   - **Accent (red):** `#ef4444` (action), `#dc2626` (hover), `#fca5a5` (highlight)
   - **Semantic:** success `#22c55e`, warning `#eab308`, error `#ef4444`, info `#3b82f6`
   - **Radius:** 12 px standard, 20 px cards, 9999 px pills
   - **Type scale:** Inter Variable (UI), JetBrains Mono Variable (code)
   - **Shadow:** `0 8px 32px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(255,255,255,0.04) inset`
9. **Motion budget:** all animations ≤ 200 ms unless explicitly a "reveal" (≤ 600 ms). Respect `prefers-reduced-motion`.
10. **Bundle size target:** renderer initial JS ≤ 350 KB gzipped. Rust binary ≤ 15 MB stripped.
11. **Performance targets:**
    - Cold start ≤ 800 ms (warm launch)
    - Session list (200 sessions) renders ≤ 16 ms / frame
    - Timeline (10k events) scrolls at 60 fps with virtualization
    - Live event round-trip: SSE → render ≤ 100 ms p95
12. **Schema migrations:** every DB change is a new versioned file in `src-tauri/migrations/NNNN_*.sql`; never edit old migrations.
13. **Commits:** Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `style:`). One logical change per commit.
14. **Testing:** each task ends with `cargo test --workspace` + `pnpm test` both green.
15. **Source data is read-only.** v1 observes, never steers. No `POST /session/:id/abort`, no permission responses, no prompt injection.
16. **No third-party API key transmission.** All tokens stay local; OS keychain only.
17. **No subscription, no auth walls, no rate limits, no phone-home.** Open-source from day one.

---

## File Structure

```
spire-bridge/
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── tailwind.config.ts
├── postcss.config.js
├── index.html
├── .gitignore
├── .editorconfig
├── .nvmrc
├── README.md
├── LICENSE                              # MIT
├── docs/
│   └── plans/
│       ├── phase-1-spire-bridge.md      # this file
│       └── phase-2-*.md                 # later
├── src-tauri/
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── tauri.conf.json
│   ├── build.rs
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/                            # generated by `tauri icon`
│   ├── migrations/
│   │   ├── 0001_init.sql
│   │   └── 0002_seed_sources.sql
│   └── src/
│       ├── main.rs                       # entry, plugin wiring, tray, app boot
│       ├── lib.rs                        # Tauri builder, command registration
│       ├── error.rs                      # AppError + Result alias
│       ├── store/
│       │   ├── mod.rs                    # pool + migrations runner
│       │   ├── schema.rs                 # sqlx-style query helpers
│       │   └── redact.rs                 # secret redaction (api keys, tokens)
│       ├── sources/
│       │   ├── mod.rs                    # trait Source + SourceRegistry
│       │   ├── claude/
│       │   │   ├── mod.rs                # ClaudeSource
│       │   │   ├── otel.rs               # OTLP HTTP receiver (port 4318)
│       │   │   ├── jsonl.rs              # ~/.claude/projects tailer
│       │   │   └── normalize.rs          # → CanonicalEvent
│       │   ├── opencode/
│       │   │   ├── mod.rs                # OpenCodeSource
│       │   │   ├── http.rs               # /session /messages fetcher
│       │   │   ├── sse.rs                # /event subscriber
│       │   │   └── normalize.rs
│       │   └── hermes/
│       │       ├── mod.rs                # HermesSource
│       │       ├── http.rs               # /api/sessions, /api/profiles/sessions
│       │       ├── ws.rs                 # /api/events websocket
│       │       └── normalize.rs
│       ├── sync/
│       │   ├── mod.rs                    # SyncEngine: orchestrator, fan-out
│       │   ├── live.rs                   # live event broadcast
│       │   └── backfill.rs               # initial history load
│       ├── ipc/
│       │   ├── mod.rs                    # command registry
│       │   ├── sessions.rs               # list_sessions, get_session
│       │   ├── events.rs                 # stream_live_events (Channel<T>)
│       │   ├── stats.rs                  # dashboard aggregates
│       │   └── settings.rs               # get/set settings, keyring bridge
│       └── secrets.rs                    # keyring wrapper
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles/
│   │   ├── globals.css                   # tailwind + glass tokens + base layer
│   │   └── glass.css                     # reusable glass utility classes
│   ├── routes/                           # TanStack Router file-based
│   │   ├── __root.tsx
│   │   ├── index.tsx                     # Overview
│   │   ├── agents/
│   │   │   ├── $sourceId.tsx
│   │   ├── sessions/
│   │   │   ├── index.tsx
│   │   │   └── $sessionId.tsx
│   │   ├── hosts.tsx                     # multi-host placeholder (Phase 2 wired)
│   │   ├── cost.tsx
│   │   └── settings.tsx
│   ├── components/
│   │   ├── glass/
│   │   │   ├── GlassPanel.tsx
│   │   │   ├── GlassCard.tsx
│   │   │   ├── GlassPill.tsx
│   │   │   └── GlassButton.tsx
│   │   ├── shell/
│   │   │   ├── AppShell.tsx              # sidebar + main + statusbar
│   │   │   ├── Sidebar.tsx
│   │   │   ├── TitleBar.tsx              # custom macOS-style traffic-light area
│   │   │   └── StatusBar.tsx
│   │   ├── timeline/
│   │   │   ├── SessionTimeline.tsx       # virtuoso
│   │   │   ├── EventRow.tsx
│   │   │   ├── ToolCallRow.tsx
│   │   │   ├── AssistantTextRow.tsx
│   │   │   ├── UserPromptRow.tsx
│   │   │   └── CodeBlock.tsx             # shiki renderer
│   │   ├── charts/
│   │   │   ├── CostSparkline.tsx
│   │   │   ├── TokenFlow.tsx
│   │   │   └── ActivityHeatmap.tsx
│   │   ├── live/
│   │   │   ├── LiveStream.tsx            # subscribes to IPC channel
│   │   │   └── LiveIndicator.tsx         # pulsing dot animation
│   │   ├── session/
│   │   │   ├── SessionCard.tsx
│   │   │   ├── SessionList.tsx
│   │   │   └── SessionHeader.tsx
│   │   └── primitives/
│   │       ├── Icon.tsx                  # lucide-react wrapper
│   │       ├── Skeleton.tsx
│   │       └── Empty.tsx
│   ├── lib/
│   │   ├── api/
│   │   │   ├── client.ts                 # typed invoke wrappers
│   │   │   └── stream.ts                 # listen<Event> helper
│   │   ├── query/
│   │   │   ├── keys.ts                   # query key factory
│   │   │   └── client.ts                 # QueryClient setup
│   │   ├── normalize/
│   │   │   ├── types.ts                  # CanonicalSession, CanonicalEvent
│   │   │   └── format.ts                 # duration, cost, token formatters
│   │   ├── store/
│   │   │   ├── ui.ts                     # zustand: theme, sidebar, layout
│   │   │   └── selection.ts              # zustand: selected session/filters
│   │   ├── motion/
│   │   │   ├── variants.ts               # shared motion variants
│   │   │   └── useReducedMotion.ts       # safe wrapper
│   │   └── hooks/
│   │       ├── useLiveEvents.ts
│   │       └── useShortcuts.ts
│   ├── routesTree.gen.ts                 # generated by TanStack Router plugin
│   └── vite-env.d.ts
└── tests/
    ├── rust/
    │   ├── sources/
    │   │   ├── claude_normalize.rs
    │   │   ├── opencode_normalize.rs
    │   │   └── hermes_normalize.rs
    │   ├── store.rs
    │   └── redact.rs
    └── ts/
        ├── normalize.test.ts
        ├── format.test.ts
        └── timeline.test.tsx
```

---

## Task 1: Bootstrap repo, Tauri scaffold, and dev loop

**Files:**
- Create: `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/main.tsx`, `src/App.tsx`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `src-tauri/capabilities/default.json`, `src-tauri/icons/.gitkeep`, `.gitignore`, `.nvmrc`, `.editorconfig`, `LICENSE`

**Step 1.1 — Initialize package.json + tooling**

Pin Node 20, pnpm 9, Vitest 2.x.

Run: `cd /home/ubuntu/spire-bridge && pnpm init`
Then write `package.json` with deps:
- runtime: `react@^19.0.0`, `react-dom@^19.0.0`, `@tanstack/react-router@^1.95.0`, `@tanstack/react-query@^5.59.0`, `zustand@^5.0.0`, `motion@^12.0.0`, `react-virtuoso@^4.12.0`, `recharts@^2.13.0`, `@radix-ui/themes@^3.1.0`, `@radix-ui/react-icons@^1.3.0`, `lucide-react@^0.460.0`, `react-hook-form@^7.53.0`, `zod@^3.23.0`, `clsx@^2.1.0`, `tailwind-merge@^2.5.0`, `date-fns@^4.1.0`, `shiki@^1.22.0`, `react-markdown@^9.0.0`, `remark-gfm@^4.0.0`
- dev: `typescript@^5.7.0`, `vite@^6.0.0`, `@vitejs/plugin-react@^4.3.0`, `tailwindcss@^4.0.0`, `@tailwindcss/vite@^4.0.0`, `@tauri-apps/cli@^2.1.0`, `@tauri-apps/api@^2.1.0`, `@tauri-apps/plugin-updater@^2.0.0`, `@tanstack/router-vite-plugin@^1.95.0`, `vitest@^2.1.0`, `@testing-library/react@^16.1.0`, `@testing-library/jest-dom@^6.6.0`, `jsdom@^25.0.0`, `@types/react@^19.0.0`, `@types/react-dom@^19.0.0`

**Step 1.2 — Write `vite.config.ts`**

TanStack Router plugin + React + Tailwind v4:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
import { TanStackRouterVite } from "@tanstack/router-vite-plugin";
import path from "node:path";

export default defineConfig({
  plugins: [react(), tailwind(), TanStackRouterVite({ target: "react", autoCodeSplitting: true })],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { target: "esnext", sourcemap: true },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./tests/setup.ts"],
  },
});
```

**Step 1.3 — Write `tsconfig.json`** (strict mode + path aliases).

**Step 1.4 — Write `tauri.conf.json`**

Product name: `Spire Bridge`. Identifier: `com.spire-bridge.app`. Windows: titleBarStyle: "overlay", transparent: true. macOS: titleBarStyle: "overlay", hiddenTitle: true, windowBackgroundColor: "#0a0a0a". Security: `csp: "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"`.

**Step 1.5 — Write `src-tauri/Cargo.toml`**

Workspace deps:
- `tauri = { version = "2.1", features = ["tray-icon", "image-png"] }`
- `tauri-plugin-updater = "2"`
- `serde = { version = "1", features = ["derive"] }`
- `serde_json = "1"`
- `tokio = { version = "1", features = ["full"] }`
- `reqwest = { version = "0.12", features = ["json", "stream"] }`
- `eventsource-client = "0.13"`
- `tokio-tungstenite = { version = "0.24", features = ["connect-hyper"] }`
- `rusqlite = { version = "0.32", features = ["bundled", "chrono"] }`
- `refinery = { version = "0.8", features = ["rusqlite"] }`
- `chrono = { version = "0.4", features = ["serde"] }`
- `uuid = { version = "1", features = ["v4", "serde"] }`
- `keyring = "3"`
- `notify = "6"`
- `tracing = "0.1"`
- `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
- `thiserror = "1"`
- `anyhow = "1"`
- `parking_lot = "0.12"`
- `dashmap = "6"`

**Step 1.6 — Write minimal `src/main.rs` + `src/lib.rs`**

`lib.rs` exports `run()` that builds the Tauri app with the updater plugin + tray. `main.rs` calls `lib::run()`.

**Step 1.7 — Write minimal React entry**

`src/main.tsx` mounts `<App />` in `#root`. `src/App.tsx` renders a centered `<h1>Spire Bridge</h1>` on the dark background to verify the renderer works.

**Step 1.8 — Verify dev loop works**

Run: `cd /home/ubuntu/spire-bridge && pnpm install`
Run: `pnpm tauri dev` (background, 60 s budget)
Expected: window opens with "Spire Bridge" text, dark background. Kill the process.

**Step 1.9 — Commit + push**

```bash
git add -A
git -c user.name="ferre" -c user.email="ferre@ob-vault.local" commit -m "feat: bootstrap Tauri 2 + React 19 + TS scaffold"
git push origin main
```

**Files:** as listed in Files block.

**Interfaces:** none (foundation).

---

## Task 2: Global styles, design tokens, glass primitive

**Files:**
- Create: `src/styles/globals.css`, `src/styles/glass.css`
- Create: `src/components/glass/GlassPanel.tsx`, `GlassCard.tsx`, `GlassPill.tsx`, `GlassButton.tsx`
- Create: `src/components/primitives/Icon.tsx`, `src/lib/cn.ts`
- Create: `tests/ts/glass.test.tsx`

**Step 2.1 — Write `src/styles/globals.css`**

Tailwind v4 import + `:root` CSS custom properties for all tokens from Global Constraint #8. Body background `#0a0a0a`, text `#f5f5f5`, font-family `Inter Variable, system-ui, sans-serif`. Define `--glass-bg`, `--glass-border`, `--glass-blur`, `--glass-shadow` CSS vars. Add `prefers-reduced-motion` media query that disables all transitions.

**Step 2.2 — Write `src/styles/glass.css`**

```css
.glass {
  background: rgba(255,255,255,0.04);
  border: 1px solid rgba(255,255,255,0.08);
  backdrop-filter: blur(24px) saturate(140%);
  -webkit-backdrop-filter: blur(24px) saturate(140%);
  box-shadow: 0 8px 32px rgba(0,0,0,0.4), inset 0 0 0 1px rgba(255,255,255,0.04);
  border-radius: 12px;
}
.glass-strong {
  background: rgba(255,255,255,0.06);
  backdrop-filter: blur(40px) saturate(160%);
}
.glass-hover:hover { background: rgba(255,255,255,0.07); border-color: rgba(255,255,255,0.12); }
@media (prefers-reduced-motion: reduce) { .glass, .glass-hover { transition: none !important; } }
```

**Step 2.3 — Write `src/lib/cn.ts`**

```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
export const cn = (...inputs: ClassValue[]) => twMerge(clsx(inputs));
```

**Step 2.4 — Write `GlassPanel`**

```tsx
import { cn } from "@/lib/cn";
import { motion, type HTMLMotionProps } from "motion/react";
import { forwardRef } from "react";
type Props = HTMLMotionProps<"div"> & { strong?: boolean; interactive?: boolean };
export const GlassPanel = forwardRef<HTMLDivElement, Props>(({ strong, interactive, className, ...rest }, ref) => (
  <motion.div ref={ref} className={cn("glass", strong && "glass-strong", interactive && "glass-hover cursor-pointer", className)} {...rest} />
));
```

**Step 2.5 — Write `GlassCard`**

Wraps `GlassPanel` with padding `p-5` default, `p-6` `lg`, `p-4` `sm`.

**Step 2.6 — Write `GlassPill`**

```tsx
export const GlassPill = forwardRef<HTMLSpanElement, { tone?: "neutral" | "success" | "warning" | "error" | "info" | "accent"; className?: string; children: React.ReactNode }>(({ tone = "neutral", className, children }, ref) => {
  const tones = {
    neutral: "bg-white/[0.06] text-white/80 border-white/10",
    success: "bg-emerald-500/15 text-emerald-300 border-emerald-400/30",
    warning: "bg-yellow-500/15 text-yellow-300 border-yellow-400/30",
    error:   "bg-red-500/15 text-red-300 border-red-400/30",
    info:    "bg-blue-500/15 text-blue-300 border-blue-400/30",
    accent:  "bg-red-500/20 text-red-200 border-red-400/40",
  };
  return <span ref={ref} className={cn("inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium backdrop-blur-md", tones[tone], className)}>{children}</span>;
});
```

**Step 2.7 — Write `GlassButton`**

Variant `primary` (red accent), `ghost` (transparent), `outline`. Sizes `sm` `md` `lg`. Uses `motion.button` for hover/tap scale (1.0 ↔ 0.97), spring transition.

**Step 2.8 — Write `Icon.tsx`**

```tsx
import { icons as lucideIcons, type LucideProps } from "lucide-react";
type IconName = keyof typeof lucideIcons;
export const Icon = ({ name, ...rest }: { name: IconName } & LucideProps) => {
  const C = lucideIcons[name];
  return <C {...rest} />;
};
```

**Step 2.9 — Write `tests/ts/glass.test.tsx`**

Vitest + @testing-library/react. Render `<GlassPill tone="accent">Live</GlassPill>`. Assert it has the `bg-red-500/20` class. Render `<GlassPanel>` with `interactive` and assert the `glass-hover` class. Render `<GlassButton>` and click — assert `onClick` called.

**Step 2.10 — Verify build + tests green**

Run: `pnpm test && pnpm build`
Expected: 3+ tests pass, build succeeds.

**Step 2.11 — Commit**

```bash
git add -A
git -c user.name="ferre" -c user.email="ferre@ob-vault.local" commit -m "feat(ui): glass primitives + design tokens"
git push origin main
```

---

## Task 3: SQLite store, migrations, secret redaction

**Files:**
- Create: `src-tauri/migrations/0001_init.sql`, `src-tauri/migrations/0002_seed_sources.sql`
- Create: `src-tauri/src/error.rs`, `src-tauri/src/store/mod.rs`, `src-tauri/src/store/schema.rs`, `src-tauri/src/store/redact.rs`
- Create: `tests/rust/store.rs`, `tests/rust/redact.rs`

**Step 3.1 — Write `0001_init.sql`**

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE agent_source (
  id    TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  icon  TEXT NOT NULL,
  color TEXT NOT NULL
);

CREATE TABLE session (
  id              TEXT PRIMARY KEY,
  source_id       TEXT NOT NULL REFERENCES agent_source(id),
  native_id       TEXT NOT NULL,
  title           TEXT,
  project_dir     TEXT,
  cwd             TEXT,
  git_branch      TEXT,
  model           TEXT,
  started_at      REAL NOT NULL,
  ended_at        REAL,
  end_reason      TEXT,
  input_tokens    INTEGER NOT NULL DEFAULT 0,
  output_tokens   INTEGER NOT NULL DEFAULT 0,
  cache_read      INTEGER NOT NULL DEFAULT 0,
  cache_write     INTEGER NOT NULL DEFAULT 0,
  reasoning_tokens INTEGER NOT NULL DEFAULT 0,
  cost_usd        REAL NOT NULL DEFAULT 0,
  message_count   INTEGER NOT NULL DEFAULT 0,
  tool_call_count INTEGER NOT NULL DEFAULT 0,
  parent_session_id TEXT,
  raw_json        TEXT,
  source_path     TEXT NOT NULL DEFAULT '',
  updated_at      REAL NOT NULL DEFAULT (unixepoch()),
  UNIQUE(source_id, native_id)
);
CREATE INDEX session_started_idx ON session(started_at DESC);
CREATE INDEX session_source_idx  ON session(source_id);

CREATE TABLE event (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id       TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  seq              INTEGER NOT NULL,
  occurred_at      REAL NOT NULL,
  kind             TEXT NOT NULL,
  payload          TEXT NOT NULL,
  duration_ms      INTEGER,
  tool_name        TEXT,
  tool_input_size  INTEGER,
  tool_result_size INTEGER,
  cost_usd         REAL NOT NULL DEFAULT 0,
  tokens_in        INTEGER NOT NULL DEFAULT 0,
  tokens_out       INTEGER NOT NULL DEFAULT 0,
  model            TEXT,
  UNIQUE(session_id, seq)
);
CREATE INDEX event_session_seq_idx  ON event(session_id, seq);
CREATE INDEX event_session_time_idx ON event(session_id, occurred_at);
CREATE INDEX event_kind_idx         ON event(kind);
CREATE INDEX event_tool_idx         ON event(tool_name);

CREATE TABLE host (
  id        TEXT PRIMARY KEY,
  label     TEXT NOT NULL,
  hostname  TEXT NOT NULL,
  added_at  REAL NOT NULL DEFAULT (unixepoch())
);
```

**Step 3.2 — Write `0002_seed_sources.sql`**

```sql
INSERT INTO agent_source (id, label, icon, color) VALUES
  ('claude_code', 'Claude Code', 'terminal',  '#ef4444'),
  ('opencode',    'OpenCode',    'code',      '#f5f5f5'),
  ('hermes',      'Hermes',      'zap',       '#a3a3a3');
```

**Step 3.3 — Write `src-tauri/src/error.rs`**

```rust
use thiserror::Error;
#[derive(Debug, Error)]
pub enum AppError {
  #[error("io: {0}")] Io(#[from] std::io::Error),
  #[error("sqlite: {0}")] Sqlite(#[from] rusqlite::Error),
  #[error("http: {0}")] Http(#[from] reqwest::Error),
  #[error("json: {0}")] Json(#[from] serde_json::Error),
  #[error("not found: {0}")] NotFound(String),
  #[error("upstream: {0}")] Upstream(String),
  #[error("auth: {0}")] Auth(String),
  #[error("other: {0}")] Other(String),
}
impl serde::Serialize for AppError {
  fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&self.to_string())
  }
}
pub type AppResult<T> = Result<T, AppError>;
```

**Step 3.4 — Write `src-tauri/src/store/mod.rs`**

Open SQLite at `app_data_dir/spire.db`. Run migrations with refinery. Expose `Store` struct with `pool: Arc<Mutex<Connection>>` (rusqlite is sync — fine for v1; tokio calls into it via `spawn_blocking`).

**Step 3.5 — Write `src-tauri/src/store/schema.rs`**

Helper functions:
- `pub fn upsert_session(conn: &Connection, s: &Session) -> AppResult<()>`
- `pub fn upsert_event(conn: &Connection, e: &Event) -> AppResult<()>`
- `pub fn list_sessions(conn, filter, limit, offset) -> AppResult<Vec<Session>>`
- `pub fn get_session(conn, id) -> AppResult<Session>`
- `pub fn list_events(conn, session_id, limit, offset) -> AppResult<Vec<Event>>`
- `pub fn dashboard_stats(conn, since: f64) -> AppResult<DashboardStats>`

Use prepared statements cached in `OnceCell`.

**Step 3.6 — Write `src-tauri/src/store/redact.rs`**

```rust
use regex::Regex;
use once_cell::sync::Lazy;
static PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| vec![
  Regex::new(r"sk-[A-Za-z0-9_-]{20,}").unwrap(),
  Regex::new(r"sk-ant-[A-Za-z0-9_-]{20,}").unwrap(),
  Regex::new(r"ghp_[A-Za-z0-9]{20,}").unwrap(),
  Regex::new(r"xoxb-[A-Za-z0-9-]{20,}").unwrap(),
  Regex::new(r"(?i)authorization:\s*bearer\s+[A-Za-z0-9._-]+").unwrap(),
  Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
]);
pub fn redact(input: &str) -> String {
  let mut out = input.to_string();
  for p in PATTERNS.iter() {
    out = p.replace_all(&out, "[REDACTED]").into_owned();
  }
  out
}
```

Add `regex = "1"` to Cargo deps.

**Step 3.7 — Write `tests/rust/redact.rs`**

```rust
use spire_bridge::store::redact::redact;
#[test] fn redacts_openai_key() { assert!(redact("hello sk-abcdefghijklmnopqrstuv world").contains("[REDACTED]")); }
#[test] fn redacts_github_pat() { assert!(redact("token ghp_abcdefghijklmnopqrst").contains("[REDACTED]")); }
#[test] fn redacts_bearer_header() { assert!(redact("Authorization: Bearer abc.def-ghi_jkl").contains("[REDACTED]")); }
#[test] fn leaves_normal_text() { assert_eq!(redact("hello world"), "hello world"); }
```

**Step 3.8 — Write `tests/rust/store.rs`**

In-memory SQLite. Apply migrations. Insert 3 sessions. `list_sessions` returns 3 ordered by `started_at DESC`. Insert events with duplicate `(session_id, seq)` — second insert fails with UNIQUE. `redact` then `upsert_event` — verify stored payload is redacted.

**Step 3.9 — Verify**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass.

**Step 3.10 — Commit**

```bash
git add -A
git -c user.name="ferre" -c user.email="ferre@ob-vault.local" commit -m "feat(store): sqlite migrations + redaction"
git push origin main
```

---

## Task 4: Canonical event/session types + Source trait

**Files:**
- Create: `src-tauri/src/sources/mod.rs`
- Create: `src-tauri/src/sources/claude/mod.rs`, `otel.rs`, `jsonl.rs`, `normalize.rs`
- Create: `src-tauri/src/sources/opencode/mod.rs`, `http.rs`, `sse.rs`, `normalize.rs`
- Create: `src-tauri/src/sources/hermes/mod.rs`, `http.rs`, `ws.rs`, `normalize.rs`
- Create: `tests/rust/sources/claude_normalize.rs`, `opencode_normalize.rs`, `hermes_normalize.rs`

**Step 4.1 — Define canonical types in `sources/mod.rs`**

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalSession {
  pub id: String,
  pub source_id: String,
  pub native_id: String,
  pub title: Option<String>,
  pub project_dir: Option<String>,
  pub cwd: Option<String>,
  pub git_branch: Option<String>,
  pub model: Option<String>,
  pub started_at: f64,
  pub ended_at: Option<f64>,
  pub end_reason: Option<String>,
  pub input_tokens: i64,
  pub output_tokens: i64,
  pub cache_read: i64,
  pub cache_write: i64,
  pub reasoning_tokens: i64,
  pub cost_usd: f64,
  pub message_count: i64,
  pub tool_call_count: i64,
  pub parent_session_id: Option<String>,
  pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
  UserPrompt,
  AssistantText,
  ToolCall,
  ToolResult,
  ApiRequest,
  ApiError,
  ApiRefusal,
  Compaction,
  Auth,
  PermissionDecision,
  SubagentStart,
  SubagentEnd,
  Unknown,
}
impl EventKind { pub fn as_str(&self) -> &'static str { /* match */ } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEvent {
  pub session_id: String,
  pub seq: i64,
  pub occurred_at: f64,
  pub kind: EventKind,
  pub payload: serde_json::Value,
  pub duration_ms: Option<i64>,
  pub tool_name: Option<String>,
  pub tool_input_size: Option<i64>,
  pub tool_result_size: Option<i64>,
  pub cost_usd: f64,
  pub tokens_in: i64,
  pub tokens_out: i64,
  pub model: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
  #[error("upstream: {0}")] Upstream(String),
  #[error("decode: {0}")] Decode(String),
  #[error("io: {0}")] Io(#[from] std::io::Error),
}

#[async_trait::async_trait]
pub trait Source: Send + Sync {
  fn id(&self) -> &'static str;
  async fn health(&self) -> Result<(), SourceError>;
  async fn backfill(&self, since: Option<f64>) -> Result<Vec<(CanonicalSession, Vec<CanonicalEvent>)>, SourceError>;
  async fn live_events(&self) -> Result<tokio::sync::mpsc::Receiver<CanonicalEvent>, SourceError>;
}
```

Add `async-trait = "0.1"` dep.

**Step 4.2 — Implement Claude OTel receiver (port 4318)**

Run an axum/Tokio HTTP server on `127.0.0.1:4318` accepting OTLP/HTTP protobuf + JSON. For Phase 1, use JSON only (`Content-Type: application/json`). Decode the OTLP envelope with `prost` for protobuf (deferred — JSON first). Emit `CanonicalEvent`s through an mpsc channel.

For Phase 1, **start with the jsonl tail** path only — it's simpler, works today without protobuf deps, and Claude Code writes JSONL alongside OTel.

**Step 4.3 — Implement Claude JSONL tail**

`jsonl.rs` watches `~/.claude/projects/*/*.jsonl` with `notify`. On new lines, parse + emit.

**Step 4.4 — Implement OpenCode HTTP + SSE**

`http.rs` calls `GET /session` and `GET /session/:id/message`. `sse.rs` subscribes to `GET /event` via `eventsource-client`, emits bus events.

**Step 4.5 — Implement Hermes HTTP + WebSocket**

`http.rs` calls `GET /api/sessions?limit=N&offset=M`. `ws.rs` connects to `ws://127.0.0.1:9119/api/events` via `tokio-tungstenite`, emits JSON-RPC events.

**Step 4.6 — Write normalizer tests**

Each `tests/rust/sources/*_normalize.rs` has 3+ tests with fixture JSON (real OTel event, real OpenCode session JSON, real Hermes session JSON) → assert correct canonical conversion.

**Step 4.7 — Commit**

```bash
git add -A && git commit -m "feat(sources): claude/opencode/hermes adapters + canonical types"
git push origin main
```

---

## Task 5: Sync engine, live broadcast

**Files:**
- Create: `src-tauri/src/sync/mod.rs`, `live.rs`, `backfill.rs`

**Step 5.1 — `live.rs`**

Holds `mpsc::Sender<CanonicalEvent>`. Each source's `live_events()` receiver is `tokio::spawn`-ed and forwarded. Subscribers (renderer via IPC channel) receive a `tokio::sync::broadcast::Receiver<CanonicalEvent>` snapshot.

**Step 5.2 — `backfill.rs`**

On startup: call each source's `backfill(None)` (full history). Upsert sessions + events in batches of 100 (transactions). Last `seq` per session stored in `meta` table (new migration `0003_sync_meta.sql`).

**Step 5.3 — `sync/mod.rs`**

```rust
pub struct SyncEngine {
  pub sources: Vec<Arc<dyn Source>>,
  pub store: Arc<Store>,
  pub live: Arc<LiveHub>,
}
impl SyncEngine {
  pub async fn start(self: Arc<Self>) -> AppResult<()> {
    let s = Arc::clone(&self);
    tokio::spawn(async move { s.backfill_all().await; });
    for src in &self.sources {
      let s = Arc::clone(&self);
      let src = Arc::clone(src);
      tokio::spawn(async move { s.run_live(src).await; });
    }
    Ok(())
  }
}
```

**Step 5.4 — Tests**

Mock source emits 5 events. Verify they're written to store + broadcast on live channel.

**Step 5.5 — Commit**

```bash
git add -A && git commit -m "feat(sync): engine + live broadcast + backfill"
git push origin main
```

---

## Task 6: IPC commands — sessions, events, stats, settings

**Files:**
- Create: `src-tauri/src/ipc/mod.rs`, `sessions.rs`, `events.rs`, `stats.rs`, `settings.rs`
- Create: `src-tauri/src/secrets.rs`

**Step 6.1 — `sessions.rs`**

```rust
#[tauri::command]
pub async fn list_sessions(state: tauri::State<'_, AppState>, filter: SessionFilter, limit: u32, offset: u32) -> AppResult<Vec<CanonicalSession>>;
#[tauri::command]
pub async fn get_session(state: tauri::State<'_, AppState>, id: String) -> AppResult<(CanonicalSession, Vec<CanonicalEvent>)>;
```

`SessionFilter { source: Option<String>, since: Option<f64>, until: Option<f64>, search: Option<String> }`.

**Step 6.2 — `events.rs`**

```rust
#[tauri::command]
pub async fn stream_live_events(window: tauri::Window, state: tauri::State<'_, AppState>) -> AppResult<()>;
```

Uses `window.emit("live_event", &CanonicalEvent)` for each broadcast item. Renderer subscribes via `listen("live_event", cb)`.

**Step 6.3 — `stats.rs`**

```rust
#[tauri::command]
pub async fn dashboard_stats(state: tauri::State<'_, AppState>, since: f64) -> AppResult<DashboardStats>;
```

Returns: total cost USD, sessions count, error count, top tools (top 5), hourly buckets (24h).

**Step 6.4 — `settings.rs`**

```rust
#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> AppResult<Settings>;
#[tauri::command]
pub async fn set_hermes_password(state: tauri::State<'_, AppState>, password: String) -> AppResult<()>;
```

`secrets.rs` wraps `keyring` crate, service name `"com.spire-bridge.app"`, keys: `"hermes_password"`, `"hermes_oauth_refresh"`.

**Step 6.5 — Tests**

Each command has 1+ test against in-memory store.

**Step 6.6 — Commit**

```bash
git add -A && git commit -m "feat(ipc): sessions/events/stats/settings commands"
git push origin main
```

---

## Task 7: Frontend — typed API client + live stream hook

**Files:**
- Create: `src/lib/api/client.ts`, `src/lib/api/stream.ts`, `src/lib/query/keys.ts`, `src/lib/query/client.ts`
- Create: `src/lib/hooks/useLiveEvents.ts`, `src/lib/hooks/useShortcuts.ts`

**Step 7.1 — `client.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
export const api = {
  listSessions: (filter: SessionFilter, limit = 50, offset = 0) =>
    invoke<CanonicalSession[]>("list_sessions", { filter, limit, offset }),
  getSession: (id: string) => invoke<{ session: CanonicalSession; events: CanonicalEvent[] }>("get_session", { id }),
  dashboardStats: (since: number) => invoke<DashboardStats>("dashboard_stats", { since }),
  getSettings: () => invoke<Settings>("get_settings"),
  setHermesPassword: (password: string) => invoke<void>("set_hermes_password", { password }),
};
```

All types mirror Rust canonical types (regenerated by `cargo test` → `pnpm gen:types` script in Task 16).

**Step 7.2 — `stream.ts`**

```ts
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
export function streamLiveEvents(onEvent: (e: CanonicalEvent) => void): Promise<UnlistenFn> {
  return listen<CanonicalEvent>("live_event", (msg) => onEvent(msg.payload));
}
```

**Step 7.3 — `query/client.ts`**

QueryClient with `staleTime: 5_000`, `refetchOnWindowFocus: false`. Default `gcTime: 60_000`.

**Step 7.4 — `query/keys.ts`**

Factory pattern: `keys.sessions.list(f)`, `keys.sessions.detail(id)`, `keys.dashboard(since)`.

**Step 7.5 — `useLiveEvents.ts`**

Zustand `liveBuffer: CanonicalEvent[]` (cap 1000). On new event, push + trim. Expose `useLiveEventsBySession(sessionId)` selector.

**Step 7.6 — `useShortcuts.ts`**

Global keyboard shortcuts: `⌘K` (command palette), `⌘/` (search), `⌘1-9` (navigate routes), `Esc` (close dialog).

**Step 7.7 — Tests**

`tests/ts/api.test.ts` mocks `invoke`, asserts typed return.

**Step 7.8 — Commit**

```bash
git add -A && git commit -m "feat(frontend): api client + live stream hook"
git push origin main
```

---

## Task 8: App shell — sidebar, title bar, status bar

**Files:**
- Create: `src/components/shell/AppShell.tsx`, `Sidebar.tsx`, `TitleBar.tsx`, `StatusBar.tsx`
- Create: `src/routes/__root.tsx`, `src/routes/index.tsx`

**Step 8.1 — `TitleBar.tsx`**

Custom drag region (`data-tauri-drag-region`), 32 px tall, holds app name on left, three dot indicators on right (live / sync / health).

**Step 8.2 — `Sidebar.tsx`**

240 px wide, glass background. Sections:
- **Overview**, **Sessions**, **Hosts** (Phase 2), **Cost**, **Settings**
- Per-source collapsible group (Claude Code, OpenCode, Hermes) with session counts

Active item: red accent border-left + bg-white/8. Hover: bg-white/4.

**Step 8.3 — `StatusBar.tsx`**

32 px tall, glass. Shows: total active sessions, today's spend, sync status (green dot = healthy / red = error), "v0.1.0" version.

**Step 8.4 — `AppShell.tsx`**

Grid: `grid-cols-[240px_1fr]` rows `[32px_1fr_32px]`. Drag region on title bar + sidebar + status bar. Main scroll area.

**Step 8.5 — `__root.tsx`**

TanStack Router root with `<Outlet />`. Wraps with QueryClientProvider + Theme provider.

**Step 8.6 — `index.tsx` (Overview)**

Placeholder: "Spire Bridge" hero with subtitle "Loading…" + Skeleton cards (3 of them).

**Step 8.7 — Tests**

`tests/ts/shell.test.tsx` — render AppShell, assert sidebar items present, navigate to `/settings` via Sidebar click.

**Step 8.8 — Commit**

```bash
git add -A && git commit -m "feat(shell): app shell + sidebar + statusbar"
git push origin main
```

---

## Task 9: Overview dashboard — live activity, today's stats, top tools

**Files:**
- Create: `src/components/charts/CostSparkline.tsx`, `TokenFlow.tsx`, `ActivityHeatmap.tsx`
- Create: `src/components/live/LiveStream.tsx`, `LiveIndicator.tsx`
- Create: `src/components/session/SessionCard.tsx`, `SessionList.tsx`, `SessionHeader.tsx`
- Modify: `src/routes/index.tsx`

**Step 9.1 — `LiveIndicator.tsx`**

Pulsing red dot (Motion: `animate={{ scale: [1, 1.3, 1] }} transition={{ repeat: Infinity, duration: 1.5 }}`). When stream has no events for 5 s, dim it.

**Step 9.2 — `LiveStream.tsx`**

Scrolls live events into a virtualized list (max 200 rows visible). Each row: timestamp + agent icon + 1-line summary. Auto-scroll-to-top on new event; "Jump to latest" pill when scrolled up.

**Step 9.3 — `CostSparkline.tsx`**

Recharts `<AreaChart>` 24h hourly buckets, red gradient, glass background, no axis lines (clean look).

**Step 9.4 — `ActivityHeatmap.tsx`**

7×24 grid (last 7 days × 24 hours), cell opacity = event count. Tooltip on hover.

**Step 9.5 — `TokenFlow.tsx`**

Stacked bar: input / output / cache / reasoning over 7 days.

**Step 9.6 — `SessionCard.tsx`**

Glass card. Title, source badge, model, started, cost pill, message count, duration, last activity.

**Step 9.7 — Overview page composition**

Grid:
- Row 1: 4 KPI glass cards (today's spend, sessions, errors, avg cost/session)
- Row 2: 2-col — CostSparkline (60%) + ActivityHeatmap (40%)
- Row 3: 2-col — LiveStream (50%) + Top Tools list (50%)
- Row 4: recent sessions grid

**Step 9.8 — Tests**

`tests/ts/overview.test.tsx` — render Overview, mock API, assert 4 KPI cards render with mocked values.

**Step 9.9 — Commit**

```bash
git add -A && git commit -m "feat(dashboard): overview with live stream + charts"
git push origin main
```

---

## Task 10: Sessions list page with filters

**Files:**
- Create: `src/routes/sessions/index.tsx`
- Create: `src/components/session/SessionList.tsx`

**Step 10.1 — Filters UI**

Source chips (Claude Code / OpenCode / Hermes, multi-select). Date range. Search (debounced 250 ms). Status (active/ended/error).

**Step 10.2 — Table**

Glass background. Columns: Title, Source, Model, Started, Duration, Cost, Status, Actions.

**Step 10.3 — Pagination**

TanStack Query infinite query, 50 per page, "Load more" button at bottom.

**Step 10.4 — Row click**

Navigate to `/sessions/$sessionId`.

**Step 10.5 — Tests**

Mock API, assert filter changes refetch with new query key.

**Step 10.6 — Commit**

```bash
git add -A && git commit -m "feat(sessions): list page with filters"
git push origin main
```

---

## Task 11: Session detail — timeline (the money shot)

**Files:**
- Create: `src/components/timeline/SessionTimeline.tsx`, `EventRow.tsx`, `ToolCallRow.tsx`, `AssistantTextRow.tsx`, `UserPromptRow.tsx`, `CodeBlock.tsx`
- Create: `src/routes/sessions/$sessionId.tsx`
- Create: `src/components/charts/SessionCostChart.tsx`

**Step 11.1 — `SessionHeader.tsx`**

Top of session detail: title (editable inline), source icon, model, project path, git branch, started/ended, total cost, total tokens, tool call count.

**Step 11.2 — `SessionCostChart.tsx`**

Step chart showing cumulative cost + tokens per turn.

**Step 11.3 — `CodeBlock.tsx`**

Shiki highlighter (lazy-loaded with `@shikijs/transformers` for diff), monospace, glass-bg, line numbers, copy button, collapse if > 50 lines.

**Step 11.4 — `EventRow.tsx`**

Variants: `user` (right-aligned bubble, glass-bg), `assistant` (left-aligned, full-width), `tool` (icon + name + collapsible args/result), `error` (red left border, glass-bg red tint), `api` (compact metadata strip).

**Step 11.5 — `SessionTimeline.tsx`**

react-virtuoso with `followOutput: 'smooth'` when scrolled to bottom, "Jump to latest" pill otherwise. Sticky day separator rows.

**Step 11.6 — Subscribe to live events**

If session is active, subscribe to `streamLiveEvents` and prepend new events as they arrive.

**Step 11.7 — Tests**

`tests/ts/timeline.test.tsx` — render with 50 mocked events, assert virtualization mounts, scroll-to-bottom works.

**Step 11.8 — Commit**

```bash
git add -A && git commit -m "feat(session): detail page with virtualized timeline"
git push origin main
```

---

## Task 12: Per-agent pages + subagent tree

**Files:**
- Create: `src/routes/agents/$sourceId.tsx`
- Create: `src/components/session/SubagentTree.tsx`

**Step 12.1 — `$sourceId.tsx`**

Tabs: Sessions / Errors / Cost. Per-source aggregates.

**Step 12.2 — `SubagentTree.tsx`**

Recursive component (max depth 4) rendering parent → child sessions with cost rollup. Animated expand/collapse with Motion `layout`.

**Step 12.3 — Commit**

```bash
git add -A && git commit -m "feat(agents): per-source pages with subagent tree"
git push origin main
```

---

## Task 13: Cost analytics page

**Files:**
- Create: `src/routes/cost.tsx`
- Create: `src/components/charts/CostBreakdown.tsx`

**Step 13.1 — Cost page**

Date range picker. Stacked area: cost by source. Top sessions table. Forecast (simple linear regression on last 14 days).

**Step 13.2 — Commit**

```bash
git add -A && git commit -m "feat(cost): analytics page with forecasts"
git push origin main
```

---

## Task 14: Settings + Hermes auth flow + Claude OTel launcher

**Files:**
- Create: `src/routes/settings.tsx`
- Create: `src-tauri/src/ipc/setup.rs` (Claude OTel env injection)

**Step 14.1 — Settings page**

Sections: Sources, Hermes Auth, Claude Code, Display, About.

**Step 14.2 — Hermes password input**

`<TextField type="password">` → `setHermesPassword` command. Validate by calling Hermes `GET /global/health` after save.

**Step 14.3 — Claude Code integration toggle**

When enabled, write `~/.claude/settings.json` with `OTEL_*` env vars (or update user-level `~/.config/claude/settings.json`). Provide a "Test connection" button that reads the latest `~/.claude/projects/*/*.jsonl`.

**Step 14.4 — Commit**

```bash
git add -A && git commit -m "feat(settings): sources + auth + Claude OTel launcher"
git push origin main
```

---

## Task 15: Polish — animations, transitions, empty states, skeletons

**Files:**
- Modify: every page + every glass primitive to add Motion variants
- Create: `src/lib/motion/variants.ts`, `src/components/primitives/Empty.tsx`, `Skeleton.tsx`

**Step 15.1 — Motion variants**

Standard fade-in-up for routes: `{ initial: { opacity: 0, y: 8 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -8 } transition: { duration: 0.18 } }`. Respect `prefers-reduced-motion`.

**Step 15.2 — Stagger children**

List items stagger 30 ms each, max 200 ms total.

**Step 15.3 — Skeletons**

`<Skeleton variant="card" />`, `<Skeleton variant="text-line" />`, `<Skeleton variant="chart" />`. Glass-style shimmer animation.

**Step 15.4 — Empty states**

`<Empty icon="inbox" title="No sessions yet" hint="Start an agent run to see it here" />`.

**Step 15.5 — Commit**

```bash
git add -A && git commit -m "style: motion variants + skeletons + empty states"
git push origin main
```

---

## Task 16: CI, build pipeline, type-gen script

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `scripts/gen-types.sh`, `scripts/check.sh`
- Create: `src-tauri/gen/schemas/*.json` (committed for offline use)

**Step 16.1 — CI workflow**

Matrix: ubuntu-latest, macos-latest, windows-latest. Steps: install Rust 1.83, Node 20, pnpm 9. `pnpm install --frozen-lockfile`. `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`. `pnpm test && pnpm build && pnpm tauri build --bundles none` (CI smoke build).

**Step 16.2 — `gen-types.sh`**

Generate TS types from Rust canonical types via `ts-rs` (add `ts-rs = "0.9"` to Cargo deps, derive `TS` on canonical types, run `cargo test` → outputs `.ts` files).

**Step 16.3 — `check.sh`**

One-shot local CI: `pnpm install && cd src-tauri && cargo fmt --check && cargo clippy && cargo test && cd .. && pnpm test && pnpm build`.

**Step 16.4 — Commit + verify**

```bash
git add -A && git commit -m "ci: github actions matrix + type-gen"
git push origin main
./scripts/check.sh
```

Expected: all green.

---

## Task 17: Deep QA — exercise every flow, log defects, fix in batches

**Step 17.1 — Manual + scripted QA**

Run app for 30 min while generating real data:
- `claude` in 3 dirs (background)
- `opencode` in 1 dir
- `hermes` Telegram → use gateway
- Kill network on one agent (offline mode)
- Restart app, verify backfill works

**Step 17.2 — Performance check**

Cold start ≤ 800 ms (measure with `time`). 200-session list ≤ 16 ms/frame (Chrome DevTools Performance). 10k-event timeline at 60 fps (virtuoso).

**Step 17.3 — Visual review**

Take screenshots of every page. Verify: glass blur visible, no muddy text, dark/light contrast, red accent consistent, no dead pixels, no layout shift.

**Step 17.4 — Bug triage**

Collect all defects. Severity buckets: blocker / major / minor. File them in `docs/qa/iteration-1-defects.md`.

**Step 17.5 — Fix wave**

Dispatch fix subagents in parallel (one per defect cluster). Re-run full QA.

**Step 17.6 — Commit**

```bash
git add -A && git commit -m "fix(qa): iteration 1 defects"
git push origin main
```

---

## Task 18: Iteration 2 plan → iteration 2 build

After Task 17 ships cleanly: write `docs/plans/phase-1-iteration-2.md` covering every defect and improvement found. Build with same workflow. Repeat until 2 clean iterations ship.

**Step 18.1 — Write iteration-2 plan**

Same structure as this plan; tasks are the gaps found.

**Step 18.2 — Execute**

Subagent-driven. Stop if same defect reappears twice in a row → research → mark deferred.

**Step 18.3 — Final sign-off**

`./scripts/check.sh` green. README updated with screenshots. Tag `v0.1.0`.

---

## Task 19: Phase 2 plan

**Step 19.1 — Write `docs/plans/phase-2.md`**

Out of scope for Phase 1, deferred:
- Multi-host edge agent
- Steering (approve/abort/inject)
- Collaboration (multi-user, threaded comments)
- Workflows (DAG, versioned, replay)
- Builds/Containers surface
- Errors correlation engine
- Mobile companion (read-only)
- Self-hosted backend (Phase 3)

**Step 19.2 — Commit**

```bash
git add -A && git commit -m "docs: phase 2 plan"
git push origin main
```

---

## Self-Review Checklist (run before declaring Phase 1 done)

- [ ] All 17 tasks shipped + green
- [ ] At least 2 clean QA iterations completed
- [ ] Every page has a screenshot in `docs/screenshots/`
- [ ] Bundle size ≤ 350 KB gzipped
- [ ] Cold start ≤ 800 ms
- [ ] 200-session list renders at 60 fps
- [ ] 10k-event timeline scrolls at 60 fps
- [ ] No `unwrap()` in Rust non-test code
- [ ] No `any` in TS boundary types
- [ ] All agent secrets use OS keychain
- [ ] No third-party network calls
- [ ] Privacy toggle works (no `raw_json` write when off)
- [ ] `prefers-reduced-motion` honored
- [ ] CI green on all 3 OSes
- [ ] README has install + dev + build + screenshot
- [ ] MIT license committed
- [ ] Phase 2 plan committed

---

## Notes for implementer subagents

- Tauri dev needs the system webview (Linux: `webkit2gtk-4.1`); if missing, `cargo tauri dev` will error — install with `sudo apt install libwebkit2gtk-4.1-dev`.
- Avoid `serde_json::Value` everywhere in canonical types — use it only in `payload` where structure varies; everything else is typed.
- React 19 + react-virtuoso: virtuoso accepts `Virtuoso` component, but for variable-height content prefer `Virtuoso` (not `TableVirtuoso`).
- TanStack Router file-based routing generates `routesTree.gen.ts` on Vite start — don't edit it.
- Motion (formerly Framer Motion) v12 renamed imports: `import { motion } from "motion/react"` (NOT `framer-motion`).
- Recharts 2.x: set `style={{ overflow: 'visible' }}` on `<ResponsiveContainer>` parent to prevent clipping.
- For Tauri capabilities, `core:default` is required; add specific plugin caps as needed.
- Tauri 2 IPC streaming: use `tauri::ipc::Channel<T>` for renderer-initiated streams; `window.emit("event", ...)` for backend-initiated.