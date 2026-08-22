---
name: land
description: >-
  Commit gated work, rebase the feature branch onto develop, fast-forward, push,
  and close the Linear ticket(s) with evidence. Only after /gate and /docs-sync
  pass this session — refuses to land otherwise.
argument-hint: "[GTW-N ...]"
---

# /land — finish the feature into develop

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

Binding background: `.claude/rules/git-workflow.md`, `.claude/rules/linear-discipline.md`.

Resolve `GTW-N` from argument or branch. It is the ticket `/gate` passed, and the `TICKET=` you write into `.claude/.gate-pass` in step 1.

## Preconditions — any failure: refuse, state which, stop

1. **Gate passed for this ticket, this session.** `/gate` reported full PASS on this branch, for this ticket, in this session. No gate report, a different ticket, a different branch, or a report from an earlier session — refuse and re-run `/gate`. The report need not carry a `SCOPE=` line: you derive scope yourself in precondition 3, for the tree you commit.

   You write `.claude/.gate-pass` yourself, in step 1 of **Steps**; `/gate` does not. So there is no file to check here, and no fingerprint comparison: `/docs-sync` runs between `/gate` and `/land` and normally moves the tree, so any such comparison would either fail on a legitimate docs edit or pass against your own fresh recompute. What replaces it is precondition 3 — you re-run the suite on the exact tree you commit.

2. **Docs-sync done (or confirmed clean).** `/docs-sync` has been run after the gate (or explicitly confirmed no drift). Do not land with drifted docs.

3. **Suite green now.** Re-apply the **Suite scope** rules in `/gate` to the tree you are about to commit (same path allowlist; default FULL). This is the check that the tree is still good after `/docs-sync`.
   - If the gate reported `SCOPE=DOCS` and the tree is still docs-only under those rules, cargo is not required.
   - If the gate reported `SCOPE=DOCS` and the tree is now FULL (any non-allowlisted path, or `.claude/rules/verification.md`), refuse and re-run `/gate` — the gate audited a docs-only tree.
   - Otherwise run the full suite from [`.claude/rules/verification.md`](../../rules/verification.md). Any non-zero → refuse.

4. **On a `feature/*` branch** for this ticket. Not `develop` / `main`.

5. **No files outside ticket scope.** List extras from `git status --porcelain` and ask — never auto-stage, stash, or delete. Never stage `.claude/.gate-pass`.

6. **Every contract clause implemented** — no stubs, no "for now".

7. **Ticket scope not shrunk mid-session.** Re-pull each ticket; if scope shrank relative to the /gate contract, surface the diff and ask.

## Steps

1. Write `.claude/.gate-pass` for the tree you just ran the suite on: `TICKET=`, `BRANCH=` (current branch), `HEAD=` (current HEAD), `FINGERPRINT=` from the **one** command in [verification.md → Gate-pass fingerprint](../../rules/verification.md#gate-pass-fingerprint), `SCOPE=` from precondition 3. It records what you are committing and is what the pre-commit hook reads. You are its only writer.
2. Stage explicit files by name — never `git add -A`, `-u`, or `.`.
3. Commit: subject `Area: summary (GTW-N)`, body what changed and why. Match `git log --oneline -15` voice. The body ends the message — no session URL, no `Co-Authored-By`, nothing after it. Pre-commit hook re-checks gate-pass; do not use `--no-verify`.
4. `OLD=$(git rev-parse origin/develop)`.
5. Finish. Rebase then fast-forward, so the ticket lands as one commit with no merge commit:

   ```bash
   git fetch origin develop
   git rebase origin/develop          # still on feature/gtw-N-slug
   git checkout develop
   git pull origin develop
   git merge --ff-only feature/gtw-N-slug
   git push origin develop
   git branch -d feature/gtw-N-slug
   ```

   Stop on conflicts. `--ff-only` fails instead of making a merge commit, so if the rebase did not leave the branch ahead of develop, stop and report rather than working around it.
6. Move ticket(s) to **Done** via Linear MCP with evidence: merge SHA, suite result, pushed range.
7. Report `git log --oneline OLD..develop`, then delete `.claude/.gate-pass`.
