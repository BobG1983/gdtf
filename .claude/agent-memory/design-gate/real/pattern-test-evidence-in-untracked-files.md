---
name: pattern-test-evidence-in-untracked-files
description: Tests in untracked files pass locally but are invisible to the repo guards and would not land — run git ls-files -o before crediting any test coverage.
metadata:
  type: feedback
---

Before crediting a clause's test coverage, check the test files are TRACKED.

**Why it matters twice:** (1) staging is explicit by name (`git-workflow.md` rule 4), so a
commit that does not name an untracked file lands a tree that will not build — the `mod`
declaration is tracked and its file is not; (2) the module-layout guard walks
`git ls-files -- crates bins` (`crates/gdtf_test_utils/tests/module_layout/tree.rs:19`), so an
untracked file's line count and its mod.rs purity are never measured. `cargo dtest` passing
proves nothing about what will land.

Three files hit this at once during a review: `bins/gdtf_qa_mcp/src/mcp/control/test/logs.rs`,
`bins/gdtf_qa_mcp/tests/lifecycle/child_output.rs` and
`crates/gdtf_app/src/dev/net_qa/wire/test/scalars.rs` were all `mod`-declared and untracked, two
of them holding the only tests for their clause. All three are tracked today — the check is what
got them there.

**How to apply:** run `git ls-files -o --exclude-standard -- '*.rs'` on EVERY review, before
judging test coverage. If a clause's evidence sits in an untracked file, say so in the verdict
and `wc -l` those files by hand.
