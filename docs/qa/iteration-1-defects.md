# Iteration 1 — Deep QA Defect Log

> Living document. Filled in during Task 17 (Deep QA) and consumed by
> Task 18 (Iteration 2 plan). Severity follows P0/P1/P2/P3, where
> P0 = blocker (no ship), P1 = major (must fix this iteration),
> P2 = minor (next iteration if cheap, else backlog), P3 = nit.

**Owner:** orchestrator
**Status:** in-progress (QA wave)
**Last updated:** 2026-07-05

---

## 0. How to run this QA wave

This section is the script a QA subagent (or a human) walks through.
Run order matters — later flows depend on earlier ones seeding the
SQLite store with realistic data.

### 0.1 Prereqs
- `source ~/.spire_env` (puts pnpm + cargo on PATH).
- `libwebkit2gtk-4.1-dev` installed (Linux only).
- Three scratch directories in `~/qa/`:
  - `~/qa/c1`, `~/qa/c2`, `~/qa/c3` — Claude Code projects.
  - `~/qa/o1` — OpenCode project.
  - `~/qa/h1` — Hermes session dir.
- A real Hermes gateway running locally (Telegram token in OS keychain
  for entry `hermes.gateway.telegram`).

### 0.2 Steps (mirrors plan Task 17.1)

```bash
# Terminal A — three Claude sessions, varied workloads
( cd ~/qa/c1 && claude "Review the src tree and list TODO comments" )
( cd ~/qa/c2 && claude "Run `pnpm build` and summarize the output" )
( cd ~/qa/c3 && claude "Write 10 vitest tests for ./src/lib/math.ts" ) &

# Terminal B — OpenCode session
( cd ~/qa/o1 && opencode "Add a new route /foo returning JSON {ok:true}" ) &

# Terminal C — Hermes gateway already running. Drop a Telegram message
# that triggers the gateway to spawn a session.

# Wait ~30 minutes for events to land.

# Now exercise the offline-mode branch on the CLAUDE source:
sudo iptables -A OUTPUT -p tcp --dport 443 -j DROP   # or unplug nic
# Watch the Spire Bridge app — Claude entry should flip to "Offline" state
# within ~15s (the retry/backoff cycle from sources/claude/*).
sudo iptables -D OUTPUT -p tcp --dport 443 -j DROP   # restore
# Verify backfill resyncs the missed events within 60s of restart.

# Restart the app:
pkill -f spire-bridge; sleep 2; pnpm tauri dev &
# Verify sessions count is >= what it was before restart (no data loss).
```

While the app is running:
1. **Perf** (17.2): `time` cold start (kill → launch), record ms.
2. **Visual** (17.3): screenshot every page → `docs/screenshots/`.
3. **Defect log** (17.4): for anything broken, append a row below.

### 0.3 Perf budget (from plan)

| Metric                              | Target        | How to measure                          |
|-------------------------------------|---------------|-----------------------------------------|
| Cold start (tap → window visible)   | ≤ 800 ms      | `time pnpm tauri dev` (first frame)     |
| 200-session list render             | ≤ 16 ms / frame | Chrome DevTools Performance           |
| 10k-event timeline scroll (virtuoso)| 60 fps        | DevTools FPS meter                       |
| Bundle size (gz)                    | ≤ 350 KB      | `pnpm build` then `gzip -c dist/...js` |
| Bundle size (raw)                   | tbd           | `du -h dist/assets/*.js`                |

---

## 1. Defects

> Copy a row from the template, bump the ID, fill it in.

### Template

```
### D-XXX — <one-line title>
- **Severity:** P0 | P1 | P2 | P3
- **Surface:** UI | IPC | Rust core | Store | Sync | Build | CI | A11y | Visual
- **Repo area:** `<path>`
- **Repro:**
  1. …
  2. …
- **Expected:** …
- **Actual:** …
- **Logs/screens:** `docs/qa/iteration-1/evidence/D-XXX.{png,txt}`
- **Discovered:** YYYY-MM-DD by <name>
- **Status:** open | triaged | in-fix | fixed | deferred
- **Fix cluster:** <links the parallel-fix dispatch, e.g. "fix(qa)-cluster-2">
```

### Open defects

> Populated below from the seed list. New defects append here. Re-runs
> during the fix wave may surface follow-ups — those get IDs D-NEW-N
> and go above any "fixed" entries.

#### Seed: defects anticipated from the plan's hard rules + global constraints

These are *anticipated* — confirm/deny during QA. If confirmed, they
get promoted to real IDs (D-001, D-002, …) below.

| ID    | Title                                                       | Sev | Plan ref                                |
|-------|-------------------------------------------------------------|-----|-----------------------------------------|
| D-001 | Any IPC call resolves a `0.0.0.0` upstream URL               | P0  | Global Constraint #2 (127.0.0.1 only)   |
| D-002 | Any code path sends payloads to a third-party network host  | P0  | Global Constraint #1 (no telemetry)     |
| D-003 | Secret value reaches the React layer unredacted             | P0  | Constraint + keychain plan              |
| D-004 | Permission/Auth/Refusal/Abort event accidentally steered   | P0  | Read-only v1 constraint                 |
| D-005 | Bundle size > 350 KB gz                                     | P1  | Self-review checklist                   |
| D-006 | Cold start > 800 ms                                         | P1  | Self-review checklist                   |
| D-007 | Timeline scroll drops below 60 fps on 10k events            | P1  | Self-review checklist                   |
| D-008 | Recharts parent missing `overflow: visible`                | P2  | Pitfall (AGENTS.md)                     |
| D-009 | Custom color/radius used outside the design-token system    | P1  | Global Constraint #8                    |
| D-010 | Animation > 200 ms w/o reveal justification (> 600 ms)      | P2  | Motion budget                           |
| D-011 | Animation runs despite `prefers-reduced-motion: reduce`     | P1  | Global Constraint + motion budget       |
| D-012 | TanStack Router `routesTree.gen.ts` hand-edited             | P2  | Pitfall                                 |
| D-013 | Capabilities file missing `core:default`                    | P0  | Pitfall                                 |
| D-014 | `unwrap()` outside `#[cfg(test)]`                           | P1  | clippy::all = deny (CI)                 |
| D-015 | Any `any` on Tauri IPC boundary types                       | P1  | TS strict + noUncheckedIndexedAccess    |
| D-016 | New dep added in a fix wave w/o license/MIT audit           | P1  | Convention                             |
| D-017 | Repo contains `dist/` artifacts in git                      | P2  | gitignore                               |
| D-018 | Adapter writes raw secret to `raw_json` column              | P0  | Redaction (Task 3)                      |
| D-019 | Backfill double-counts on restart                           | P1  | Task 5 (sync/backfill.rs)               |
| D-020 | Live event stream silently drops on websocket error         | P1  | Task 5 (sync/live.rs)                   |
| D-021 | Hermes gateway token leaked to renderer                     | P0  | Settings (Task 14)                      |
| D-022 | OpenCode URL allowlist accepts non-loopback                 | P0  | Sources (Task 4)                        |
| D-023 | Cost analytics page shows unbounded precision               | P2  | Visual review                          |
| D-024 | Session detail "money shot" timeline layout shifts CLs     | P2  | Visual review                          |
| D-025 | Light theme: glass blur renders muddy on white             | P1  | Visual review                          |
| D-026 | Sidebar collapses to 0 width at viewport ≤ 1024 px         | P2  | Visual review                          |
| D-027 | No empty-state copy on every list page                      | P2  | Task 15 polish                         |
| D-028 | Keyboard trap in dialogs/menus                              | P1  | A11y                                   |
| D-029 | Color-only signaling for errors (no icon/text)              | P1  | A11y                                   |
| D-030 | Tauri build fails on Windows MSRV linker                    | P1  | CI matrix                              |
| D-031 | Tauri build fails on macOS aarch64 universal                | P1  | CI matrix                              |
| D-032 | `cargo fmt --check` fails on any PR (no nightly rustfmt)    | P2  | CI                                     |
| D-033 | Offline-mode agent doesn't show re-connecting indicator     | P2  | UX                                     |
| D-034 | Hermes OTel launcher background child not killed on quit    | P1  | Task 14                                |
| D-035 | Per-agent page subagent tree crashes on circular parent_id  | P1  | Task 12                                |

#### Confirmed defects (live)

> Empty until QA subagent populates. As defects are confirmed by the
> QA wave, copy the seed row here, change the ID, add logs/screens,
> and mark status. If denied, leave the seed row but append a note
> on the row: `Denied by QA on YYYY-MM-DD — <why>`.

### Closed defects (after fix wave)

> Defects move here once the matching `fix(qa)` commit lands. Keep
> the row; just update `Status: fixed` and add the commit SHA.

---

## 2. Severity summary

> Filled in at the end of the QA wave, before the fix-wave dispatch.

```
P0: __ open, __ fixed
P1: __ open, __ fixed
P2: __ open, __ fixed
P3: __ open, __ fixed
```

Sign-off criterion: **P0 = 0**, **P1 = 0**, **P2 ≤ N** (where N is the
orchestrator-set budget — default 5). P3 may ship to a follow-up.

---

## 3. Fix-wave dispatch plan

When the wave is sealed:

1. Group remaining defects into clusters by **Repo area** (one cluster
   per area, max ~3-5 defects per cluster — keeps diffs reviewable).
2. Dispatch one subagent per cluster in parallel. Each gets the list
   of `D-XXX` IDs in scope and the template row format for the PR body.
3. Sequentially after all clusters land: re-run this whole QA doc
   (Step 17.5 re-run). Update defect statuses. Loop until sign-off.
4. Stop-if rule: if any defect reappears in two consecutive QA waves,
   the orchestrator marks it **deferred** with a one-line rationale
   (taken from plan Task 18.2).

---

## 4. Sign-off

```
QA iteration 1 sealed on YYYY-MM-DD by <name>
P0 count: 0
P1 count: 0
Sign-off commit: <sha7>
```

When the above lines are filled, Task 17 is done and Task 18 begins.
