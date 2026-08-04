---
name: pattern-guard-placed-in-sibling-not-named-file
description: When a clause names the guard FILE to change, open that file and read its filter before calling a sibling placement a deviation — a module in the same test binary is the same coverage.
metadata:
  type: feedback
---

A clause that names a guard file ("update `<suite>/check.rs` so the new step is guarded")
is about coverage, not file placement. Open the named file and read its walk before ruling.

**Why:** `crates/gdtf_test_utils/tests/ci_workflow_features/check.rs` keeps only the CI
commands that contain both `cargo ` and `--workspace` (`:23`), then matches them against
`REQUIRED_COMMANDS` (`:12-19`). A requirement about a command that must NOT carry
`--workspace` is unreachable from that walk no matter what you add to the list — meeting
the clause literally would mean rewriting the reader. And `main.rs` is three `mod` lines
(`check`, `run_steps`, `tree`), so a new sibling module runs in the same test binary on
the same `cargo dtest`: placement changes nothing about whether the check runs.

**How to apply:** read the filter first. Same test binary, declared in `main.rs`, and shown
failing before the fix = the substance was delivered — report the placement as a note, not
a block. If the named file structurally cannot host the check, say which line stops it.
Related: [[pattern-manifest-dep-defeats-flag-guard]].
