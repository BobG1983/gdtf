---
name: pattern-ci-run-url-unprovable-pre-land
description: An acceptance clause demanding a CI run URL cannot be satisfied from an unpushed feature branch — rule it a violation and cite the missing remote branch and the workflow triggers.
metadata:
  type: feedback
---

An acceptance clause of the form "Evidence: a CI run URL showing X" is unsatisfiable while the
work sits as local edits on a `feature/*` branch. Both workflows trigger only on pushed refs —
`.github/workflows/test.yml:3-6` and `.github/workflows/clippy.yml:2-5` are
`on: push: branches: [develop]` / `pull_request` — so no run object can exist yet.

**Why:** an evidence clause the reviewer did not personally reproduce is not satisfied, and
uncertainty never resolves in the implementer's favour. A local `cargo test` log with non-zero
counts is a substitute, not the named artifact. `gh run list` cannot rescue it either: the
signed-in `gh` account has no access to this repo.

**How to apply:** check `git status` (committed?), `git ls-remote --heads origin` (pushed?),
and the workflow `on:` triggers. If all three say no run can exist, rule the clause a
violation and say the resolution is land-then-attach, or a user-agreed split into a follow-up
ticket. When the ticket's scope says "note the judgment call in the ticket", read the comment
thread too — an empty thread means that instruction is unmet regardless of what the repo docs
say.

Corollary for the structure lens: when the diff touches zero `.rs` files (YAML and markdown
only), there is no code to audit. Say so with the empty
`git diff develop --name-only | grep '\.rs$'` result rather than implying a clean audit of
code that was never changed.

Related: [[pattern-unsatisfiable-acceptance-clause]].
