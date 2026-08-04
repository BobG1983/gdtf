---
name: build-it-in-a-workflow
description: Every ticket runs as a resumable workflow with the gate folded in as a bounded gate-fix-regate loop; audit the clauses before opening the ticket, engineers never commit, all agents opus.
metadata:
  type: feedback
---

Build a ticket as a **resumable workflow**, not a hand-driven sequence of one-off agents.
`.claude/workflows/build-ticket.js` is the shape: clause-audit, open, build, verify, three-lens
gate, bounded fix loop, docs-sync, land, confirm, close.

**Why:** a run that dies must be resumable from its run id rather than restarted by hand, and a
gate whose failures bubble up to the orchestrator turns the orchestrator into a message bus —
every round costs a recovery tick. Folding the repair loop into the workflow keeps the run
self-correcting and leaves only genuine design ambiguity to escalate.

**How to apply:**

- **Audit the clauses before the ticket goes In Progress.** Phase 0 fetches the live description
  and full comment thread and reads the acceptance clauses as a **set**: does the permitted route
  produce this evidence at gate time, do any two clauses contradict, does each assert something
  checkable. `AUDIT_BLOCK` returns without touching the board. Audit the fetched text, never a
  summary someone wrote about it. The in-flight tell for a contradictory pair is a gate failing
  the same clause every round while the suite stays green.
- **Mark the ticket In Progress before any code is touched**, and In Review at the gate. A
  workflow that closes a ticket must also have opened it — Linear cannot backdate a transition, so
  a late correction records when you fixed the board, not when the work happened.
- **Fold the gate in as a bounded loop.** `while (!allPass() && attempt < 5)` — verify, three
  parallel read-only lenses (fidelity / tests / structure), any non-compliant blocks, then one fix
  agent gets every blocking verdict quoted whole. Lenses run zero cargo and never spawn further
  agents.
- **Engineers never commit.** Build, fix and docs-sync agents are all told not to commit; the land
  agent owns staging, the commit and the merge, and re-runs the suite if develop moved. Landing is
  proven by a separate confirm agent reading `origin/develop`, not by the land agent's claim.
- **One worktree per ticket, plain git.** `git worktree add -b feature/<slug>
  .claude/worktrees/<slug> develop`, adopt it if it already exists, never reset finished unlanded
  work. No `git flow` — `.claude/rules/git-workflow.md` is the authority.
- **One cargo build at a time.** A stale dylib against fresh rlibs fails at land after a green
  verify. Gate lenses run no cargo, so they still fan out; a docs-only run can sit beside a build.
- **All agents opus.** Judge by turns-to-correct, not per-token price — sonnet needed more
  correction rounds and cost more overall. Never pin `fable`: access is exhausted and the call
  dies outright. Sonnet is acceptable only for fully specified mechanical moves.
- **In an unattended run, do not block on a question you can answer.** Log the assumption and keep
  going; edit a running plan rather than restarting it. Never drop scope silently and never claim
  evidence you did not produce. When the user changes a rule mid-run, write it down in the same
  turn — a change that lives only in conversation dies at the next compaction.

Related: [[a-dead-agent-is-not-a-pass]], [[resume-dont-restart-a-dead-run]],
[[forward-evidence-never-assert-a-pass]].
