---
paths:
  - "**/*"
---

# Git workflow — branch per ticket, commit on gate-pass

Why this rule exists: giant multi-ticket uncommitted working trees that nobody
can review, bisect, or land are a recurring failure. One ticket = one branch =
one reviewable change.

`CLAUDE.md`'s git-flow section stays authoritative for the branch model
(`main` = releases, `develop` = integration). This rule is the process layer
on top of it.

## Rules

1. Branch per ticket: `git flow feature start gtw-N-slug` →
   `feature/gtw-N-slug` off `develop` (start via `/next-task`). Never code
   directly on `develop` or `main`.
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
6. Land via `/land`: `GIT_EDITOR=true git flow feature finish <name>`, then
   `git push origin develop`. Landing is the only way work reaches `develop`.
7. When a workflow spawns a sub-agent to run git plumbing, the same rules
   apply to it — explicit staging, gate-gated commits, ticket-tagged subjects.

## Optional: two epics at once via shared-target worktrees

To work two INDEPENDENT epics in parallel, use git worktrees that SHARE one cargo
target dir — Bevy compiles once across all worktrees, so a 2nd worktree's diverged
build is ~9x cheaper (verified, GTW-382). The setup is committed:

- `.cargo/config.toml` sets `[unstable] checksum-freshness = true` — hash-based
  freshness so a shared target dir can't false-"fresh" across worktrees and silently
  run stale code (the mtime hazard). Nightly-only; the repo pins nightly.
- `.envrc` exports `CARGO_TARGET_DIR=$HOME/.cache/gdtf-target` (direnv; per-machine,
  invisible to CI). One-time per machine: add `eval "$(direnv hook zsh)"` to your
  shell rc, then `direnv allow` in each checkout/worktree.

Then `git worktree add ../gdtf-<epic> <branch>` per epic. Builds serialize on cargo's
target-dir lock only when two compile at the same instant (rare). One ticket per
worktree still holds (Rule 2, per-worktree).
