# Spire Bridge — Agent Guide

**What this is:** Desktop cockpit (Tauri 2 + React 19) that unifies live data from **Claude Code**, **OpenCode**, and **Hermes Agent** into a single liquid-glass dashboard. Phase 1 = read-only observation. Phase 2+ = steering, multi-host, collaboration.

**Read these before touching anything:**

1. `/home/ubuntu/spire-bridge/docs/plans/phase-1-spire-bridge.md` — the master plan (19 tasks, exact filenames, exact deps, exact code snippets).
2. `/home/ubuntu/spire-bridge/.superpowers/sdd/progress.md` — progress ledger; check first to see what's already done.

**Hard rules (from the plan's Global Constraints):**

- **No third-party network calls.** No analytics, no telemetry, no crash reporting. Privacy is the product.
- **All agent endpoints bind `127.0.0.1` only.** Never `0.0.0.0` for Hermes/OpenCode/Claude listeners.
- **Secrets go in the OS keychain** via the `keyring` crate (service: `com.spire-bridge.app`).
- **Read-only v1.** Do NOT call `POST /session/:id/abort`, do NOT respond to permission requests, do NOT inject prompts.
- **Strict TypeScript** (`strict: true`, `noUncheckedIndexedAccess: true`). No `any` on boundary types.
- **Strict Rust** (`clippy::all = deny` in CI, no `unwrap()` outside tests).
- **Conventional commits** (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`, `style:`).
- **Visual design tokens are locked.** See plan Global Constraint #8 — do not invent new colors or radii.
- **Motion budget:** ≤ 200 ms unless it's a "reveal" (≤ 600 ms). Respect `prefers-reduced-motion`.

**Shell setup (every shell):**

```bash
source ~/.spire_env   # adds pnpm + cargo to PATH
```

**Repo:**

- Path: `/home/ubuntu/spire-bridge/`
- Remote: `https://github.com/ferre-z/spire-bridge` (main branch)
- License: MIT
- After every commit: orchestrator pushes. Subagents never push.

**Build commands:**

- `pnpm install`
- `cd src-tauri && cargo check`
- `pnpm test`
- `pnpm build`
- `pnpm tauri dev` (needs X server / Wayland — not on this box)

**Common pitfalls (learned the hard way):**

- Cargo + pnpm are NOT on default PATH. `source ~/.spire_env` first.
- `libwebkit2gtk-4.1-dev` is required for Tauri on Linux — install via apt.
- Motion v12 imports as `motion/react`, NOT `framer-motion`.
- TanStack Router file-based routing auto-generates `src/routesTree.gen.ts` on Vite start — don't edit.
- Recharts: wrap `<ResponsiveContainer>` parent with `style={{ overflow: 'visible' }}` to prevent clipping.
- Tauri capabilities live in `src-tauri/capabilities/default.json` — `core:default` is required.