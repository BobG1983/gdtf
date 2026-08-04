---
paths:
  - "**/*"
---

# Git workflow — branch per ticket, commit on gate-pass

Why this rule exists: giant multi-ticket uncommitted working trees that nobody
can review, bisect, or land are a recurring failure. One ticket = one branch =
one reviewable change.

Branch model: **`main`** = releases, **`develop`** = integration. Plain git only
— no `git flow` / git-flow-next dependency.

## Rules

1. Branch per ticket off develop:

   ```bash
   git checkout develop && git pull origin develop
   git checkout -b feature/gtw-N-slug
   ```

   Never code directly on `develop` or `main`.
2. Never build a multi-ticket tree. One ticket's changes per working tree;
   finish (or stash and file a ticket) before starting the next.
3. Commit only on gate-pass: `/gate` must be green first. The
   `.claude/hooks/pre-commit-gate.sh` hook enforces it — no
   `.claude/.gate-pass` matching the current branch/HEAD, a develop/main
   branch, or a red suite all block the commit. `/land`'s content fingerprint
   check is the strict layer.
4. Stage files EXPLICITLY by name. Never `git add -A`, never `git add .` —
   you must be able to say why every staged file is in the commit.
5. Commit style: `Area: summary (GTW-N)` subject plus a wrapped body saying
   what changed and why. Match the voice of `git log --oneline -15`.
6. Land via `/land` with plain git (from the main repo if using a worktree):

   ```bash
   git checkout develop && git pull origin develop
   git merge --no-ff feature/gtw-N-slug
   git push origin develop
   git branch -d feature/gtw-N-slug
   ```

   Landing is the only way work reaches `develop`.
7. When a workflow spawns a sub-agent to run git plumbing, the same rules
   apply to it — explicit staging, gate-gated commits, ticket-tagged subjects.

## No worktrees — one ticket at a time in the main repo

All work happens **on a feature branch in the main repo**. No `git worktree`, and
no second ticket in flight while one is building.

"In the main repo" does not mean "on develop". Rule 1 still holds in full: branch
per ticket, `feature/gtw-N-slug`, off develop. The only thing that changed is
*where* that branch is checked out — the main tree instead of a worktree.

Two reasons, and the second is the one that bites:

- The user can see what is going on. A build inside `.claude/worktrees/` is
  invisible in their checkout.
- **The QA MCP host builds from the session's working directory.** Work in a
  worktree is work that host cannot see, so no live MCP evidence is possible for
  it — the running game is always built from the main tree. Every clause needing
  a screenshot or a driven command depends on the branch being checked out there.

A stopped or blocked ticket's branch stays put; it does not need a worktree to
survive. If two things genuinely must proceed at once, that is a scheduling
problem to raise, not a reason to fan out into worktrees.
