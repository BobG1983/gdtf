---
name: heartbeat
description: >-
  Two-hourly autonomous-build tick: read run-state, ensure the cron, check the tree,
  resume a dead run or start exactly one new build. Append one log line.
argument-hint: ""
---

# /heartbeat — recovery tick for autonomous builds

Run-state: [`.claude/run-state.md`](../../run-state.md) — gitignored and machine-local, so a
tick on another machine starts blind.
Build workflow: [`.claude/workflows/build-ticket.js`](../../workflows/build-ticket.js).
Clause audit: phase 0 of that workflow (GTW-962) — the workflow runs it, this tick does not.

## The cron — this tick owns it

Cron jobs are session-only. They die when the session exits, so nothing survives a
restart on its own. This tick is the only thing that puts it back:

| Name | Schedule | Prompt |
|------|----------|--------|
| heartbeat | every two hours | `/heartbeat` |

**Local machine timezone.** Avoid exact hour and half-hour marks — pick an off-minute
so ticks do not pile onto the same instant as everyone else's. Record the id in
run-state.

The job also expires after 7 days on its own, even in a session that never restarts.
The check is presence, not age: recreate it if `CronList` does not show it, otherwise
leave it alone.

## Tick order

1. **Read run-state** (in-flight ticket, branch, workflow run id, last log lines). If the
   file is missing, say so and stop — do not start a build blind, because a tick that
   cannot see what is in flight is the one that starts a second one.
2. **Check the cron; recreate it if missing** (record the id in run-state). An empty
   `CronList` after a restart is the normal case, not a surprise.
3. **Check the tree**: current branch, `git status --porcelain`, `git worktree list`.
   Builds run on a feature branch in the MAIN repo — worktrees are banned by
   [`git-workflow.md`](../../rules/git-workflow.md). Any entry past the main repo is
   leftover from before that ban: report it, never adopt it as a live build.
4. **Liveness** — if a run is claimed in-flight, judge from the **last records in the run transcript** (or workflow status), **not** file mtime.
5. **Act:**
   - Dead run with a resume id → resume from the **main session** with Workflow `resumeFromRunId` (a workflow cannot resume another workflow).
   - Alive run → log and stop (do not start a second build).
   - Nothing in flight → pick the next ticket off the queue in run-state and start
     **exactly one** `build-ticket`. Do not pre-audit it by hand: phase 0 fetches the live
     Linear text and audits the clauses itself, and blocks before touching status if they
     do not hold.
6. **Append one dated log line** to run-state; keep only the **last five**.

## Proof (operator)

Kill a run deliberately, let a tick fire, confirm the log line shows resume + run id.

## Do not

- Start multiple builds in one tick.
- Create a worktree, or read an existing one as a live build.
- Trust mtime alone for liveness.
- Launch a build with `Workflow({name: 'build-ticket'})`. A named workflow resolves once
  per session and replays that frozen copy, so edits to the file are ignored for the rest
  of the session. Use `{scriptPath: '.claude/workflows/build-ticket.js'}`.
