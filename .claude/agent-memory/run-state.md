# Autonomous run-state

Live values are machine-local. No secrets here.

## Crons (local machine timezone)

| Name | Schedule | Skill | Cron id |
|------|----------|-------|---------|
| heartbeat | every 2 hours at :17 | `/heartbeat` | `444e6702` (session-scoped; re-register on the next tick) |
| dream | daily at local midnight | `/dream` | _(not registered)_ |

Timezone: **local machine**, not UTC unless the host is set to UTC.

Cron jobs created through the harness are session-only and auto-expire after 7 days. A tick that
finds no heartbeat entry re-creates it and rewrites the id above.

## Standing goal

Complete GTW-17 (Battlescape, Mythos) autonomously through `.claude/workflows/build-ticket.js`,
one ticket per run, following `.claude/rules/`. User requests take priority over the queue.
Progress must survive a session death: every run records its worktree, branch and workflow run
id below before work starts.

### GTW-17 queue (board fetch 2026-08-04)

111 children, 16 open. Startable now, in board order:

1. GTW-884 Reload button is cut off — High
2. GTW-885 Melee button draws under next/prev — High
3. GTW-886 Throw button shows when there is nothing to throw — High
4. GTW-887 Throw button disappears before it can be clicked — High
5. GTW-888 Floating combat text can appear before the shot plays — High
6. GTW-561 AI: most acts get an AI path — Medium (split per act first; its body says so)
7. GTW-649 Headless tuning harness — Medium (self-gated on procgen level creation)
8. GTW-937 Bug: destroyed cover tile swaps at sim time — Medium
9. GTW-647 Presenter: isometric renderer mode — Low (reads Epic-sized)

Not startable: GTW-938 / GTW-799 / GTW-400 / GTW-890 / GTW-867 (Epics — build their children),
GTW-869 (blocked by GTW-871), GTW-536 (archived, AWAITING-ART).

MCP work under GTW-17 sits beneath the GTW-938 epic: GTW-971, GTW-998, GTW-997.

## Tick log (heartbeat keeps last 5)

```
# YYYY-MM-DDTHH:MM:SS  action  detail
2026-08-04T01:05:00  tick   nothing in flight; registered heartbeat cron 444e6702; user directive took the slot
2026-08-04T01:40:00  dream  126 design-gate memories verified: 52 durable, 55 stale, 19 superseded
2026-08-04T02:10:00  dream  apply run wf_89e5d9a3-c98 — triage stale, promote durable, merge pairs, promote global memory
```

## In-flight

- worktree: `/Users/bgardner/dev/gdtf` (main tree — no ticket worktree open)
- branch: `develop`
- workflow run id: `wf_89e5d9a3-c98` (`/dream` apply)
- ticket: comment-hygiene sweep + red-suite fix (being filed); GTW-17 build not started

### Recovery notes

- `/dream` verification results are cached at `<session scratchpad>/dream-results.json` with the
  apply script beside it. Resume with
  `Workflow({scriptPath: .../dream-apply.js, resumeFromRunId: 'wf_89e5d9a3-c98'})`.
- A workflow cannot resume another workflow. Resume from the main session.
- Uncommitted work in the main tree: ticket-id purge across 21 Rust files and 8 docs files, plus
  five clippy fixes that were red on develop. Needs its own branch before any commit — the
  pre-commit hook blocks commits on `develop`.
