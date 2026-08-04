---
name: pattern-guard-suites-duplicate-manifest-reader
description: Each guard suite under crates/gdtf_test_utils/tests/ carries its own std-only repo-root and git-ls-files reader — the duplication is the house pattern, not copy-paste drift.
metadata:
  type: feedback
---

Do not flag the duplicated tree reader in the conformance guards as a structure violation,
and do not ask for a shared helper.

**Why:** integration test targets are separate crates and cannot share private helpers
without promoting them into a public library. The guards are deliberately std-only — no
`toml` or `serde_yaml` dependency for a guard — so each one re-implements the same small
`repo_root` + `git ls-files` + fs-walk-fallback reader. `.claude/rules/module-layout.md`
rule 6 (:52) forbids inventing a shared abstraction to shrink counts, so the duplication is
the correct call.

Six suites exist today under `crates/gdtf_test_utils/tests/`: `assets_tree_clean`,
`ci_workflow_features`, `module_layout`, `no_flat_integration_tests`, `qa_commands_doc`,
`rustdoc_lint_gate`. Each is dir-form. Their readers:

- `module_layout/tree.rs:8` — `workspace_root()`, not `repo_root()`
- `ci_workflow_features/tree.rs:7`
- `assets_tree_clean/git.rs:6`
- `rustdoc_lint_gate/tree.rs:9`
- `no_flat_integration_tests/tree.rs:5`
- `qa_commands_doc/main.rs:35`

Five have a wiring-only `main.rs` (4-5 lines, zero fns). `qa_commands_doc` is the exception —
its `main.rs` is 110 lines with 7 fns and holds the reader inline, so "wiring-only main.rs"
is a five-of-six claim, not a house rule.

Most suites take a root override so the built guard can be aimed at a scratch checkout —
that is how a gate proves a guard is non-vacuous without mutating the tree. The names differ
per suite (`GDTF_MODULE_LAYOUT_ROOT`, `GDTF_CI_WORKFLOW_ROOT`, `GDTF_ASSETS_CLEAN_ROOT`,
`GDTF_RUSTDOC_GATE_ROOT`, `GDTF_QA_COMMANDS_DOC_ROOT`) and `no_flat_integration_tests` has
none, so read the file rather than assuming a naming scheme.

**How to apply:** check the new suite is auto-discovered — `crates/gdtf_test_utils/Cargo.toml`
declares no `[[test]]` and no `autotests = false`, so `tests/<dir>/main.rs` is picked up
automatically. Then check the inventory: suites get added and removed, and any memory or doc
listing them by name goes stale fast.

Related: [[pattern-guard-suite-inventory-drift]].
