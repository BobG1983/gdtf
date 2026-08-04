---
name: heartbeat
description: >-
  Two-hourly autonomous-build tick: read run-state, ensure cron, inspect worktrees,
  resume a dead run or start exactly one new build after clause audit. Append one log line.
argument-hint: ""
---

# /heartbeat — recovery tick for autonomous builds

Run-state: [`.claude/agent-memory/run-state.md`](../../agent-memory/run-state.md).
Build workflow: [`.claude/workflows/build-ticket.js`](../../workflows/build-ticket.js).
Clause audit: phase 0 of that workflow (GTW-962).

## Cron

One-line cron prompt: invoke `/heartbeat`. Schedule: every two hours, **local machine timezone**. Store cron id in run-state next to the dream cron.

## Tick order

1. **Read run-state** (in-flight ticket, worktree, workflow run id, last log lines).
2. **Check/recreate cron** if the heartbeat entry is missing (record id when created).
3. **`git worktree list`**. Ignore paths whose name contains `dream` (dream skill worktrees).
4. **Liveness** — if a run is claimed in-flight, judge from the **last records in the run transcript** (or workflow status), **not** file mtime.
5. **Act:**
   - Dead run with a resume id → resume from the **main session** with Workflow `resumeFromRunId` (a workflow cannot resume another workflow).
   - Alive run → log and stop (do not start a second build).
   - Nothing in flight → pick next ticket, run **clause audit** (phase 0 rules), then start **exactly one** `build-ticket` if the audit passes.
6. **Append one dated log line** to run-state; keep only the **last five**.

## Proof (operator)

Kill a run deliberately, let a tick fire, confirm the log line shows resume + run id. Not required for merge of this skill file alone.

## Do not

- Start multiple builds in one tick.
- Treat `dream-*` worktrees as ticket builds.
- Trust mtime alone for liveness.
