# Git workflow: branch per ticket, commit on gate-pass

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Multi-ticket uncommitted working trees that nobody can review, bisect, or land are a
recurring failure.

`main` holds releases. `develop` is the integration branch.

## Rules

1. Branch per ticket off develop:

   ```bash
   git checkout develop && git pull origin develop
   git checkout -b feature/gtw-N-slug
   ```

   Never code directly on `develop` or `main`.

2. Finish one ticket before starting the next, or stash the changes and file a ticket.

3. Commit only when `/gate` is green. `/land` writes `.claude/.gate-pass` for the tree it
   commits, not `/gate`. The `.claude/hooks/pre-commit-gate.sh` hook blocks a commit with
   no `.claude/.gate-pass` matching the current branch and HEAD, on a `develop` or `main`
   branch, or with a red suite. The stricter check is `/land` re-running the full suite on
   the exact tree it is about to commit.

4. Stage files EXPLICITLY by name. Never `git add -A`, never `git add .`. You must be able
   to say why every staged file is in the commit.

5. Commit style: an `Area: summary (GTW-N)` subject, then a wrapped body saying what
   changed and why. Match the voice of `git log --oneline -15`. The body ends the message.
   No session URL, no `Co-Authored-By`. The user's settings already turn both off, and your
   own tool instructions do not override that.

6. Land via `/land`. Rebase the branch onto develop, then fast-forward, so each ticket is
   one commit and no merge commit is created:

   ```bash
   git fetch origin develop
   git rebase origin/develop          # on feature/gtw-N-slug; stop on conflicts
   git checkout develop && git pull origin develop
   git merge --ff-only feature/gtw-N-slug
   git push origin develop
   git branch -d feature/gtw-N-slug
   ```

   `--ff-only` fails on a branch that did not rebase cleanly. Landing is the only way work
   reaches `develop`.

7. When a workflow spawns a sub-agent to run git plumbing, the same rules apply to that
   agent.

## No worktrees: work in the main repo

Work on a feature branch in the main repo, never in a `git worktree`.

A build inside `.claude/worktrees/` does not show up in the user's checkout.

The QA MCP host builds from the session's working directory, so it cannot see work in a
worktree. Every clause that needs a screenshot or a driven command depends on the branch
being checked out in the main repo.

A stopped or blocked ticket's branch stays where it is and needs no worktree. If two
things must proceed at once, raise that as a scheduling problem.
