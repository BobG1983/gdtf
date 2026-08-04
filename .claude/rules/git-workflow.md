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

## Optional: two epics at once via local-target worktrees

To work two INDEPENDENT epics in parallel, use git worktrees — one per epic:
`git worktree add ../gdtf-<epic> <branch>`. Each worktree builds into its OWN
local `target/`, which dies with `git worktree remove` — the cold-build cost of
the worktree's first build is accepted. Only the MAIN tree keeps a persistent
`./target`, cleaned when it exceeds 100G, checked at land windows. `.cargo/config.toml`'s
`[unstable] checksum-freshness = true` STAYS. One ticket per worktree still holds
(Rule 2, per-worktree).
