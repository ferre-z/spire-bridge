# Phase 2 — Steering, Multi-Host, Collaboration

> **Status:** draft (written after Phase 1 sign-off).
> **Parent plan:** [`phase-1-spire-bridge.md`](./phase-1-spire-bridge.md).
> **Goal:** turn the read-only Phase 1 cockpit into a write-capable
> hub that can drive the three agents from one place, federate across
> multiple workstations, and let a small team collaborate around the
> same live session stream.

This plan lists the eight deferred workstreams from the Phase 1 plan
plus a few that surfaced during Phase 1 review. Each is fully scoped
*enough* to be picked up by an iteration plan — not all the way down
to "fix this file on this line." Detail lands in each per-feature
plan that follows Phase 2 sign-off.

---

## Why Phase 2 exists

Phase 1 deliberately stopped at "look but don't touch." That boundary
is what made it shippable: no permission flows, no write protocols,
no multi-user state. But every user who got a Phase 1 build asked the
same first question the moment they opened it: "Can I just… approve
that run from here?"

Phase 2 is the answer. It widens the trust boundary in three
directions at once:

1. **Bidirectional:** the app stops being read-only.
2. **Multi-host:** the app stops being a single-machine tool.
3. **Multi-user:** the app stops being a single-headset tool.

Each direction is its own workstream below. They don't all ship
together and they don't have to land in the same order.

---

## Carry-over constraints (same as Phase 1)

These constraints do **not** soften in Phase 2. They get *harder*
because we're wiring write paths now.

1. **No third-party network calls.** Same as Phase 1.
2. **`127.0.0.1` only by default.** Multi-host intentionally widens
   this — see §3 — but only behind explicit opt-in per host.
3. **Secrets in OS keychain.** Expanded: now includes host auth
   tokens, OAuth refresh tokens, and gateway tokens for every host.
4. **Strict TS / Strict Rust.** No relaxation.
5. **Conventional commits.** Phase 2 brings more legal-sensitive
   surfaces (steering, auth). Each `feat:` that touches a write
   path must note the security review status in its body.
6. **Visual design tokens locked.** New surfaces (mobile companion,
  builds/containers) must adopt the same tokens, not invent new ones.
7. **Motion budget.** Same ≤ 200 ms / ≤ 600 ms reveal, but
   `prefers-reduced-motion` becomes *more* important in mobile.
8. **Read-paths stay isolated.** The Phase 1 read pipeline is sacred.
   Any Phase 2 task that needs to mutate source state must do it
   through a new `writers/` module so the read path never widens.
9. **Audit log.** Every steering action (`approve`, `abort`,
   `inject`) writes an immutable audit entry. Required for trust.

---

## 1. Steering — write paths (was read-only in Phase 1)

### 1.1 Permission decisions

The single highest-impact Phase 2 feature. Today the three agents
each implement their own permission prompt UI; Spire Bridge must
surface that prompt in the cockpit and forward the decision.

- Per-source config in `~/.spire-bridge/steering.toml` (TOML, not JSON
  — better for hand-edits) declaring which `kind`s to forward.
- IPC commands:
  - `get_pending_permissions()` → list of pending prompts.
  - `decide_permission(permission_id, decision: "allow" | "deny" | "allow_always", reason?)`.
- Audit log entries for every decision (allow, deny, allow_always).
- "Allow always" writes to per-source rule list (Claude settings,
  OpenCode allowlist, Hermes permissions) — **not** just to our DB.
- UI: new `<PermissionDrawer />` that animates in from the right
  edge of the title bar (≤ 600 ms reveal budget). Soft chime on
  arrival; respects `prefers-reduced-motion`.

### 1.2 Approve / abort

- `abort_session(session_id, reason)` — kills the upstream run; writes
  the audit entry first (so a race-y abort still leaves evidence).
- `restart_session(session_id, with_overrides)` for retryable runs.
- Per-agent nuances:
  - **Claude Code:** respect the existing `exponential_backoff` on
    hook degradation; if upstream is in a degraded state, refuse
    abort and surface the upstream error verbatim.
  - **OpenCode:** honor the per-session lockfile concept; aborting
    a locked session emits a "would break lockfile" warning.
  - **Hermes:** broadcast through the configured Telegram channel
    first; only locally abort if user holds a special modifier key.

### 1.3 Prompt injection (read-only becomes read-write)

- `inject_message(session_id, content, at: "head" | "tail" | "after_seq")`.
- Strict guard: cannot inject into a session whose `end_reason` is
  `user_aborted` or `api_refusal` — those are sealed.
- Scrub injection payload for secrets through the Phase 1 redactor
  *before* it lands upstream. (Defense in depth — the redactor was
  meant for read paths; reusing it here makes "send" safe.)
- Audit log captures final scrubbed payload (with redactions, not
  the original).

### 1.4 Why this is the biggest trust move

Any write path is the difference between "useful dashboard" and
"remote control." Three trust rules, all forced into the code (not
just into docs):

- **Pair-confirmation:** every write path requires a second unlock
  gesture unless an explicit "trusted device" flag is set in OS
  keychain. Default is OFF.
- **Session-scoped tokens:** write tokens are issued per-session,
  scoped to `(host, source_id, session_id, expire_at)`. Tokens
  expire at session end.
- **Replay defense:** every write RPC carries a monotonic nonce
  bound to the audit log; out-of-order rejects surface as
  "out-of-sync, refresh."

These three rules get implemented as a `writers/` module under
`src-tauri/src/writers/` so the read path (Phase 1) is provably
unaffected.

---

## 2. Multi-host edge agent

### 2.1 The problem

Phase 1 assumes the user runs all three agents on the same
workstation as the cockpit. That breaks the moment a teammate
fires up Claude on their laptop, or a CI farm spawns 100 OpenCode
sessions on different machines. Phase 1 can't see those.

### 2.2 The agent

A new binary, `spire-bridge-edge`, that:

- Runs on each host that has agents but no cockpit.
- Discovers agents via the same Source trait Phase 1 uses, locally.
- Forwards canonical events to a central `spire-bridge-hub` over a
  mutually-authenticated mTLS channel.
- Holds zero state of its own; SQLite lives on the hub.
- Auto-updates via the same `tauri-plugin-updater` infrastructure
  Phase 1 already wires up.

### 2.3 Discovery

Two modes:

- **Manual pairing:** user pastes an `spire://pair?token=...` URL
  into the cockpit. Token is single-use, expires in 5 min, scoped
  to the originating device fingerprint.
- **mDNS / DNS-SD on the LAN.** Local-only, never WAN. Discovered
  peers show as `host.local (laptop, home)`. LAN discovery is
  default-off.

### 2.4 Schema impact

Adds a `host` table:

```
host {
  id            TEXT PRIMARY KEY,   -- ulid
  hostname      TEXT NOT NULL,
  fingerprint   TEXT NOT NULL,     -- SHA-256 of host pubkey
  labels        TEXT NOT NULL,      -- json: { "role": "workstation", ... }
  first_seen    INTEGER NOT NULL,
  last_seen     INTEGER NOT NULL
}
```

And a `host_id` column on `session` and `event` tables (nullable for
backward compatibility — Phase 1 sessions get host_id = NULL meaning
"this host").

### 2.5 Conflict resolution

Two hosts could observe the same upstream source. Resolution rule:
**first-wins per (source_id, native_id)**. The hub records the
binding; later writes from another host are silently dropped with an
audit entry. This is the simplest rule that's safe; per-event CRDTs
are out of scope.

---

## 3. Collaboration (multi-user, threaded comments)

### 3.1 What "collaboration" means here

Not Google Docs. Sharing a **read-only annotated view** of a session
across a small team. Three primitives:

- **Threaded comments** on a single session (anchored to `seq` or
  `time_range`).
- **Presence dots** in the title bar — who's looking at what.
- **Reactions** (👍 / 👀 / 🚨) on individual events.

### 3.2 Transport

A new Tauri command lands; the client switches to a
`spire-bridge-collab` crate that:

- Uses `yrs` (Yjs) for CRDT. Per-session CRDT, hubs merge.
- Routes through the hub only. Edge agents do **not** see collab
  state — privacy boundary preserved.
- WebRTC fallback for high-frequency presence so the hub isn't the
  bottleneck for cursors.

### 3.3 Permissions

A new role model:

| Role     | Read | Comment | React | Steer |
|----------|------|---------|-------|-------|
| Viewer   | yes  | no      | yes   | no    |
| Reviewer | yes  | yes     | yes   | no    |
| Operator | yes  | yes     | yes   | yes   |

Roles are local to the hub; edge agents only see their own role.
Role assignment writes an audit log.

### 3.4 Out-of-scope (Phase 3?)

- Granular per-event permissions.
- Public share links.
- SSO / OIDC.

---

## 4. Workflows (DAG, versioned, replay)

### 4.1 What a "workflow" is

A DAG of agent tasks where each node is one of:

- `prompt(session_id, prompt_text)` — fire a prompt into a session.
- `await_seq(session_id, seq, timeout)` — wait for an event.
- `branch(condition_event, when_true, when_false)` — gate.
- `parallel([node, …])` / `sequence([node, …])` — composition.
- `human_approve(prompt_to_user)` — pause for a steer.

Workflows are versioned (semver), signed (per-user ed25519), and
replayable against any historical session.

### 4.2 Where they live

New editor surface in the cockpit: a left-rail `<WorkflowCanvas />`
powered by `reactflow`. JSON serialization, no DSL. Workflows
serialize to `~/.spire-bridge/workflows/<name>.json`.

### 4.3 Replay semantics

A workflow can be replayed against any session_id. Replay runs the
DAG in *dry-run* mode against the recorded event stream: instead of
firing `prompt`, it asserts the prompt *would have fired* at the
right `seq`. The output is a per-step table: predicted vs actual,
diff highlights. This is the killer feature — historical debugging.

### 4.4 Out-of-scope

- Cross-host workflow execution. A workflow always runs against the
  hub's local store; cross-host is a Phase 3 feature.
- Workflow marketplace / sharing.

---

## 5. Builds / Containers surface

### 5.1 Why this matters

Agent runs have artifacts: files written, commands run, processes
spawned. The Phase 1 timeline shows the *what*, but not the *where*
— where did this file land, what was the working dir, what env vars
were set.

### 5.2 The surface

A new page `/builds` shows:

- A per-session tree of file deltas (added / modified / deleted),
  computed by snapshotting `cwd` at session start and end.
  Implementation: a new `file_snapshot` table storing Merkle roots,
  not full copies.
- A container view: when a session ran inside Docker/Podman
  (detected via inspecting the process tree), show the container
  image, env (redacted), and exit code.
- Pin to commit: if the cwd is a git repo, link to the git revision
  at session end.

### 5.3 Privacy

Containers page is **opt-in per workspace**. Default is OFF — file
delta computation has to walk the tree, which is slow and surfaces
private paths the user may not want surfaced.

---

## 6. Errors correlation engine

### 6.1 The problem

Three agents × many sources × many sessions = lots of errors. Phase 1
shows them as a single "errors" count. Phase 2 makes them queryable.

### 6.2 What it does

A new `errors` table with normalized fingerprints:

```
errors {
  id            TEXT PRIMARY KEY,
  session_id    TEXT NOT NULL,
  seq           INTEGER NOT NULL,
  source_id     TEXT NOT NULL,
  fingerprint   TEXT NOT NULL,    -- 64-bit xxhash of normalized body
  kind          TEXT NOT NULL,    -- 'rate_limit', 'context_overflow', 'tool_timeout', etc.
  message_redacted TEXT NOT NULL,
  first_seen    INTEGER NOT NULL,
  last_seen     INTEGER NOT NULL,
  occurrences   INTEGER NOT NULL
}
```

A new page `/errors`:

- Cluster view: top N fingerprints with `occurrences` and "first
  seen / last seen".
- Time-series: when a given fingerprint spikes, see the side-by-side
  context (which model, which tool, what cwd).
- Per-session drill-down.

### 6.3 Out-of-scope

- ML-based clustering. We use deterministic fingerprints.
- Public dashboards. Errors stay local.

---

## 7. Mobile companion (read-only)

### 7.1 Why mobile

Steering approval needs eyes-on in 30 seconds flat. Even an amazing
desktop app can't compete with a phone notification.

### 7.2 What it is

A Tauri **iOS + Android** companion app (sharing 80% of the React
codebase via the `tauri build --target ...` cross-compile). Read-only
by default. Approve permission prompts via push notification. No
write paths beyond `decide_permission`.

### 7.3 Why Tauri, not React Native

We have a single codebase invariant: visual design tokens locked,
one motion budget, one type system. Tauri on iOS/Android keeps that
invariant. React Native would fork the codebase.

### 7.4 Phase plan

This is the latest land. Needs the Phase 1 design tokens locked
before it can even be sketched (no design dollars for two surface
areas), and needs the steering write-path (§1) shipped first (it's
the only feature that earns a notification).

### 7.5 Out-of-scope (Phase 3+)

- iPad split-view dashboard.
- Apple Watch glance.
- CarPlay. (Don't laugh — it's actually a great surface for "is
  this background session still healthy." Phase 3.)

---

## 8. Self-hosted backend (Phase 3)

Listed here so Phase 2 doesn't accidentally assume it.

### 8.1 What it is

Replace the local hub with a small multi-user server. Phase 1/2
assume single-user. A self-hosted version unlocks teams. Different
deployment model entirely:

- No Tauri (it's a CLI binary + web UI).
- Multi-tenant SQLite (different schema — DO NOT backport). New
  project: `spire-bridge-cloud`.
- Same wire protocol; SDK is shared.

### 8.2 Sequencing

Phase 2 §3 (collaboration) builds against the *single-user* hub. If
a team grows past 5 operators and starts wanting granular roles,
that's the trigger to spin up `spire-bridge-cloud`. Phase 3 is its
own company.

---

## Step ordering (Phase 2)

Most workstreams above have small dependencies on each other. A
sensible ordering:

```
1. Steering write paths (§1)               — broadest impact
2. Errors correlation (§6)                 — low-dependency, fast value
3. Builds/Containers (§5)                  — gated on §1 audit log shape
4. Multi-host edge agent (§2)              — unlocks distributed use
5. Workflows (§4)                          — gated on §1 + §6
6. Collaboration (§3)                      — gated on §2 (presence is per-host)
7. Mobile companion (§7)                   — gated on §1 (approve is the only feature)
8. Self-hosted backend (§8 = Phase 3)      — Phase 3 trigger, not Phase 2
```

But the phase boundaries above are softer than Phase 1's: any single
workstream that earns its keep ships alone. There's no monolithic
"Phase 2 release" — it's an array of landable features gated on the
constraints above.

---

## Sign-off criterion for Phase 2

When the following are true, Phase 2's first iteration is done:

- [ ] Three write paths in production with audit log + pair-confirm.
- [ ] `spire-bridge-edge` binary ships in CI matrix.
- [ ] `/errors` page exists and clusters work.
- [ ] `/builds` page opt-in path documented in README.
- [ ] No regression on Phase 1 perf budgets.
- [ ] All write paths behind a `writers/` module that's < 30%
      the size of `sources/`. (Containment is the trust contract.)

When all of those are checked, this plan moves to "frozen" status
and a `phase-2-iteration-2.md` (or similar) takes over from here.
