# Spire Bridge — QA

This directory is the QA home for Phase 1 and beyond.

## Layout

```
docs/qa/
├── README.md                       — this file
├── iteration-1-defects.md          — Phase 1 iteration 1 defect log
└── iteration-1/                    — evidence (created on first QA wave)
    ├── evidence/                   — screenshots, log dumps, repro scripts
    └── scripts/                    — any throwaway repro commands
```

## Workflow

1. **Seed the QA wave** by reading [`iteration-1-defects.md`](./iteration-1-defects.md).
   It contains the runbook + a 35-row anticipated-defect table.
2. **Walk the runbook** (set up Claude/OpenCode/Hermes, exercise
   flows, kill network on one agent, restart and verify backfill).
3. **Capture evidence** under `iteration-1/evidence/D-XXX.{png,txt}`.
4. **Promote** confirmed defects from the seed table into the
   "Confirmed defects (live)" section of `iteration-1-defects.md`,
   with the matching evidence path.
5. **Triage severity** (P0/P1/P2/P3) and dispatch fix-wave subagents
   in parallel clusters (one cluster per repo area).
6. **Re-run** after the fix wave. Loop until P0=0, P1=0, P2 ≤ N
   (orchestrator-set budget; default 5).
7. **Sign off.** Move all rows to "Closed defects" with the matching
   commit SHA, then start the iteration-2 plan.

## Hard rules from the QA wave

- Source `~/.spire_env` before every shell (puts pnpm + cargo on PATH).
- The orchestrator pushes between waves; **QA subagents never push**.
- Take screenshots in `docs/qa/iteration-1/evidence/`, not at the
  repo root, not in `Desktop/`.
- Treat screenshots as private — see `iteration-1/README.md`
  ("Privacy" section, to be written by the QA subagent).

## Why a separate QA dir

Defects are *evidence-driven*; bundling them with the rest of
`docs/` keeps review easy. Every README badge in `docs/plans/` can
link back to a single source of truth for "what was wrong and when."
