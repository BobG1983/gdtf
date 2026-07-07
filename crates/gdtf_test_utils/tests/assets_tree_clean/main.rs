//! GTW-653 assets-tree cleanliness guard — the tracked `assets/` tree must be
//! CLEAN when this suite runs, so a test that mutates shipped authored content
//! (the 2026-07-06 incident: `assets/content/gangs/gang_0.gang.ron` sat
//! modified after a day's suite runs — a comment-stripping re-serialization
//! through a real save path) is caught LOUDLY instead of silently corrupting
//! authored work and dirtying the land train.
//!
//! Two git reads, both of which must come back empty:
//!
//! - `git status --porcelain -- assets/` — worktree + index changes and
//!   untracked files under `assets/`;
//! - `git diff HEAD --stat -- assets/` — the stronger HEAD-relative diff,
//!   catching staged-over dirt from a prior run.
//!
//! HONEST SEMANTICS: cargo schedules tests in parallel, so this guard catches
//! PERSISTENT mutations — the observed incident class, where the dirt survived
//! the run — not necessarily a same-run transient write-then-revert. Per-test
//! isolation (every write-path test rooted in a `TempDir`, the GTW-555/636
//! pattern) is the GTW-653 C1 audit's job, not this guard's.
//!
//! A developer's own uncommitted `assets/` edits WILL trip this guard during
//! local dev — that is BY DESIGN for the gate/CI context (the guard is about
//! TESTS mutating authored content, not about authors authoring). The escape:
//! commit or stash your authored-content edits before running the suite.
//!
//! Reads git state only — no cargo invocations, no writes. Outside a git
//! checkout (no `git` binary / not a work tree) there is no baseline to
//! compare against, so the guard notes the skip and passes; the gate and CI
//! always run inside a checkout. The repo root defaults to
//! `CARGO_MANIFEST_DIR/../..` and can be overridden via the
//! `GDTF_ASSETS_CLEAN_ROOT` env var (the sibling guards' recipe, for pointing
//! the built guard at another checkout).

mod check;
mod git;
