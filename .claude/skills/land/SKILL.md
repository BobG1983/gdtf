---
name: land
description: >-
  Commit gated work, rebase the feature branch onto develop, fast-forward, push,
  and close the Linear ticket(s) with evidence. Refuses to land unless /gate and
  /docs-sync passed this session.
argument-hint: "[GTW-N ...]"
---

# /land

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

`.claude/rules/git-workflow.md` and `.claude/rules/linear-discipline.md` bind here too.

Resolve `GTW-N` from the argument or the branch. It is the ticket `/gate` passed, and the `TICKET=` you write into `.claude/.gate-pass` in step 1.

## Preconditions

If one fails, refuse, say which, and stop.

1. `/gate` reported a full PASS on this branch, for this ticket, in this session. If not, refuse and re-run `/gate`. The report does not need a `SCOPE=` line, because you work out the scope yourself in precondition 3.

   `/gate` does not write `.claude/.gate-pass`, so there is no file to check here. Do not compare fingerprints either. `/docs-sync` usually changes the tree between `/gate` and `/land`, so a comparison would fail on a legitimate docs edit or pass against a fingerprint you just recomputed.

2. `/docs-sync` ran after the gate and left no drift, or you have confirmed there is none.

3. The suite is green now. Apply the Suite scope rules in `/gate` to the tree you are about to commit: same path allowlist, FULL by default.
   - The gate reported `SCOPE=DOCS` and the tree is still docs-only under those rules: cargo is not required.
   - The gate reported `SCOPE=DOCS` and the tree is now FULL (any path outside the allowlist, or `.claude/rules/verification.md`): refuse and re-run `/gate`, because the gate audited a docs-only tree.
   - Otherwise run the full suite from [`.claude/rules/verification.md`](../../rules/verification.md). Any non-zero exit means refuse.

4. You are on a `feature/*` branch for this ticket, not `develop` or `main`.

5. The tree holds no files outside ticket scope. List anything extra from `git status --porcelain` and ask. Never auto-stage, stash or delete it. Never stage `.claude/.gate-pass`.

6. Every contract clause is implemented. No stubs, no "for now".

7. Ticket scope has not shrunk mid-session. Re-pull each ticket, and if the scope is smaller than the `/gate` contract, show what changed and ask.

## Steps

1. Write `.claude/.gate-pass` for the tree you just ran the suite on. It holds `TICKET=`, `BRANCH=` (current branch), `HEAD=` (current HEAD), `FINGERPRINT=` from the one command in [verification.md → Gate-pass fingerprint](../../rules/verification.md#gate-pass-fingerprint), and `SCOPE=` from precondition 3.
2. Stage files by name. Never `git add -A`, `-u`, or `.`.
3. Commit with the subject `Area: summary (GTW-N)` and a body saying what changed and why. Match the voice of `git log --oneline -15`. The body ends the message: no session URL, no `Co-Authored-By`. The pre-commit hook re-checks the gate-pass. Do not use `--no-verify`.
4. `OLD=$(git rev-parse origin/develop)`.
5. Rebase, then fast-forward, so the ticket lands as one commit:

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
6. Move the ticket(s) to **Done** through the Linear MCP, with evidence: merge SHA, suite result, pushed range.
7. Report `git log --oneline OLD..develop`, then delete `.claude/.gate-pass`.
