---
name: pattern-guard-suite-inventory-drift
description: A counted claim in docs/ ("Three guard suites", "three processes") is guarded by nothing — any diff that adds to or removes from a counted list has to fix the count, and the miss is usually in a file the diff never opened.
metadata:
  type: feedback
---

Whenever a diff adds to or removes from a list that `docs/` prose COUNTS, re-read the
lead-in sentence and re-count against the list as it stands.

**Why — a live example right now:** `docs/testing.md:33` opens "Three repo-wide **guard
suites** live in `crates/gdtf_test_utils/tests/`" and there are six: `assets_tree_clean`,
`ci_workflow_features`, `module_layout`, `no_flat_integration_tests`, `qa_commands_doc`,
`rustdoc_lint_gate`. Its third bullet (`:37`) still describes `binary_feature_passthrough`,
which no longer exists. Nothing turned red, because no guard reads a number:
`module_layout` checks `mod.rs` wiring and line bands, `qa_commands_doc` checks that cited
paths exist and named symbols appear (`qa_commands_doc/main.rs:11-33`),
`ci_workflow_features` checks CI command text, `assets_tree_clean` checks the assets tree
is git-clean.

**How to apply:** on any diff touching `docs/`, grep the edited file for number words near
the edited list (`One|Two|Three|Four|Five|both|all three`), then grep `docs/` repo-wide for
the old number AND for the thing being counted (`ToolName::ALL` — five today, at
`bins/gdtf_qa_mcp/src/mcp/tools/name.rs:18-24`; `QaRequest` — three, at
`crates/gdtf_qa_protocol/src/message/request.rs:10-17`). Fixing one instance is not
evidence the class is clear, and a finding raised on an earlier round is not evidence it
was acted on — re-read the file. Related:
[[pattern-fixture-uniqueness-claim-collides]].
