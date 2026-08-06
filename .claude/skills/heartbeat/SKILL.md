---
name: heartbeat
description: >-
  Two-hourly autonomous-build tick: ensure the cron, check the tree, judge liveness,
  resume a dead run or start exactly one new build.
argument-hint: ""
---

# /heartbeat — recovery tick for autonomous builds

Run-state is read and written by the **`/run-state` skill**, which owns the file's sections and
their formats. This tick never states a run-state format itself.
Build workflow: [`.claude/workflows/build-ticket.js`](../../workflows/build-ticket.js).
Clause audit: phase 0 of that workflow — the workflow runs it, this tick does not.

## The cron — this tick owns it

Cron jobs are session-only. They die when the session exits, so nothing survives a
restart on its own. This tick is the only thing that puts it back:

| Name | Schedule | Prompt |
|------|----------|--------|
| heartbeat | every two hours | `/heartbeat` |

**Local machine timezone.** Avoid exact hour and half-hour marks — pick an off-minute
so ticks do not pile onto the same instant as everyone else's.

The job also expires after 7 days on its own, even in a session that never restarts.
The check is presence, not age: recreate it if `CronList` does not show it, otherwise
leave it alone.

## Tick order

1. Check if .claude/run-state.md exists and is readable. If it does not exist or is unreadable, say so and stop. Do not start a build, and do not recreate the file from memory.
2. **Check the cron; recreate it if missing.** An empty `CronList` after a restart is the
   normal case, not a surprise.
3. **Check the tree**: current branch, `git status --porcelain`, `git worktree list`.
   Builds run on a feature branch in the MAIN repo — worktrees are banned by
   [`git-workflow.md`](../../rules/git-workflow.md). Any entry past the main repo is
   leftover from before that ban: report it, never adopt it as a live build.
4. **Liveness** — if a run is claimed in-flight, judge from the **last records in the run transcript** (or workflow status), **not** file mtime.
5. **Act:**
   - Dead run with a resume id → resume from the **main session** with Workflow `resumeFromRunId` (a workflow cannot resume another workflow).
   - Alive run → do not start a second build.
   - Nothing in flight → pick the next ticket off the queue in run-state.md and start
     **exactly one** `build-ticket`. Do not pre-audit it by hand: phase 0 fetches the live
     Linear text and audits the clauses itself, and blocks before touching status if they
     do not hold.
6. **Run `/run-state`** to write the file. It owns which sections change and how much of
   each is kept.

## Proof (operator)

Kill a run deliberately, let a tick fire, confirm the log line shows resume + run id.

## Do not

- Start multiple builds in one tick.
- Create a worktree, or read an existing one as a live build.
- Trust mtime alone for liveness.
- Launch a build with `Workflow({name: 'build-ticket'})`. A named workflow resolves once
  per session and replays that frozen copy, so edits to the file are ignored for the rest
  of the session. Use `{scriptPath: '.claude/workflows/build-ticket.js'}`.
