---
name: dream
description: >-
  Promote durable facts from agent-memory ephemeral/ into real/, then consolidate
  near-duplicates in real/. Always re-check claims against code, docs, or the board.
  Prefer a dedicated worktree so the main tree stays clean.
argument-hint: ""
---

# /dream — promote and consolidate agent memory

Layout: [`.claude/agent-memory/`](../../agent-memory/index.md) (see GTW-952).

## Where this runs

- Prefer a **dedicated worktree** whose path contains `dream` (e.g. `.claude/worktrees/dream-YYYYMMDD`).
- Branch name should start with `dream/` or `feature/dream-` so `/heartbeat` does not treat it as a ticket build.
- Never write real/ from a dirty main tree mid-ticket.
- Cron (optional): local midnight, one line that invokes `/dream`. Timezone = **local machine**. Record cron id in `agent-memory/run-state.md`.

## Pass 1 — Promote

1. List every immediate child of `.claude/agent-memory/` that is a directory with `ephemeral/` and `real/`.
2. For each agent, read every file under `ephemeral/` (skip empty / `.gitkeep`).
3. For each claim: re-check against the tree, `docs/`, or Linear. If unverifiable → leave in ephemeral or mark uncertain; **do not promote on faith**.
4. Promote durable true facts into `real/` (same memory file format as siblings). Update [`index.md`](../../agent-memory/index.md).
5. Never delete an ephemeral file without reporting what was dropped and why.

## Pass 2 — Consolidate

1. Within each `real/`, find near-duplicates.
2. Merge only after re-checking the merged claim. Drop false claims with a one-line reason.
3. Refresh `index.md` so it links every real file and nothing under ephemeral.

## Report (required)

Counts: **promoted**, **merged**, **deleted**, **left alone**. If ephemeral is empty, say so — do not invent promotions.

## Out of scope

Game code changes. Ticket builds. Editing `qa-protocol/` design records.
