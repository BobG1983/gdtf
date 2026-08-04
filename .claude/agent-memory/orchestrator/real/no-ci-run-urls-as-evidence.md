---
name: no-ci-run-urls-as-evidence
description: gh on this machine is permanently on the work account and 404s on this personal repo, so a CI run URL can never be acceptance evidence — pin the workflow config with a test instead
metadata:
  type: feedback
---

Never make a CI run URL, or anything read from GitHub Actions, an acceptance clause. It cannot be
satisfied on this machine, ever.

**Why:** `gh` authenticates as the work account over HTTPS, while this repo is `BobG1983/gdtf` on the
user's personal account and is pushed over an SSH alias — `git remote -v` shows
`git@github.com-personal:BobG1983/gdtf.git`, and `github.com-personal` is a host entry in
`~/.ssh/config`. Two independent auth paths: `git push` works, `gh run list` returns HTTP 404. The
user cancelled the ticket that would have fixed this — `gh` stays on the work account — so it is
permanent, not a bug awaiting repair.

The timing fails too. One clause required a CI run URL as evidence and four gate rounds refused it,
correctly: the gate runs BEFORE the commit and the workflows only trigger on push, so no run can
exist yet. Even after landing, nobody here could read it.

**How to apply:**

- Prove CI behaviour by reading the workflow files and pinning them with a test.
  `crates/gdtf_test_utils/tests/ci_workflow_features/` is the pattern. `tree.rs` enumerates every
  git-tracked `.github/workflows/*.yml`; `check.rs` then fails if a `--workspace` cargo command names
  `dynamic_linking`, or still names the removed `net_qa` / `schema` features, or if
  `.github/workflows/test.yml`'s `cargo test --workspace` and `.github/workflows/clippy.yml`'s
  `cargo clippy --workspace --all-targets -- -D warnings` are not reached.
- Nothing else in the repo reads `.github/` — `check.rs` and `tree.rs` are the only Rust files that
  mention the path. Without such a test, a dropped step or a changed flag leaves the whole suite
  green.
- Rewrite any CI-URL clause as a guard test plus local suite output. That is both satisfiable and
  stronger evidence than a run URL.

Related: [[dont-build-tooling-to-prove-the-obvious]].
