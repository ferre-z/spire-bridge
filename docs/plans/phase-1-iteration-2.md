# Phase 1 — Iteration 2

> Companion to `phase-1-spire-bridge.md`. Run **after** Task 17's
> `docs/qa/iteration-1-defects.md` is sealed and only if severity counts
> at sign-off time don't already exceed the P1=0 / P2≤N budget.

## Why this doc exists

Iteration 1 ships a desktop cockpit that observes Claude Code, OpenCode,
and Hermes Agent. Iteration 2 is the first feedback round — small,
sharply scoped tasks that close the gaps surfaced by deep QA and that
the orchestrator couldn't cleanly fit into Phase 1 without scope creep.

Hard rules (from `phase-1-spire-bridge.md` Global Constraints) still
apply. Anything that violates the read-only-v1 stance gets deferred to
Phase 2.

**Stop rule (from plan Task 18.2):** if the same defect reappears in two
consecutive QA waves, mark it `deferred` and move it to Phase 2 scope
where it can be re-scoped properly. Do not loop forever on one bug.

---

## Global Constraints (carry-over)

These are unchanged from Phase 1. Re-state here so an iteration-2
subagent doesn't have to load the master plan to know what's allowed.

1. **No third-party network calls.** Privacy is the product.
2. **All agent endpoints bind `127.0.0.1` only.**
3. **Secrets live in the OS keychain** (`keyring`, service
   `com.spire-bridge.app`). Never in SQLite, never in frontend state.
4. **Strict TypeScript:** `strict: true`, `noUncheckedIndexedAccess: true`,
   `no any` on Tauri IPC boundary types.
5. **Strict Rust:** `clippy::all = deny` in CI, no `unwrap()` outside tests.
6. **Conventional commits:** `feat:`, `fix:`, `chore:`, `docs:`,
   `refactor:`, `test:`, `style:`, `build:`, `ci:`.
7. **Visual design tokens locked.** No new colors / radii.
8. **Motion budget:** ≤ 200 ms (≤ 600 ms for "reveal" animations).
   Respect `prefers-reduced-motion`.
9. **No bundling of network/file-system telemetry** in the binary.

---

## Tasks

### T2-1 — Reliability: backfill idempotency

**Files:** `src-tauri/src/sync/backfill.rs`, `src-tauri/src/store/mod.rs`,
new `src-tauri/src/sync/tests_backfill_idempotent.rs`.

**Why:** the QA wave's `D-019` (backfill double-counts on restart) is
likely the highest-impact data-correctness bug in the v1 sync engine.
Backfill must be safe to run repeatedly over the same window without
producing duplicate `event` rows or inflated session totals.

**Steps:**

1. Add a unique index `(session_id, seq)` to the `event` table in the
   next migration (`migrations/V004__event_unique_seq.rs`). Backfill
   uses `INSERT OR IGNORE` so a re-run is a no-op.
2. Cover with a Rust unit test: run the same backfill twice against a
   seeded store; assert row counts and totals match between runs.
3. Re-run `cargo test` + `./scripts/check.sh`. CI must stay green.

**Out of scope:** changing the on-disk SQLite format beyond the new
unique index. No data migration backfill logic is needed because new
sessions written after the migration inherit the constraint.

---

### T2-2 — ts-rs integration for type generation

**Files:** `src-tauri/Cargo.toml`, `src-tauri/src/sources/mod.rs`,
`src-tauri/src/store/schema.rs`, `scripts/gen-types.sh`,
`src-tauri/gen/schemas/*.canonical.json` (regen),
`src/types/canonical.ts` (regen).

**Why:** today `scripts/gen-types.sh` hand-emits JSON fallbacks. Long
term that drifts. Land the `ts-rs = "0.9"` integration the Phase 1
plan called for so `cargo test` (or a dedicated `--features ts-rs`
test) emits `target/bindings/*.ts` and `gen-types.sh` becomes a thin
copy step.

**Steps:**

1. Add `ts-rs = "0.9"` to `src-tauri/Cargo.toml` (`[dependencies]`)
   plus a `ts-rs = "0.9"` entry under `[features.ts-rs]` keyed off a
   new `ts-rs` feature. Only enabled when `--features ts-rs`.
2. Derive `#[derive(TS)]` on `CanonicalSession`, `CanonicalEvent`,
   `EventKind`, `Session`, `Event`; annotate each with `#[ts(export)]`.
3. Add a `tests/gen_types.rs` integration test gated on the feature
   that calls `m::export_all_to("./target/bindings")`.
4. In `scripts/gen-types.sh` Stage 1, replace the heredoc block with:
   ```bash
   ( cd src-tauri && cargo test --features ts-rs --test gen_types )
   cp -r src-tauri/target/bindings/. "$GEN_DIR/" || true
   ```
5. Regenerate the canonical JSONs and TS interfaces. Diff must be
   empty vs. committed.

**Verification:** `./scripts/check.sh` green. `./scripts/gen-types.sh --check`
green. `pnpm typecheck` clean against the regenerated `src/types/canonical.ts`.

---

### T2-3 — Live stream error surfacing

**Files:** `src-tauri/src/sync/live.rs`, `src/types/ipc.ts`,
`src/components/ConnectionBanner.tsx` (new).

**Why:** QA likely catches `D-020` (silent drop on websocket error).
The current behavior is "events stop, no UI signal." The fix is a
bounded retry loop with backoff + a status flag wired through Tauri
events so the React shell can show a "Reconnecting…" banner.

**Steps:**

1. Wrap the upstream websocket reads in `sync::live::LiveStream` with
   an exponential backoff (250 ms → 1 s → 4 s → 15 s, cap at 15 s).
   Don't infinite-loop — cap retry count at N=20 and emit `SourceError`.
2. Emit a new Tauri event `source_status` with payload
   `{ source_id: string, status: "connecting" | "live" | "reconnecting" | "offline" }`.
3. React: new `<ConnectionBanner />` in the title bar that listens on
   `source_status` and shows a pulsing amber dot + "Reconnecting…" copy
   for any non-`live` state. Banner respects `prefers-reduced-motion`.
4. Test: a small Vitest unit test fakes the listener and asserts the
   banner renders the right state for each status. Wire it to the
   existing vitest setup.

**Out of scope:** changing the backfill or auth flows.

---

### T2-4 — Vitest coverage for IPC client + live hook

**Files:** `tests/ipc-client.test.ts`, `tests/use-live-events.test.ts`,
new mocks under `tests/__mocks__/`.

**Why:** `D-014`-adjacent — clippy is fine, but the IPC client and
the live-events hook in the React shell have no automated coverage
today. Both are public API surface.

**Steps:**

1. Stub `window.__TAURI_INTERNALS__` (or use `@tauri-apps/api/mocks`)
   to return canned `invoke` and `listen` responses.
2. Assert `listSessions` returns the typed shape, errors propagate.
3. Assert the `useLiveEvents` hook emits updates and tears down the
   listener on unmount.
4. Add to `pnpm test` (already runs `vitest run`).

**Verification:** `pnpm test` covers both new files; coverage delta
> 5 percentage points on `src/ipc/`.

---

### T2-5 — Performance pass: lazy-loaded agent pages

**Files:** `src/routes/agents/*.{tsx}`, `vite.config.ts`,
new `src/routes/agents/lazy.tsx`.

**Why:** `D-007` and `D-026` cluster around rendering cost. The
per-agent pages each pull `recharts`, `shiki`, and a heavy subagent
tree. Lazy-load them behind TanStack Router's route-level code split.

**Steps:**

1. Convert `src/routes/agents/` to use TanStack's `lazyRouteComponent`
   or equivalent (since file-based routes are auto-generated, do this
   inside the leaf file with `createLazyRoute`).
2. Verify the bundle splits out — `pnpm build` and `ls dist/assets/`
   should show ≥ 2 chunks for the agent pages.
3. Add a comment with the perf budget table from
   `docs/qa/iteration-1-defects.md`.

**Verification:** `pnpm build` produces chunks; manual perf trace on
a synthetic 10k-event timeline.

---

### T2-6 — A11y sweep

**Files:** varies. Surface that came up in QA:

- `src/components/EmptyState.tsx` (new — pull out from `EmptyState`
  inside route files into a shared component).
- `src/components/IconButton.tsx` — ensures `aria-label` always set.
- `src/components/GlassCard.tsx` — contrast fixes for light theme.
- `src/styles/globals.css` — focus-visible style block.

**Why:** `D-028`, `D-029`, `D-025` cluster on accessibility and
contrast. Most are P1 from the seed inventory.

**Steps:**

1. Wire `aria-live="polite"` on the connection banner (T2-3 feeds
   into this).
2. Audit every red-only error pill; require an icon + text label.
3. Run `axe` over each page (manual script in QA iteration 2).
4. Add a `prefers-reduced-motion: reduce` test that vitest can run
   headlessly (mock `matchMedia`, assert class names flip).

---

### T2-7 — README + screenshots

**Files:** `README.md`, `docs/screenshots/README.md` (new),
`docs/screenshots/*.png` (5-7 images).

**Why:** the Phase 1 self-review checklist requires screenshots for
every page, and the README is what a landing visitor actually sees.

**Steps:**

1. Capture one screenshot per page at 1440×900, light + dark mode.
2. Embed in `README.md` using `<img>` blocks with `alt` text.
3. Add a "Self-hosting" section that points at Phase 2.

---

### T2-8 — Tauri build matrix cleanup

**Files:** `.github/workflows/ci.yml` (regen), `src-tauri/Cargo.toml`
(possible MSRV bump), `src-tauri/tauri.conf.json` (Windows resource
if needed).

**Why:** `D-030`, `D-031` — Windows MSRV linker failure and macOS
aarch64 universal-target oversights. These are real if anyone has run
the CI matrix off Linux.

**Steps:**

1. Run the full matrix locally once via `act` or in GitHub; collect
   log fragments; fix as needed.
2. Bump MSRV to a version that has a known-good linker if Cargo.lock
   regressed.
3. Verify macOS universal target builds with `pnpm tauri build
   --target universal-apple-darwin` (informational only — CI stays
   on per-arch for speed).

---

## Step ordering

```
        ┌──────────────┐
        │  T2-1 (backfill)  │  most data-critical, do first
        └──────┬───────┘
               │
        ┌──────▼───────┐
        │  T2-3 (live stream + banner) │  shared infra with T2-4
        └──────┬───────┘
               │
  ┌────────────┴────────────┐
  ▼                         ▼
┌────────────┐   ┌────────────────┐
│ T2-2 (ts-rs)│   │ T2-4 (vitest ipc) │
└────────────┘   └────────────────┘
        │                    │
        └────────┬───────────┘
                 ▼
          ┌──────────────┐
          │ T2-5 (lazy split) │
          └──────────────┘
                 │
          ┌──────▼───────┐
          │ T2-6 (a11y)  │
          └──────┬───────┘
                 │
          ┌──────▼───────┐
          │ T2-7 (README+screenshots) │
          └──────┬───────┘
                 │
          ┌──────▼───────┐
          │ T2-8 (build matrix) │
          └──────┬───────┘
                 │
        ┌──────▼───────┐
        │   ./scripts/check.sh        │
        │   + full re-run of QA doc   │
        │   → if P1=0 / P2≤5: seal v0.1.0 │
        └─────────────────────────────┘
```

---

## Sign-off (Task 18.3)

When all eight tasks merge and `./scripts/check.sh` is green:

```bash
git tag v0.1.0
git push origin main --tags   # orchestrator only
```

Tag is the gate; README badges reference the tag SHA; Phase 2 plan
(`phase-2.md`) becomes the active plan the next iteration builds against.
