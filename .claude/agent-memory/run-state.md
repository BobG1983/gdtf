# Autonomous run-state

Live values are machine-local. No secrets here.

## Crons (local machine timezone)

| Name | Schedule | Skill | Cron id |
|------|----------|-------|---------|
| heartbeat | every 2 hours at :17 | `/heartbeat` | `444e6702` (session-scoped; re-register on the next tick) |
| dream | daily at local midnight | `/dream` | _(not registered)_ |

Timezone: **local machine**, not UTC unless the host is set to UTC. Tick log times are local.

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
2026-08-04T08:41:00  build  GTW-884 launched — stopped, needs MCP screenshots; all 10 open bugs now gated on GTW-1014
2026-08-04T09:20:00  land   GTW-1028 background-work rule merged as f46b47e6 and pushed
2026-08-04T09:35:00  clean  removed idle worktrees gtw-884 and gtw-971; both branches deleted
2026-08-04T09:40:00  red    develop red at f46b47e6 — 7 clippy sites, fallout from 5c0b601a not the bevy-systems merge
2026-08-04T09:29:11  build  GTW-1020 launched as wf_7d6b0238-0f9 — red fix first, then the expect sweep
```

## In-flight

- worktree: `.claude/worktrees/gtw-1020-expect-sweep` (created by the build agent)
- branch: `feature/gtw-1020-expect-sweep`
- workflow run id: `wf_7d6b0238-0f9` (build-ticket, GTW-1020)
- ticket: GTW-1020 — expect sweep, carrying the red-develop fix

Resume a dead run from the MAIN session:
`Workflow({name:'build-ticket', resumeFromRunId:'<id>', args:{...}})`. A workflow
cannot resume another workflow.

### Develop is red — nothing else can gate until GTW-1020 lands

Seven sites fail `cargo dclippy -- -D warnings`, all fallout from `5c0b601a`
(deny `#[allow]`, prefer `#[expect]`, drop dead cast expects — 53 files, no ticket).
One unfulfilled `type_complexity` expectation in `gdtf_ui`, six `let_and_return`
across sim, input and editor. A second wave of unfulfilled `missing_docs`
expectations in `gdtf_app` is expected once the first clears — `gdtf_app` is never
reached today because earlier crates fail to compile.

Rule for the sweep: an unfulfilled expectation means the lint stopped firing, so
the attribute goes. Never change code to re-trigger a lint.

### Everything else is gated

All 10 open bugs are blocked by GTW-1014 (`battle.start` + `capture.screenshot`) —
a bug fix cannot be confirmed against its reported symptom until the QA MCP can
drive the game and photograph a running battle. GTW-888 is the weakest fit for
that rule (an ordering bug a seeded headless test pins better than any still
frame) and is flagged on its ticket for overrule.

GTW-1008 is unblocked — both blockers ruled: `settings.read` widens to
`pub(crate)`, `Keybinds` gets a fallback so it always exists. It is next after
GTW-1020, and the path that unblocks the bugs is GTW-1008 → GTW-1014.

### MCP leaf order under GTW-938 (audited 2026-08-04)

1. **GTW-1008** game shell reads — cleanest clauses, unblocks 8 downstream. Correction the
   builder needs: `app.phase` is ALREADY built (`crates/gdtf_app/src/dev/net_qa/commands/set.rs:7`
   is `GAME_COMMANDS = &[&AppPhase]`). The real work is the other six reads.
2. **GTW-1000** editor host + `editor.phase` — blocked on a decision: the editor still carries an
   old QA module (`crates/gdtf_content_editor/src/net_qa/` has config/env/router/screenshot/present
   but no `wire/`, no command set) even though "delete the old surface" is marked Done. The ticket
   never says whether this replaces, wraps, or lands beside it. Ask before building.
3. **GTW-999** sim owns every TU cost — blocked on a decision: its stated first step is GTW-984,
   which is **Canceled**, so the move/stance/turn/reload predicates are unowned. Its `battle.cost`
   clause also names a command that GTW-1015 creates, and GTW-1015 is blocked BY GTW-999. Ask.
4. **GTW-971** two coverage gaps — clean, small, no clause defects. Good filler.

Evidence rule for all of these: the in-repo socket test, never the resident `mcp__gdtf-qa__*`
tools. A ticket that changes how a host starts cannot be proven by driving a host built from
develop.

### Recovery notes

- `/dream` verification results are cached at `<session scratchpad>/dream-results.json` with the
  apply script beside it. Resume with
  `Workflow({scriptPath: .../dream-apply.js, resumeFromRunId: 'wf_89e5d9a3-c98'})`.
- A workflow cannot resume another workflow. Resume from the main session.
- Uncommitted work in the main tree: ticket-id purge across 21 Rust files and 8 docs files, plus
  five clippy fixes that were red on develop. Needs its own branch before any commit — the
  pre-commit hook blocks commits on `develop`.
