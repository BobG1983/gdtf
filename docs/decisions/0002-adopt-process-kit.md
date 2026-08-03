---
name: "ADR 0002: Adopt the .claude process kit"
description: Adopt the .claude process kit — the next-task→gate→land loop, gate-pass hook, design-gate audit, Linear ticket discipline, and Workflows-not-teams.
---

# 0002. Adopt the `.claude` process kit

## Status

`Accepted` — 2026-06-12.

## Context

GDTF is built largely through Claude Code sessions. The recurring failure modes
of unguided agent development are known and documented in the binding rules
([../../.claude/rules/](../../.claude/rules)): agents silently *narrowing* a
specified design, claiming completion with nothing actually run, building giant
un-landable multi-ticket working trees, and marking tracker tickets "Done"
while the work is un-done or absent.

The project needs a process that makes the **green suite, the design canon, and
the tracker move *with* the work** rather than after it — and that does so
consistently regardless of which session is driving. It also needs an
orchestration model: how do multi-step tasks get decomposed and executed.

## Decision

We will adopt the **`.claude` process kit** as the standing development process
for GDTF:

- **The dev loop is `/next-task` → build → `/gate` → `/land`.** Pick a ticket
  onto its own feature branch, build it, gate it, then land it onto
  `develop` and close the ticket.
- **`/gate` runs the one definition of green plus a `design-gate` audit.** Green
  = the five-command suite of
  [verification.md](../../.claude/rules/verification.md) — `cargo fmt --check`,
  `cargo dclippy -- -D warnings`, `cargo dtest`, `cargo dbuild`, and
  `cargo doc --workspace --no-deps` — all pass; the `design-gate` agent
  additionally audits the diff against the ticket and `docs/`. A change that is
  green but deviates from spec fails the gate.
- **A gate-pass commit hook enforces it.** `.claude/hooks/pre-commit-gate.sh`
  (paths via `$CLAUDE_PROJECT_DIR`) blocks a commit on `develop`/`main`, with a
  red suite, or without a fresh `.gate-pass` matching the current branch and
  HEAD.
- **Linear ticket discipline.** The work queue is Linear (project **GDTF**,
  tickets owned by the team discovered via the Linear MCP). Every
  change has a ticket; statuses move with the work; bugs are filed *before*
  fixing.
- **Workflows, not teams.** Orchestration is Claude Code Workflows /
  on-demand sub-agents. There is no persistent team: a workflow step (the main
  session) spawns a sub-agent per need, the sub-agent does its slice and reports
  back, and the orchestrating workflow relays the result.

## Consequences

- "Done" gains a single, enforced meaning: the green suite was observed green in
  the session after the final edit, and the diff passed the `design-gate` audit
  against the ticket and `docs/`.
- The four standing failure modes are structurally blocked: silent narrowing
  (design-gate + `design-fidelity.md`), unverified completion claims
  (`verification.md` + the gate), multi-ticket trees (branch-per-ticket +
  `git-workflow.md`), and tracker drift (`linear-discipline.md`).
- `docs/` is elevated to a contract *alongside* the Linear ticket — the
  design-gate treats both as binding, which is why the design canon (ADR 0001)
  must stay current.
- Process overhead is accepted as the cost: a one-line fix still takes a ticket,
  a branch, a gate, and a land. This is deliberate — the kit's value is that the
  process is uniform and auditable, not optional.
- Orchestration stays lightweight and stateless: no team to stand up or tear
  down; sub-agents are ephemeral and per-need.

## Alternatives considered

- **No process kit — ad-hoc development.** Rejected. It is exactly the
  unguided mode whose failure modes the kit exists to prevent; nothing would
  enforce the green suite, the design contract, or tracker accuracy.
- **A persistent agent team (TeamCreate-style standing roster).** Rejected in
  favor of Workflows / on-demand sub-agents: a standing team adds coordination
  and lifecycle overhead with no benefit for a single-maintainer project where
  tasks are decomposed per-need and sub-agents report straight back to the
  orchestrating workflow.
- **Tracking work in markdown / a hand-kept TODO instead of Linear.** Rejected;
  a hand-tracked list drifts from reality and cannot serve as evidence. The
  board must move with the work, operated through the Linear MCP.
