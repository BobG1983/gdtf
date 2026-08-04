---
name: land
description: >-
  Commit gated work, finish the feature branch into develop, push, and close the
  Linear ticket(s) with evidence. Only after /gate and /docs-sync pass this
  session — refuses to land otherwise.
argument-hint: "[GTW-N ...]"
---

# /land — finish the feature into develop

Binding background: `.claude/rules/git-workflow.md`, `.claude/rules/linear-discipline.md`.

Resolve `GTW-N` from argument or branch. Must match the `TICKET=` set in `.claude/.gate-pass`.

## Preconditions — any failure: refuse, state which, stop

1. **Gate passed for this tree.** `.claude/.gate-pass` exists; `TICKET=` matches; `FINGERPRINT=` equals a fresh recompute of the /gate fingerprint command.

2. **Docs-sync done (or confirmed clean).** `/docs-sync` has been run after the gate (or explicitly confirmed no drift). Do not land with drifted docs.

3. **Suite green now.** Run the full suite from [`.claude/rules/verification.md`](../../rules/verification.md). Any non-zero → refuse.

4. **On a `feature/*` branch** for this ticket. Not `develop` / `main`.

5. **No files outside ticket scope.** List extras from `git status --porcelain` and ask — never auto-stage, stash, or delete. Never stage `.claude/.gate-pass`.

6. **Every contract clause implemented** — no stubs, no "for now".

7. **Ticket scope not shrunk mid-session.** Re-pull each ticket; if scope shrank relative to the /gate contract, surface the diff and ask.

## Steps

1. Stage explicit files by name — never `git add -A`, `-u`, or `.`.
2. Commit: subject `Area: summary (GTW-N)`, body what changed and why. Match `git log --oneline -15` voice. Pre-commit hook re-checks gate-pass; do not use `--no-verify`.
3. `OLD=$(git rev-parse origin/develop)`.
4. `GIT_EDITOR=true git flow feature finish gtw-N-slug`. Stop on conflicts.
5. `git push origin develop`.
6. Move ticket(s) to **Done** via Linear MCP with evidence: merge SHA, suite result, pushed range.
7. Report `git log --oneline OLD..develop`, then delete `.claude/.gate-pass`.
